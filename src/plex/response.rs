use std::convert::TryInto;
use color_eyre::eyre::{Error, Result};
use tracing::{error, warn};

use crate::music_api::{Album, Artist, MusicApiType, Playlist, Playlists, Song, Songs};
use super::model::{PlexCreatePlaylistResponse, PlexPlaylist, PlexPlaylistSongsResponse, PlexPlaylistsResponse, PlexSearchTrackResponse, Track};

impl TryInto<Playlist> for PlexPlaylist {
    type Error = Error;

    fn try_into(self) -> Result<Playlist, Self::Error> {
        Ok(Playlist {
            id: self.rating_key,
            name: self.title,
            songs: vec![],
            owner: None,
        })
    }
}

impl TryInto<Playlists> for PlexPlaylistsResponse {
    type Error = Error;

    fn try_into(self) -> Result<Playlists, Self::Error> {
        let mut result = vec![];
        for playlist in self.playlists.into_iter() {
            match playlist.try_into() {
                Ok(p) => result.push(p),
                Err(e) => {
                    error!("failed to parse playlist, skipping: {}", e);
                    continue;
                }
            }
        }
        Ok(Playlists(result))
    }
}

impl TryInto<Playlists> for PlexCreatePlaylistResponse {
    type Error = Error;

    fn try_into(self) -> Result<Playlists, Self::Error> {
        let mut result = vec![];
        for playlist in self.playlists.into_iter() {
            match playlist.try_into() {
                Ok(p) => result.push(p),
                Err(e) => {
                    error!("failed to parse playlist, skipping: {}", e);
                    continue;
                }
            }
        }
        Ok(Playlists(result))
    }
}

impl TryInto<Song> for Track {
    type Error = Error;

    fn try_into(self) -> Result<Song, Self::Error> {
        let album = if !self.parent_title.is_empty() {
            Some(Album {
                id: Some(self.parent_rating_key),
                name: self.parent_title,
            })
        } else {
            None
        };

        let artists = if !self.grandparent_title.is_empty() {
            vec![Artist {
                id: Some(self.grandparent_rating_key),
                name: self.grandparent_title,
            }]
        } else {
            vec![]
        };

        let title = if !self.title.is_empty() {
            self.title
        } else {
            self.title_sort
        };

        Ok(Song {
            id: self.rating_key,
            name: title,
            album,
            artists,
            duration_ms: self.duration as usize,
            source: MusicApiType::Plex,
            sid: None,
            isrc: None,
        })
    }
}

impl TryInto<Songs> for PlexPlaylistSongsResponse {
    type Error = Error;

    fn try_into(self) -> Result<Songs, Self::Error> {
        let mut result = vec![];

        if let Some(tracks) = self.tracks {
            for track in tracks.into_iter() {
                match track.try_into() {
                    Ok(s) => result.push(s),
                    Err(e) => {
                        error!("failed to parse song, skipping: {}", e);
                        continue;
                    }
                }
            }
        } else {
            warn!("no tracks found in playlist '{}' - {}", self.title, self.rating_key);
        }

        Ok(Songs(result))
    }
}

impl TryInto<Songs> for PlexSearchTrackResponse {
    type Error = Error;

    fn try_into(self) -> Result<Songs, Self::Error> {
        let mut result = vec![];

        if let Some(tracks) = self.tracks {
            for track in tracks.into_iter() {
                match track.try_into() {
                    Ok(s) => result.push(s),
                    Err(e) => {
                        error!("failed to parse song, skipping: {}", e);
                        continue;
                    }
                }
            }
        }

        Ok(Songs(result))
    }
}