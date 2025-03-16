pub mod model;
mod response;

use std::collections::HashMap;
use std::fmt::Write;
use std::io::Read;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use async_trait::async_trait;
use color_eyre::eyre::{eyre, Result};
use model::{YtMusicAddLikeResponse, YtMusicOAuthDeviceRes};
use reqwest::header::{HeaderMap, HeaderName};
use serde::de::DeserializeOwned;
use serde_json::json;
use tokio::time::Instant;
use tracing::{debug, info};

use self::model::{YtMusicContinuationResponse, YtMusicPlaylistEditResponse, YtMusicResponse};
use crate::music_api::{
    MusicApi, MusicApiType, OAuthRefreshToken, OAuthToken, Playlist, Playlists, Song, Songs,
    PLAYLIST_DESC,
};
use crate::yt_music::model::{YtMusicPlaylistCreateResponse, YtMusicPlaylistDeleteResponse};
use crate::yt_music::response::{SearchSongUnique, SearchSongs};
use crate::ConfigArgs;

static CONTEXT: LazyLock<serde_json::Value> = LazyLock::new(|| {
    json!({
        "client": {
            "clientName": "WEB_REMIX",
            "clientVersion": "1.20241205.01.00",
            "hl": "en"
        },
        "user": {}
    })
});

pub struct YtMusicApi {
    client: reqwest::Client,
    client_id: String,
    client_secret: String,
    oauth_token_path: PathBuf,
    last_token_refresh: Instant,
    config: ConfigArgs,
}

impl YtMusicApi {
    const BASE_API: &'static str = "https://music.youtube.com/youtubei/v1/";
    const BASE_PARAMS: &'static str = "?alt=json&key=AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX30";

    const OAUTH_SCOPE: &'static str = "https://www.googleapis.com/auth/youtube";
    const OAUTH_CODE_URL: &'static str = "https://www.youtube.com/o/oauth2/device/code";
    const OAUTH_TOKEN_URL: &'static str = "https://oauth2.googleapis.com/token";
    const OAUTH_GRANT_TYPE: &'static str = "http://oauth.net/grant_type/device/1.0";
    const OAUTH_USER_AGENT: &'static str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:88.0) Gecko/20100101 Firefox/88.0 Cobalt/Version";

    pub async fn new_oauth(
        client_id: &str,
        client_secret: &str,
        oauth_token_path: PathBuf,
        clear_cache: bool,
        config: ConfigArgs,
    ) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", Self::OAUTH_USER_AGENT.parse()?);
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        let token = if !oauth_token_path.exists() || clear_cache {
            Self::request_token(&client, client_id, client_secret).await?
        } else {
            info!("refreshing token");
            Self::refresh_token(&client, client_id, client_secret, &oauth_token_path).await?
        };
        // Write new token
        let mut file = std::fs::File::create(&oauth_token_path)?;
        serde_json::to_writer(&mut file, &token)?;

        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", Self::OAUTH_USER_AGENT.parse()?);
        headers.insert("Cookie", "SOCS=CAI".parse()?);
        headers.insert(
            "Authorization",
            format!("Bearer {}", token.access_token).parse()?,
        );

        let mut client = reqwest::Client::builder()
            .cookie_store(true)
            .default_headers(headers);
        if let Some(proxy) = &config.proxy {
            client = client
                .proxy(reqwest::Proxy::all(proxy)?)
                .danger_accept_invalid_certs(true)
        }
        let client = client.build()?;

        Ok(YtMusicApi { client, client_id: client_id.to_string(), client_secret: client_secret.to_string(), oauth_token_path, config, last_token_refresh: Instant::now() })
    }

    async fn refresh_token(
        client: &reqwest::Client,
        client_id: &str,
        client_secret: &str,
        oauth_token_path: &PathBuf,
    ) -> Result<OAuthToken> {
        let reader = std::fs::File::open(oauth_token_path)?;
        let mut oauth_token: OAuthToken = serde_json::from_reader(reader)?;

        let params = json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "grant_type": "refresh_token",
            "refresh_token": &oauth_token.refresh_token,
        });
        let res = client
            .post(Self::OAUTH_TOKEN_URL)
            .form(&params)
            .send()
            .await?;
        let res = res.error_for_status()?;
        let refresh_token: OAuthRefreshToken = res.json().await?;
        oauth_token.access_token = refresh_token.access_token;
        oauth_token.expires_in = refresh_token.expires_in;
        Ok(oauth_token)
    }

    async fn request_token(
        client: &reqwest::Client,
        client_id: &str,
        client_secret: &str,
    ) -> Result<OAuthToken> {
        // 1. request access
        let params = json!({
            "client_id": client_id,
            "scope": Self::OAUTH_SCOPE,
        });
        let res = client
            .post(Self::OAUTH_CODE_URL)
            .form(&params)
            .send()
            .await?;
        let res = res.error_for_status()?;
        let oauth_res: YtMusicOAuthDeviceRes = res.json().await?;

        let auth_url = format!(
            "{}?user_code={}",
            oauth_res.verification_url, oauth_res.user_code
        );
        if webbrowser::open(&auth_url).is_err() {
            info!("Please authorize the app by visiting the following URL: {}", auth_url);
        } else {
            info!("Please authorize the app in your browser and press enter");
        }
        std::io::stdin().read_exact(&mut [0])?;

        // 2. request the token
        let mut params = HashMap::new();
        params.insert("client_id", client_id);
        params.insert("code", &oauth_res.device_code);
        params.insert("client_secret", client_secret);
        params.insert("grant_type", Self::OAUTH_GRANT_TYPE);
        let res = client
            .post(Self::OAUTH_TOKEN_URL)
            .form(&params)
            .send()
            .await?;
        let res = res.error_for_status()?;
        let token: OAuthToken = res.json().await?;
        Ok(token)
    }

    pub async fn new_headers(headers: &PathBuf, client_id: &str, client_secret: &str, oauth_token_path: PathBuf, config: ConfigArgs) -> Result<Self> {
        let header_data = std::fs::read_to_string(headers)?;
        let header_json: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&header_data)?;
        let mut headers = HeaderMap::new();
        for (key, val) in header_json.into_iter() {
            if let serde_json::Value::String(s) = val {
                headers.insert(
                    HeaderName::from_bytes(key.to_lowercase().as_bytes())?,
                    s.parse()?,
                );
            }
        }
        headers.remove("accept-encoding");
        headers.remove("content-encoding");

        let mut client = reqwest::ClientBuilder::new()
            .cookie_store(true)
            .default_headers(headers);

        if let Some(proxy) = &config.proxy {
            client = client
                .proxy(reqwest::Proxy::all(proxy)?)
                .danger_accept_invalid_certs(true)
        }

        let client = client.build().unwrap();

        Ok(YtMusicApi { client, client_id: client_id.to_string(), client_secret: client_secret.to_string(), oauth_token_path, last_token_refresh: Instant::now(), config })
    }

    fn build_endpoint(&self, path: &str, ctoken: Option<&str>) -> String {
        let mut endpoint = format!("{}{}{}", Self::BASE_API, path, Self::BASE_PARAMS,);
        if let Some(c) = ctoken {
            std::write!(&mut endpoint, "&ctoken={c}&continuation={c}", c = c).unwrap();
        }
        endpoint
    }

    fn add_context(&self, body: &serde_json::Value) -> serde_json::Value {
        let mut body = body.clone();
        match body.as_object_mut() {
            Some(o) => o.insert("context".to_string(), CONTEXT.clone()),
            _ => unreachable!(),
        };
        body
    }

    async fn paginated_request(
        &mut self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<YtMusicResponse> {
        let mut response: YtMusicResponse = self.make_request(path, body, None).await?;
        let mut continuation = response.get_continuation();

        while let Some(cont) = continuation {
            let mut response2: YtMusicContinuationResponse =
                self.make_request(path, body, Some(&cont)).await?;
            continuation = response2.get_continuation();
            response.merge(&mut response2);
        }
        Ok(response)
    }

    

    async fn make_request<T>(
        &mut self,
        path: &str,
        body: &serde_json::Value,
        ctoken: Option<&str>,
    ) -> Result<T>
    where
        T: DeserializeOwned + std::fmt::Debug,
    {
        // Refresh the token if more than 5 minutes have passed
        if self.last_token_refresh.elapsed() > Duration::from_secs(300) {
            info!("Refreshing token");
            self.update_refresh_token().await?;
        }

        let body = self.add_context(body);
        let endpoint = self.build_endpoint(path, ctoken);

        /* Switch to info for dev env */
        debug!("Requesting: {}", endpoint);

        let res = self.client.post(&endpoint).json(&body).send().await?;
        let obj = if self.config.debug {
            let status = res.status();
            let text = res.text().await?;
            std::fs::write("debug/yt_music_last_res.json", &text)?;
            if status.is_client_error() || status.is_server_error() {
                return Err(eyre!("Error: {}", text));
            }
            serde_json::from_str(&text)?
        } else {
            let res = res.error_for_status()?;
            res.json().await?
        };
        Ok(obj)
    }

    pub fn clean_playlist_id(id: &str) -> String {
        if let Some(id) = id.strip_prefix("VL") {
            return id.to_string();
        }
        id.to_string()
    }

    async fn update_refresh_token(&mut self) -> Result<()> {
        let client_id = &self.client_id;
        let client_secret = &self.client_secret;
        let oauth_token_path = &self.oauth_token_path;

        let reader = std::fs::File::open(oauth_token_path)?;
        let mut oauth_token: OAuthToken = serde_json::from_reader(reader)?;

        let params = json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "grant_type": "refresh_token",
            "refresh_token": &oauth_token.refresh_token,
        });
        let res = self.client
            .post(Self::OAUTH_TOKEN_URL)
            .form(&params)
            .send()
            .await?;
        let res = res.error_for_status()?;
        let refresh_token: OAuthRefreshToken = res.json().await?;
        oauth_token.access_token = refresh_token.access_token;
        oauth_token.expires_in = refresh_token.expires_in;

        // Write new token
        let mut file = std::fs::File::create(oauth_token_path)?;
        serde_json::to_writer(&mut file, &oauth_token)?;

        // Update the authorization header
        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", Self::OAUTH_USER_AGENT.parse()?);
        headers.insert("Cookie", "SOCS=CAI".parse()?);
        headers.insert(
            "Authorization",
            format!("Bearer {}", oauth_token.access_token).parse()?,
        );
        self.client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        // Update the last token refresh time
        self.last_token_refresh = Instant::now();

        Ok(())
    }
}

#[async_trait]
impl MusicApi for YtMusicApi {
    fn api_type(&self) -> MusicApiType {
        MusicApiType::YtMusic
    }

    fn country_code(&self) -> &str {
        // TODO: it seems impossible to get the country code from YtMusic
        "UNKNOWN"
    }

    async fn create_playlist(&mut self, name: &str, public: bool) -> Result<Playlist> {
        let privacy_status = match public {
            true => "PUBLIC",
            false => "PRIVATE",
        };
        let body = json!({
            "title": name,
            "description": PLAYLIST_DESC,
            "privacyStatus": privacy_status,
        });
        let response: YtMusicPlaylistCreateResponse =
            self.make_request("playlist/create", &body, None).await?;
        let id = Self::clean_playlist_id(&response.playlist_id);
        Ok(Playlist {
            id,
            name: name.to_string(),
            songs: vec![],
            owner: Some("".to_string()) // TODO: get the owner
        })
    }

    async fn get_playlists_info(&mut self) -> Result<Vec<Playlist>> {
        let browse_id = "FEmusic_liked_playlists";
        let body = json!({ "browseId": browse_id });
        let response = self.paginated_request("browse", &body).await?;
        let playlists: Playlists = response.try_into()?;
        Ok(playlists.0)
    }

    async fn get_playlist_songs(&mut self, id: &str) -> Result<Vec<Song>> {
        let browse_id = if id.starts_with("VL") {
            id.to_string()
        } else {
            format!("VL{}", id)
        };

        let body = json!({ "browseId": browse_id });
        let response = self.paginated_request("browse", &body).await?;
        let songs: Songs = response.try_into()?;
        Ok(songs.0)
    }

    async fn add_songs_to_playlist(&mut self, playlist: &mut Playlist, songs: &[Song]) -> Result<()> {
        for song in songs {
            playlist.songs.push(song.clone());
        }

        let mut actions = vec![];
        for song in songs.iter() {
            let action = json!({
                "action": "ACTION_ADD_VIDEO",
                "addedVideoId": song.id,
                "dedupeOption": "DEDUPE_OPTION_CHECK", // Allow youtube to check for duplicates
            });
            actions.push(action);
        }
        let body = json!({
            "playlistId": playlist.id,
            "actions": actions,
        });
        let response: YtMusicPlaylistEditResponse = self
            .make_request("browse/edit_playlist", &body, None)
            .await?;
        if !response.success() {
            
            if let Some(actions) = response.actions {
                for action in actions {

                    // Youtube Music sometimes returns a confirm dialog when adding duplicates
                    // This is a workaround to handle that by splitting the list and retrying
                    if let Some(confirm_dialog) = action.confirm_dialog_endpoint {
                        let title_contains_duplicates = confirm_dialog
                            .content
                            .confirm_dialog_renderer
                            .title
                            .runs
                            .iter()
                            .any(|run| run.text == "Duplicates");
    
                        if title_contains_duplicates {
                            // Handle duplicates by splitting the list and retrying
                            if songs.len() > 1 {
                                let mid = songs.len() / 2;
                                self.add_songs_to_playlist(playlist, &songs[..mid]).await?;
                                // Add a delay before the next recursive call
                                sleep(Duration::from_secs(3));
                                self.add_songs_to_playlist(playlist, &songs[mid..]).await?;
                            } else {
                                info!("Ignoring song already in playlist: {:?}", songs[0]);
                            }
                            return Ok(());
                        }
                    }

                    // Once we have split the list enough times to reach a single element
                    // we know that we have reached the song that is already in the playlist
                    // and we can ignore it
                    if let Some(add_to_toast_action) = action.add_to_toast_action {
                        let message_contains_already_in_playlist = add_to_toast_action
                            .item
                            .notification_action_renderer
                            .response_text
                            .runs
                            .iter()
                            .any(|run| run.text.contains("This track is already in the playlist"));

                        if message_contains_already_in_playlist {
                            info!("Ignoring song already in playlist: {:?}", songs[0]);
                            return Ok(());
                        }
                    }
                }
            }

            return Err(eyre!("Error adding song to playlist. Response: {:?}", response.status));
        }
        Ok(())
    }

    async fn remove_songs_from_playlist(
        &mut self,
        playlist: &mut Playlist,
        songs: &[Song],
    ) -> Result<()> {
        for song in songs {
            playlist.songs.retain(|s| s != song);
        }
        let mut actions = vec![];
        for song in songs.iter() {
            let action = json!({
                "setVideoId": song.sid.as_ref().ok_or(eyre!("Song setVideoId not found"))?,
                "removedVideoId": song.id,
                "action": "ACTION_REMOVE_VIDEO",
            });
            actions.push(action);
        }
        let body = json!({
            "playlistId": playlist.id,
            "actions": actions,
        });
        let response = self
            .make_request::<YtMusicPlaylistEditResponse>("browse/edit_playlist", &body, None)
            .await?;
        if response.success() {
            Ok(())
        } else {
            Err(eyre!("Error removing song from playlist"))
        }
    }

    async fn delete_playlist(&mut self, playlist: Playlist) -> Result<()> {
        let body = json!({
            "playlistId": playlist.id,
        });
        self.make_request::<YtMusicPlaylistDeleteResponse>("playlist/delete", &body, None)
            .await?;
        Ok(())
    }

    async fn search_song(&mut self, song: &Song) -> Result<Option<Song>> {
        debug!(
            "Searching for song: {} by {}",
            song.name,
            song.artists.iter().map(|artist| artist.name.as_str()).collect::<Vec<&str>>().join(", ")
        );

        if let Some(isrc) = &song.isrc {
            let body = json!({
                "query": format!("\"{}\"", isrc),
            });
            let response = self
                .make_request::<YtMusicResponse>("search", &body, None)
                .await?;
            let res_song: SearchSongUnique = response.try_into()?;
            if let Some(mut res_song) = res_song.0 {
                res_song.isrc = Some(isrc.to_string());
                return Ok(Some(res_song));
            }
        } else {
            let ignore_spelling = "AUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D";
            let params = format!("EgWKAQ{}{}", "II", ignore_spelling);
            let mut queries = song.build_queries();
            while let Some(query) = queries.pop() {
                let body = json!({
                    "query": query,
                    "params": params,
                });
                let response = self
                    .make_request::<YtMusicResponse>("search", &body, None)
                    .await?;
                let res_songs: SearchSongs = response.try_into()?;
                // iterate over top 3 results
                for res_song in res_songs.0.into_iter().take(3) {
                    if song.compare(&res_song) {
                        return Ok(Some(res_song));
                    }
                }
            }
        }
        Ok(None)
    }

    async fn add_like(&mut self, songs: &[Song]) -> Result<()> {
        // TODO: find a way to bulk-like
        for song in songs {
            let body = json!({
                "target": {
                    "videoId": song.id,
                }
            });
            let _: YtMusicAddLikeResponse = self.make_request("like/like", &body, None).await?;
        }
        Ok(())
    }

    async fn get_likes(&mut self) -> Result<Vec<Song>> {
        let songs = self.get_playlist_songs("LM").await?;
        Ok(songs)
    }
}

#[cfg(test)]
mod tests {}
