use async_trait::async_trait;
use color_eyre::eyre::{eyre, Ok};
use color_eyre::Result;
use model::{PlexCreatePlaylistResponse, PlexHubSearchResponse, PlexLibrarySectionsResponse, PlexPlaylist, PlexPlaylistSongsResponse, PlexPlaylistsResponse, PlexSearchTrackResponse, PlexUriResponse, PlexUserResponse, Track};
use reqwest::header::HeaderMap;
use urlencoding::encode;

use crate::music_api::{MusicApi, MusicApiType, Playlist, Playlists, Song, Songs};
use crate::ConfigArgs;

mod model;
mod response;

#[allow(dead_code)]
pub struct PlexApi {
    client: reqwest::Client,
    server_url: String,
    config: ConfigArgs,
    user_id: String,
    music_library: String,
    uri_root: String
}

impl PlexApi {
    pub async fn new(server: &str, token: &str, music_library: &String, config: ConfigArgs) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("X-Plex-Token", token.parse()?);

        let mut client_builder = reqwest::Client::builder()
            .cookie_store(true)
            .default_headers(headers);

        if let Some(proxy_url) = &config.proxy {
            client_builder = client_builder
                .proxy(reqwest::Proxy::all(proxy_url)?)
                .danger_accept_invalid_certs(true);
        }
    
        let client = client_builder.build()?;

        // Fetch user info
        let response = client
            // Query server + /myplex
            .get(format!("{}/myplex/account", server))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let logged_in_user: PlexUserResponse = serde_xml_rs::from_str(&response)?;

        // Fetch URI root info
        let uri_response = client
        .get(format!("{}/", server))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
        let uri_response_parsed: PlexUriResponse = serde_xml_rs::from_str(&uri_response)?;

        let uri_root = format!("server://{}/com.plexapp.plugins.library", uri_response_parsed.machine_identifier);

        Ok(Self {
            client,
            server_url: server.into(),
            config,
            user_id: logged_in_user.username,
            music_library: music_library.into(),
            uri_root: uri_root.into()
        })

    }
    
    async fn get_library_id_by_name(&self) -> Result<String> {
        let response = self.client
            .get(format!("{}/library/sections", self.server_url))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let parsed_res: PlexLibrarySectionsResponse = serde_xml_rs::from_str(&response)?;

        
        if let Some(directories) = parsed_res.directories {
         
            for res_section in directories.into_iter() {
                if let Some(title) = &res_section.title {
                    if title == &self.music_library {
                        if let Some(key) = &res_section.key {
                            return Ok(key.clone());
                        }
                    }
                }
            }
        }

        Err(eyre!("No library found for name: {}", self.music_library))
    }


    async fn get_first_library_track(&self) -> Result<Song> {
        let library_id = self.get_library_id_by_name().await?;

        let response = self.client
            .get(format!("{}/library/sections/{}/all?type=10&limit=10", self.server_url, library_id))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let container: PlexPlaylistSongsResponse = serde_xml_rs::from_str(&response)?;

        let res_songs: Songs = container.try_into()?;

        /* Throw error if no tracks available */
        if res_songs.0.is_empty() {
            return Err(eyre!("No tracks found in library: {}", self.music_library));
        }

        Ok(res_songs.0[0].clone())
    }

    async fn get_playlist_tracks(&self, playlist: &Playlist) -> Result<Vec<Track>> {
        // get all songs in a playlist
        let response = self.client
            .get(format!("{}/playlists/{}/items", self.server_url, playlist.id))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        // 2) Deserialize XML into your PlexPlaylistSongsResponse struct
        let container: PlexPlaylistSongsResponse = serde_xml_rs::from_str(&response)?;

        if let Some(songs) = container.tracks {
            return Ok(songs);
        }

        Ok(vec![])
    }

    async fn encode_query(&self, query: &str) -> Result<String> {
        let encoded = encode(&query);

        // Remove invalid trailing characters from encoded query
        // This causes a crash in Plex search
        let encoded = encoded.trim_end_matches("%2F").trim_end_matches("%3F").to_string().replace("%29", "");


        Ok(encoded)
    }

    #[allow(dead_code)]
    async fn search_song_strict(&self, query: &str) -> Result<Vec<Song>> {
        let encoded_query = self.encode_query(query).await?;
        let response = self.client
            .get(format!("{}/search?type=10&query={}",
                self.server_url, encoded_query))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let parsed_res: PlexSearchTrackResponse = serde_xml_rs::from_str(&response)?;

        let res_songs: Songs = parsed_res.try_into()?;

        Ok(res_songs.0)
    }

    async fn search_song_hub(&self, query: &str) -> Result<Vec<Song>> {
        let encoded_query = self.encode_query(query).await?;
        let response = self.client
            .get(format!("{}/library/search?searchTypes=music&query={}",
                self.server_url, encoded_query))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let parsed_res: PlexHubSearchResponse = serde_xml_rs::from_str(&response)?;
        
        let mut res_songs: Vec<Song> = vec![];

        for search_result in parsed_res.search_results {
            if let Some(tracks) = search_result.tracks {
                let songs: Result<Vec<Song>> = tracks.into_iter().map(|t| t.try_into()).collect();
                res_songs.extend(songs?);
            }
        }

        Ok(res_songs)
    }

}

#[async_trait]
impl MusicApi for PlexApi {
    fn api_type(&self) -> MusicApiType {
        MusicApiType::Plex
    }

    fn country_code(&self) -> &str {
        // TODO: it seems that plex has a country code, just need to convert it properly
        "UNKNOWN"
    }

    async fn create_playlist(&mut self, name: &str, _public: bool) -> Result<Playlist> {
        // Get first track from library
        let first_track = self.get_first_library_track().await?;

        // Construct the URI
        let uri = format!("{}/library/metadata/{}", self.uri_root, first_track.id);

        // Create a new playlist on Plex
        let response = self.client
            .post(format!("{}/playlists", self.server_url))
            .query(&[
                ("uri", uri.as_str()),
                ("type", "audio"),
                ("title", name),
                ("smart", "0"),
            ])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        // Deserialize XML into your PlexPlaylist struct
        let container: PlexCreatePlaylistResponse = serde_xml_rs::from_str(&response)?;

        // Convert to Playlist
        let playlists: Playlists = container.try_into()?;

        // Get the playlist tracks
        let tracks = self.get_playlist_tracks(&playlists.0[0]).await?;

        if !tracks.is_empty() {
            for track in tracks {
                let playlist_item_id = track.playlist_item_id.to_string();
            
                self.client
                    .delete(format!("{}/playlists/{}/items/{}", self.server_url, playlists.0[0].id, playlist_item_id))
                    .send()
                    .await?
                    .error_for_status()?;
            }
        }
        
        Ok(playlists.0[0].clone())
    }
    
    async fn get_playlists_info(&mut self) -> Result<Vec<Playlist>> {
        // get all playlist names and ids
        let response = self.client
            .get(format!("{}/playlists", self.server_url))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;


        // 2) Deserialize XML into your PlexPlaylistsResponse struct
        let container: PlexPlaylistsResponse = serde_xml_rs::from_str(&response)?;
        let playlists: Vec<PlexPlaylist> = container.playlists.clone();

        /* filter down to audio playlists */
        let filtered: Vec<PlexPlaylist> = playlists
        .iter()
        .filter(|p| p.playlist_subtype == "audio")
        .cloned()
        .collect();

        // Construct a new PlexPlaylistsResponse with only the filtered playlists
        let audio_container = PlexPlaylistsResponse {
            size: Some(filtered.len() as u32),
            playlists: filtered,
        };
    
        // Convert to Playlists
        let mid_playlists: Playlists = audio_container.try_into()?;

        // Add ourselves as owner of every playlist
        let res_playlists: Vec<Playlist> = mid_playlists.0.into_iter().map(|mut p| {
            p.owner = Some(self.user_id.clone());
            p
        }).collect();

        Ok(res_playlists)
    }

    async fn get_playlist_songs(&mut self, id: &str) -> Result<Vec<Song>> {
        // get all songs in a playlist
        let response = self.client
            .get(format!("{}/playlists/{}/items", self.server_url, id))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        // 2) Deserialize XML into your PlexPlaylistSongsResponse struct
        let container: PlexPlaylistSongsResponse = serde_xml_rs::from_str(&response)?;

        // Convert to Songs
        let res_songs: Songs = container.try_into()?;
        Ok(res_songs.0)
    }

    async fn add_songs_to_playlist(&mut self, playlist: &mut Playlist, songs: &[Song]) -> Result<()> {
        // add songs to a playlist in batches of 5
        for chunk in songs.chunks(5) {
            let rating_keys: Vec<String> = chunk.iter()
                .map(|song| song.id.clone())
                .collect();
            let rating_keys_str = rating_keys.join(",");
            let uri = format!("{}/library/metadata/{}", self.uri_root, rating_keys_str);

            self.client
                .put(format!("{}/playlists/{}/items", self.server_url, playlist.id))
                .query(&[
                    ("uri", uri.as_str())
                ])
                .send()
                .await?
                .error_for_status()?;
        }

        Ok(())
    }
    async fn remove_songs_from_playlist(
        &mut self,
        _playlist: &mut Playlist,
        _songs_ids: &[Song],
    ) -> Result<()> {
        todo!()
    }
    async fn delete_playlist(&mut self, _playlist: Playlist) -> Result<()> {
        todo!()
    }

    async fn search_song(&mut self, song: &Song) -> Result<Option<Song>> {
        let mut queries = song.build_queries();

        while let Some(query) = queries.pop() {
            // let res_songs = self.search_song_strict(&query).await?; // Second option, this gets less results
            let res_songs = self.search_song_hub(&query).await?;
            
            for res_song in res_songs.into_iter() {
                if song.compare(&res_song) {
                    return Ok(Some(res_song));
                }
            }
        }

        Ok(None)
    }

    async fn add_likes(&mut self, _songs: &[Song]) -> Result<()> {
        Ok(())
        // todo!()
    }
    async fn get_likes(&mut self) -> Result<Vec<Song>> {
        Ok(vec![])
        //todo!()
    }
}