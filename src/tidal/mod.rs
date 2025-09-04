mod model;
mod response;

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

use async_trait::async_trait;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use model::{TidalMediaResponse, TidalMediaResponseSingle, TidalOAuthDeviceRes};
use reqwest::Response;
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use serde_json::json;
use tracing::{info, warn};

use self::model::{TidalPageResponse, TidalPlaylistResponse, TidalSongItemResponse};
use crate::ConfigArgs;
use crate::music_api::{
    MusicApi, MusicApiType, OAuthRefreshToken, OAuthReqToken, OAuthToken, PLAYLIST_DESC, Playlist,
    Playlists, Song, Songs,
};
use crate::tidal::model::{TidalPlaylistCreateResponse, TidalSearchResponse};

pub struct TidalApi {
    client: reqwest::Client,
    config: ConfigArgs,
    user_id: String,
    country_code: String,
}

#[derive(Debug)]
enum HttpMethod<'a> {
    Get(&'a serde_json::Value),
    Post(&'a serde_json::Value),
    Put(&'a serde_json::Value),
}

impl TidalApi {
    const API_URL: &'static str = "https://api.tidal.com";
    const API_V2_URL: &'static str = "https://openapi.tidal.com/v2";

    const AUTH_URL: &'static str = "https://auth.tidal.com/v1/oauth2/device_authorization";
    const TOKEN_URL: &'static str = "https://auth.tidal.com/v1/oauth2/token";
    const SCOPE: &'static str = "r_usr w_usr w_sub";

    pub async fn new(
        client_id: &str,
        client_secret: &str,
        oauth_token_path: PathBuf,
        clear_cache: bool,
        config: ConfigArgs,
    ) -> Result<Self> {
        let token = if !oauth_token_path.exists() || clear_cache {
            info!("requesting new token");
            Self::request_token(client_id, client_secret, config.debug).await?
        } else {
            info!("refreshing token");
            Self::refresh_token(client_id, client_secret, &oauth_token_path, config.debug).await?
        };
        // Write new token
        let mut file = std::fs::File::create(&oauth_token_path)?;
        serde_json::to_writer(&mut file, &token)?;

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", token.access_token).parse()?,
        );
        headers.insert("Content-Type", "application/vnd.tidal.v1+json".parse()?);

        let mut client = reqwest::Client::builder()
            .cookie_store(true)
            .default_headers(headers);
        if let Some(proxy) = &config.proxy {
            client = client
                .proxy(reqwest::Proxy::all(proxy)?)
                .danger_accept_invalid_certs(true)
        }
        let client = client.build()?;

        let url = format!("{}/users/me", Self::API_V2_URL);
        let me_res: TidalMediaResponseSingle = Self::make_request_json_internal(
            &client,
            &url,
            &HttpMethod::Get(&json!({})),
            None,
            config.debug,
        )
        .await?;
        let country_code = me_res.data.attributes.country.unwrap_or("US".into());

        Ok(Self {
            client,
            config,
            user_id: me_res.data.id,
            country_code,
        })
    }

    async fn request_token(
        client_id: &str,
        client_secret: &str,
        debug: bool,
    ) -> Result<OAuthToken> {
        let client = reqwest::Client::new();
        let params = json!({
            "client_id": client_id,
            "scope": Self::SCOPE,
        });

        let device_res: TidalOAuthDeviceRes = Self::make_request_json_internal(
            &client,
            Self::AUTH_URL,
            &HttpMethod::Post(&params),
            None,
            debug,
        )
        .await?;

        let url = format!("https://{}", device_res.verification_uri_complete);

        if webbrowser::open(&url).is_err() {
            info!("Please authorize the app by visiting the following URL: {}", url);
        } else {
            info!("Please authorize the app in your browser and press enter");
        }

        std::io::stdin().read_exact(&mut [0])?;

        let auth_token = OAuthReqToken {
            client_id: client_id.to_string(),
            device_code: device_res.device_code.clone(),
            grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
            scope: Self::SCOPE.to_string(),
        };
        let res = client
            .post(Self::TOKEN_URL)
            .basic_auth(client_id, Some(client_secret))
            .form(&auth_token)
            .send()
            .await?;
        // TODO: move to Self::make_request_json_internal, the basic_auth is the problem
        let token: OAuthToken = res.json().await?;

        Ok(token)
    }

    async fn refresh_token(
        client_id: &str,
        client_secret: &str,
        oauth_token_path: &PathBuf,
        debug: bool,
    ) -> Result<OAuthToken> {
        let client = reqwest::Client::new();
        let reader = std::fs::File::open(oauth_token_path)?;
        let mut oauth_token: OAuthToken = serde_json::from_reader(reader)?;

        let params = json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "grant_type": "refresh_token",
            "refresh_token": &oauth_token.refresh_token,
        });
        let refresh_token: OAuthRefreshToken = Self::make_request_json_internal(
            &client,
            Self::TOKEN_URL,
            &HttpMethod::Post(&params),
            None,
            debug,
        )
        .await?;

        oauth_token.access_token = refresh_token.access_token;
        oauth_token.expires_in = refresh_token.expires_in;
        oauth_token.scope = refresh_token.scope;
        Ok(oauth_token)
    }

    async fn paginated_request<T>(
        &self,
        url: &str,
        method: &HttpMethod<'_>,
        limit: usize,
    ) -> Result<TidalPageResponse<T>>
    where
        T: DeserializeOwned + std::fmt::Debug,
    {
        let mut res: TidalPageResponse<T> = self.make_request_json(url, method, limit, 0).await?;
        if res.items.is_empty() {
            return Ok(res);
        }

        let mut offset = limit;
        while offset < res.total_number_of_items {
            let res2: TidalPageResponse<T> =
                self.make_request_json(url, method, limit, offset).await?;
            if res2.items.is_empty() {
                break;
            }
            res.items.extend(res2.items);
            offset += limit;
        }
        Ok(res)
    }

    async fn make_request(
        &self,
        url: &str,
        method: &HttpMethod<'_>,
        lim_off: Option<(usize, usize)>,
    ) -> Result<Response> {
        Self::make_request_internal(&self.client, url, method, lim_off).await
    }

    async fn make_request_json<T>(
        &self,
        url: &str,
        method: &HttpMethod<'_>,
        limit: usize,
        offset: usize,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        Self::make_request_json_internal(
            &self.client,
            url,
            method,
            Some((limit, offset)),
            self.config.debug,
        )
        .await
    }

    async fn make_request_internal(
        client: &reqwest::Client,
        url: &str,
        method: &HttpMethod<'_>,
        lim_off: Option<(usize, usize)>,
    ) -> Result<Response> {
        let mut request = match method {
            HttpMethod::Get(p) => client.get(url).query(p),
            HttpMethod::Post(b) => client.post(url).form(b),
            HttpMethod::Put(b) => client.put(url).form(b),
        };
        if let Some((limit, offset)) = lim_off {
            request = request.query(&[("limit", limit), ("offset", offset)]);
        }
        let res = request.send().await?;
        let res = res.error_for_status()?;
        Ok(res)
    }

    async fn make_request_json_internal<T>(
        client: &reqwest::Client,
        url: &str,
        method: &HttpMethod<'_>,
        lim_off: Option<(usize, usize)>,
        debug: bool,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let res = Self::make_request_internal(client, url, method, lim_off).await?;
        let obj = if debug {
            let text = res.text().await?;
            std::fs::write("debug/tidal_last_res.json", &text)?;
            serde_json::from_str(&text)?
        } else {
            res.json().await?
        };
        Ok(obj)
    }
}

#[async_trait]
impl MusicApi for TidalApi {
    fn api_type(&self) -> MusicApiType {
        MusicApiType::Tidal
    }

    fn country_code(&self) -> &str {
        &self.country_code
    }

    async fn create_playlist(&mut self, name: &str, public: bool) -> Result<Playlist> {
        let url = format!(
            "{}/v2/my-collection/playlists/folders/create-playlist",
            Self::API_URL
        );
        let params = json!({
            "name": name,
            "description": PLAYLIST_DESC,
            "public": public,
            "folderId": "root"
        });
        let res: TidalPlaylistCreateResponse = self
            .make_request_json(&url, &HttpMethod::Put(&params), 0, 5)
            .await?;

        Ok(Playlist {
            id: res.data.uuid,
            name: name.to_string(),
            songs: vec![],
            owner: Some("".to_string()) // TODO: get the owner
        })
    }

    async fn get_playlists_info(&mut self) -> Result<Vec<Playlist>> {
        let url = format!("{}/v1/users/{}/playlists", Self::API_URL, self.user_id);
        let params = json!({
            "countryCode": self.country_code,
        });
        let res: TidalPageResponse<TidalPlaylistResponse> = self
            .paginated_request(&url, &HttpMethod::Get(&params), 100)
            .await?;
        let playlists: Playlists = res.try_into()?;
        
        // Deduplicate playlists by ID to handle Tidal API returning duplicates
        let mut seen_ids = HashMap::new();
        let mut deduplicated = Vec::new();
        let original_count = playlists.0.len();
        
        for playlist in &playlists.0 {
            if let Some(_existing) = seen_ids.get(&playlist.id) {
                warn!("Duplicate playlist found: \"{}\" (ID: {}), keeping first occurrence", 
                      playlist.name, playlist.id);
                continue;
            }
            seen_ids.insert(playlist.id.clone(), true);
            deduplicated.push(playlist.clone());
        }
        
        info!("Fetched {} playlists, {} after deduplication", 
              original_count, deduplicated.len());
              
        Ok(deduplicated)
    }

    async fn get_playlist_songs(&mut self, id: &str) -> Result<Vec<Song>> {
        let url = format!("{}/v1/playlists/{}/items", Self::API_URL, id);
        let params = json!({
            "countryCode": self.country_code,
        });
        // NOTE: a limit > 100 triggers a 400 error
        let res: TidalPageResponse<TidalSongItemResponse> = self
            .paginated_request(&url, &HttpMethod::Get(&params), 100)
            .await?;
        let songs: Songs = res.try_into()?;
        Ok(songs.0)
    }

    async fn add_songs_to_playlist(&mut self, playlist: &mut Playlist, songs: &[Song]) -> Result<()> {
        if songs.is_empty() {
            return Ok(());
        }

        let url = format!("{}/v1/playlists/{}", Self::API_URL, playlist.id);
        let params = json!({
            "countryCode": self.country_code,
        });
        let res = self
            .make_request(&url, &HttpMethod::Get(&params), None)
            .await?;
        let etag = res
            .headers()
            .get("ETag")
            .ok_or(eyre!("No ETag in Tidal Response"))?;

        // TODO: accomodate make_request to access request headers + body

        let url = format!("{}/v1/playlists/{}/items", Self::API_URL, playlist.id);
        let params = json!({
            "trackIds": songs.iter().map(|s| s.id.as_str()).collect::<Vec<_>>().join(","),
            "onDuplicate": "FAIL",
            "onArtifactNotFound": "FAIL",
        });
        let res = self
            .client
            .post(url)
            .header("If-None-Match", etag)
            .form(&params)
            .send()
            .await?;
        res.error_for_status()?;

        Ok(())
    }

    async fn remove_songs_from_playlist(
        &mut self,
        _playlist: &mut Playlist,
        _songs_ids: &[Song],
    ) -> Result<()> {
        todo!()
    }

    async fn delete_playlist(&mut self, playlist: Playlist) -> Result<()> {
        let url = format!(
            "{}/v2/my-collection/playlists/folders/remove",
            Self::API_URL
        );
        let params = json!({
            "trns": format!("trn:playlist:{}", playlist.id),
        });
        let _res = self
            .make_request(&url, &HttpMethod::Put(&params), None)
            .await?;
        Ok(())
    }

    async fn search_song(&mut self, song: &Song) -> Result<Option<Song>> {
        if let Some(isrc) = &song.isrc {
            let url = format!("{}/tracks", Self::API_V2_URL);
            let params = json!({
                "countryCode": self.country_code,
                "include": "albums,artists",
                "filter[isrc]": isrc.to_uppercase(),
            });
            let res: TidalMediaResponse = self
                .make_request_json(&url, &HttpMethod::Get(&params), 0, 1)
                .await?;
            if res.data.is_empty() {
                return Ok(None);
            }
            let mut res_songs: Songs = res.try_into()?;
            if res_songs.0.is_empty() {
                return Ok(None);
            }
            return Ok(Some(res_songs.0.remove(0)));
        }

        let url = format!("{}/v1/search", Self::API_URL);
        let mut queries = song.build_queries();

        while let Some(query) = queries.pop() {
            let params = json!({
                "countryCode": self.country_code,
                "query": query,
                "type": "TRACKS",
            });
            let res: TidalSearchResponse = self
                .make_request_json(&url, &HttpMethod::Get(&params), 0, 3)
                .await?;
            let res_songs: Songs = res.try_into()?;
            // iterate over top 3 results
            for res_song in res_songs.0.into_iter().take(3) {
                if song.compare(&res_song) {
                    return Ok(Some(res_song));
                }
            }
        }
        Ok(None)
    }

    async fn add_likes(&mut self, songs: &[Song]) -> Result<()> {
        if songs.is_empty() {
            return Ok(());
        }

        let url = format!(
            "{}/v1/users/{}/favorites/tracks",
            Self::API_URL,
            self.user_id
        );
        let tracks = songs.iter().map(|s| s.id.as_str()).collect::<Vec<_>>();

        // NOTE: we get error 500 if we like too much songs at once
        for tracks_chunk in tracks.chunks(100) {
            let params = json!({
                "countryCode": self.country_code,
                "trackIds": tracks_chunk.join(","),
                "onArtifactNotFound": "FAIL",
            });
            self.make_request(&url, &HttpMethod::Post(&params), None)
                .await?;
        }
        Ok(())
    }

    async fn get_likes(&mut self) -> Result<Vec<Song>> {
        let url = format!(
            "{}/v1/users/{}/favorites/tracks",
            Self::API_URL,
            self.user_id
        );
        let params = json!({
            "countryCode": self.country_code,
        });
        let res: TidalPageResponse<TidalSongItemResponse> = self
            .paginated_request(&url, &HttpMethod::Get(&params), 1000)
            .await?;
        let songs: Songs = res.try_into()?;
        Ok(songs.0)
    }
}
