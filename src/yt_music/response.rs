use color_eyre::eyre::{Error, Result, eyre};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::YtMusicApi;
use super::model::YtMusicResponse;
use crate::music_api::{Album, Artist, MusicApiType, Playlist, Playlists, Song, Songs};

#[derive(Deserialize, Serialize, Debug)]
pub struct SearchSongs(pub Vec<Song>);

#[derive(Deserialize, Serialize, Debug)]
pub struct SearchSongUnique(pub Option<Song>);

pub fn parse_duration(duration_str: &str) -> Result<usize> {
    let multipliers = [1, 60, 3600];
    let mut seconds = 0;
    for (i, part) in duration_str.rsplit(':').enumerate() {
        seconds += part.parse::<usize>()? * multipliers[i];
    }
    Ok(seconds * 1000)
}

impl TryInto<Playlists> for YtMusicResponse {
    type Error = Error;

    fn try_into(mut self) -> Result<Playlists, Self::Error> {
        let mut playlists = vec![];
        for mtrir in self
            .get_mtrirs()
            .ok_or(eyre!("No mtrirs found in response, are you authenticated? You may need to refresh your cookie/token."))?
            .iter()
            // Ignore the first "New Playlist" playlist
            // Ignore the second "Your Likes" playlist
            .skip(2)
        {
            let id = mtrir.get_id().ok_or(eyre!("No playlist id"))?;
            let id = YtMusicApi::clean_playlist_id(&id);
            let name = mtrir
                .get_name()
                .ok_or(eyre!("No playlist name"))?
                .trim()
                .to_string();
            let owner = mtrir
                .get_owner()
                .ok_or(eyre!("No playlist owner name"))?
                .trim()
                .to_string();
            let playlist = Playlist {
                id,
                name,
                songs: vec![],
                owner: Some(owner)
            };
            playlists.push(playlist);
        }

        let playlists = Playlists(playlists);
        Ok(playlists)
    }
}

impl TryInto<Songs> for YtMusicResponse {
    type Error = Error;

    fn try_into(mut self) -> Result<Songs, Self::Error> {
        let mut songs_vec = vec![];

        let mrlirs = match self.get_mrlirs() {
            Some(x) => x,
            // No songs in the playlist
            None => return Ok(Songs(songs_vec)),
        };

        for mrlir in mrlirs
            .iter()
            // Ignore unavailable songs
            .filter(|item| item.playlist_item_data.is_some())
        {
            let id = mrlir.get_id().ok_or(eyre!("No song id"))?;
            let set_id = mrlir.get_set_id().ok_or(eyre!("No song set_id"))?;

            let mut duration_str = {
                // Check both fixed_columns and flex_columns for the duration
                let fixed_cols = mrlir.fixed_columns.as_ref();
                let flex_cols = mrlir.flex_columns.as_ref();

                if let Some(fixed_cols) = fixed_cols {
                    // debug!("Found {} fixed_columns", fixed_cols.len());
                    // Get the first fixed column
                    let fixed_col = fixed_cols.get(0)
                        .ok_or(eyre!("No fixed column at index 0"))?;
                    // debug!("Fixed column 0: {:?}", fixed_col);
                    
                    // Access the renderer text
                    let text = &fixed_col.music_responsive_list_item_fixed_column_renderer.text;
                    // debug!("Fixed column text field: {:?}", text);
                    
                    // Get the runs array
                    let runs = text.runs.as_ref()
                        .ok_or(eyre!("No runs found in fixed column text"))?;
                    // debug!("Runs in fixed column: {:?}", runs);
                    
                    // Get the first run and extract its text
                    let first_run = runs.first()
                        .ok_or(eyre!("No first run in fixed column text"))?;
                    // debug!("First run: {:?}", first_run);
                    
                    first_run.text.clone()
                } else if let Some(flex_cols) = flex_cols {
                    // debug!("Found {} flex_columns", flex_cols.len());
                    // Get the third flex column (assuming duration is in the third column)
                    let flex_col = flex_cols.get(2)
                        .ok_or(eyre!("No flex column at index 2"))?;
                    // debug!("Flex column 2: {:?}", flex_col);
                    
                    // Access the renderer text
                    let text = &flex_col.music_responsive_list_item_flex_column_renderer.text;
                    // debug!("Flex column text field: {:?}", text);
                    
                    // Get the runs array
                    let runs = text.runs.as_ref()
                        .ok_or(eyre!("No runs found in flex column text"))?;
                    // debug!("Runs in flex column: {:?}", runs);
                    
                    // Get the first run and extract its text
                    let first_run = runs.first()
                        .ok_or(eyre!("No first run in flex column text"))?;
                    // debug!("First run: {:?}", first_run);
                    
                    first_run.text.clone()
                } else {
                    return Err(eyre!("No fixed_columns or flex_columns in item {:?}", mrlir));
                }
            };

            if duration_str.is_empty() || !duration_str.contains(":") {
                // Attempt to find duration from another source or format
                let alternative_duration_str = {
                    if let Some(flex_cols) = &mrlir.flex_columns {
                        if let Some(flex_col) = flex_cols.get(3) {
                            if let Some(runs) = &flex_col.music_responsive_list_item_flex_column_renderer.text.runs {
                                if let Some(run) = runs.first() {
                                    run.text.clone()
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                };
            
                if alternative_duration_str.is_empty() || !alternative_duration_str.contains(":") {
                    return Err(eyre!("Failed to extract duration from fixed_columns or alternative source"));
                } else {
                    duration_str = alternative_duration_str;
                }
            };

            if duration_str.is_empty() || !duration_str.contains(":") {
                info!("Full item data: {:?}", mrlir);
                info!("Flex columns: {:?}", mrlir.flex_columns);
                info!("Fixed columns: {:?}", mrlir.fixed_columns);
                return Err(eyre!("Failed to extract duration from fixed_columns"));
                // Crash the program if the duration is not found
            };
            
            let duration = parse_duration(&duration_str)?;
            debug!("Parsed duration (ms): {}", duration);

            // fc0 = song title
            // fc1 = artists
            // fc2 = album

            let name = mrlir.get_col_run_text(0, 0, true).ok_or(eyre!("No name"))?;
            let album = mrlir.get_col_runs(2, true).and_then(|_| {
                Some(Album {
                    id: mrlir.get_col_run_id(2, 0, true),
                    name: mrlir.get_col_run_text(2, 0, true)?,
                })
            });
            let mut artists: Vec<Artist> = vec![];
            for run in mrlir
                .get_col_runs(1, true)
                .ok_or(eyre!("No flex col 1"))?
                .iter()
                .step_by(2)
            {
                artists.push(Artist {
                    name: run.get_text(),
                    id: run.get_id(),
                });
            }
            let song = Song {
                source: MusicApiType::YtMusic,
                id,
                sid: Some(set_id),
                isrc: None,
                name,
                artists,
                album,
                duration_ms: duration,
            };

            songs_vec.push(song);
        }

        let songs: Songs = Songs(songs_vec);
        Ok(songs)
    }
}

impl TryInto<SearchSongs> for YtMusicResponse {
    type Error = Error;

    fn try_into(mut self) -> Result<SearchSongs, Self::Error> {
        let mut songs_vec = vec![];

        let mrlirs = match self.get_mrlirs() {
            Some(x) => x,
            None => return Ok(SearchSongs(songs_vec)),
        };

        let re_duration = Regex::new(r"^(\d+:)*\d+:\d+$")?;

        for mrlir in mrlirs
            .iter()
            .filter(|item| item.playlist_item_data.is_some())
        {
            let id = mrlir.get_id().ok_or(eyre!("No song id"))?;
            let name = mrlir.get_col_run_text(0, 0, true).ok_or(eyre!("No name"))?;

            // fc0 = song title
            // fc1 = artists, album, duration

            let mut album = None;
            let mut artists: Vec<Artist> = vec![];
            let mut duration = 0;

            for run in mrlir
                .get_col_runs(1, true)
                .ok_or(eyre!("No flex col 1"))?
                .iter()
                .step_by(2)
            {
                let text = run.get_text();
                if let Some(nav) = &run.navigation_endpoint {
                    let id = nav
                        .browse_endpoint
                        .as_ref()
                        .ok_or(eyre!("No browse endpoint"))?
                        .browse_id
                        .clone();
                    if id.starts_with("MPRE") {
                        album = Some(Album {
                            id: Some(id),
                            name: text,
                        });
                    } else {
                        artists.push(Artist {
                            id: Some(id),
                            name: text,
                        });
                    }
                } else if re_duration.is_match(&text) {
                    duration = parse_duration(&text)?;
                } else {
                    debug!("artist without id: {}", text);
                    artists.push(Artist {
                        id: None,
                        name: text,
                    });
                }
            }
            if album.is_none() || artists.is_empty() || duration == 0 {
                debug!("skipping song with missing data: {}", name);
                continue;
            }
            let song = Song {
                source: MusicApiType::YtMusic,
                id,
                sid: None,
                isrc: None,
                name,
                artists,
                album,
                duration_ms: duration,
            };

            songs_vec.push(song);
        }

        let songs = SearchSongs(songs_vec);
        Ok(songs)
    }
}

impl TryInto<SearchSongUnique> for YtMusicResponse {
    type Error = Error;

    fn try_into(mut self) -> Result<SearchSongUnique, Self::Error> {
        let card_shelf = match self.get_card_shelf() {
            Some(x) => x,
            None => return Ok(SearchSongUnique(None)),
        };

        let id = card_shelf.get_id().ok_or(eyre!("No song id"))?;
        let name = card_shelf.get_name().ok_or(eyre!("No song name"))?;

        // fc0 = song title
        // fc1 = artists, album, duration

        let mut album = None;
        let mut artists: Vec<Artist> = vec![];
        let mut duration = 0;
        let re_duration = Regex::new(r"^(\d+:)*\d+:\d+$")?;

        for run in card_shelf
            .subtitle
            .as_ref()
            .ok_or(eyre!("no subtitle"))?
            .runs
            .as_ref()
            .ok_or(eyre!("no subtitle.runs"))?
            .iter()
            .step_by(2)
            .skip(1)
        {
            let text = run.get_text();

            if let Some(nav) = &run.navigation_endpoint {
                let id = nav
                    .browse_endpoint
                    .as_ref()
                    .ok_or(eyre!("No browse endpoint"))?
                    .browse_id
                    .clone();
                if id.starts_with("MPRE") {
                    album = Some(Album {
                        id: Some(id),
                        name: text,
                    });
                } else {
                    artists.push(Artist {
                        id: Some(id),
                        name: text,
                    });
                }
            } else if re_duration.is_match(&text) {
                duration = parse_duration(&text)?;
            } else {
                debug!("artist without id: {}", text);
                artists.push(Artist {
                    id: None,
                    name: text,
                });
            }
        }

        // FIXME: it looks like album metadata is never present in search results
        // maybe there's a way to get it?
        //if album.is_none() || artists.is_empty() || duration == 0 {
        //    debug!("skipping song with missing data: {}", name);
        //    return Ok(SearchSongUnique(None));
        //}

        let song = Song {
            source: MusicApiType::YtMusic,
            id,
            sid: None,
            isrc: None,
            name,
            artists,
            album,
            duration_ms: duration,
        };
        Ok(SearchSongUnique(Some(song)))
    }
}
