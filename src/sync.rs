use color_eyre::eyre::{Result, eyre};
use serde_json::json;
use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};

use crate::ConfigArgs;
use crate::music_api::{DynMusicApi, MusicApiType, Playlist, Song};
use crate::utils::dedup_songs;

// TODO: Parse playlist owner to ignore platform-specific playlists?
const SKIPPED_PLAYLISTS: [&str; 12] = [
    // Yt Music specific
    "New playlist",
    "Your Likes",
    "My Supermix",
    "Discover Mix",
    "Episodes for Later",
    "New Release Mix",
    "Archive Mix",
    // Spotify specific
    "Liked Songs",
    "Discover Weekly",
    "Big Room House Mix",
    "Motivation Electronic Mix",
    "High Energy Mix",
];

pub async fn synchronize(
    mut src_api: DynMusicApi,
    mut dst_api: DynMusicApi,
    config: ConfigArgs,
    skip_playlists: Vec<String>,
    _src_owner: String,
    dst_owner: String,
) -> Result<()> {
    if !config.diff_country
        && src_api.api_type() != MusicApiType::YtMusic
        && dst_api.api_type() != MusicApiType::YtMusic
        && src_api.api_type() != MusicApiType::Plex // TODO: Revert once Plex country code is added
        && dst_api.api_type() != MusicApiType::Plex
        && src_api.country_code() != dst_api.country_code()
    {
        return Err(eyre!(
            "source and destination music platforms are in different countries ({} vs {}). \
                You can specify --diff-country to allow it, \
                but this might result in incorrect sync results.",
            src_api.country_code(),
            dst_api.country_code()
        ));
    }

    if config.debug {
        std::fs::create_dir_all("debug")?;
    }

    info!("retrieving source playlists...");
    let src_playlists = src_api.get_playlists_full().await?;

    synchronize_playlists(src_playlists, &mut dst_api, &config, skip_playlists, dst_owner).await?;

    if config.sync_likes {
        info!("synchronizing likes...");
        let src_likes = src_api.get_likes().await?;
        let dst_likes = dst_api.get_likes().await?;

        let mut new_likes = Vec::new();
        for src_like in src_likes.into_iter() {
            if dst_likes.contains(&src_like) {
                continue;
            }
            let Some(song) = dst_api.search_song(&src_like).await? else {
                debug!("no match found for song: {}", src_like);
                continue;
            };
            // HACK: takes into account discrepancy for YtMusic with no ISRC
            if dst_likes.contains(&song) {
                debug!("discrepancy, song already liked: {}", song);
                continue;
            }
            new_likes.push(song);
        }

        info!("synchronizing {} new likes", new_likes.len());
        dst_api.add_likes(&new_likes).await?;
    }

    Ok(())
}

pub async fn synchronize_playlists(
    mut src_playlists: Vec<Playlist>,
    dst_api: &mut DynMusicApi,
    config: &ConfigArgs,
    skip_playlists: Vec<String>,
    dst_owner: String,
) -> Result<()> {
    let mut all_missing_songs = json!({});
    let mut all_new_songs = json!({});
    let mut no_albums = json!({});
    let mut stats = json!({});

    info!("retrieving destination playlists...");
    let mut dst_playlists = dst_api.get_playlists_full().await?;
    let mut dst_likes = vec![];
    if config.like_all {
        info!("retrieving destination likes...");
        dst_likes = dst_api.get_likes().await?;
    }

    /* Filter to specific playlists */
    // Filter by playlist name
    //src_playlists.retain(|playlist| playlist.name == "sh22");

    // Filter by playlist owner if we want to sync only our own playlists
    // src_playlists.retain(|playlist| playlist.owner == Some(src_owner.to_string()));

    // Remove skipped playlists by matching name lowercase
    src_playlists.retain(|playlist| {
        !skip_playlists
            .iter()
            .any(|skipped| playlist.name.to_lowercase() == skipped.to_lowercase())
    });

    // Remove destinaton playlists that are not owned by our user
    dst_playlists.retain(|playlist| {
        if playlist.owner != Some(dst_owner.clone()) {
            warn!(
                "destination playlist \"{}\" is not owned by user \"{}\", skipping",
                playlist.name, dst_owner
            );

            // Remove matching playlist from source playlists
            if let Some(i) = src_playlists.iter().position(|p| p.name == playlist.name) {
                src_playlists.remove(i);
            }
            
            false
        } else {
            true
        }
    });

    
    static mut SONG_COUNTER: usize = 0;
    static mut SLEEP_DURATION: u64 = 180; // Initial sleep duration in seconds (3 minutes)

    for mut src_playlist in src_playlists
        .into_iter()
        .filter(|p| !SKIPPED_PLAYLISTS.contains(&p.name.as_str()) && !p.songs.is_empty())
    {
        if src_playlist.songs.is_empty() {
            continue;
        }

        let mut dst_playlist = match dst_playlists
            .iter()
            .position(|p| p.name == src_playlist.name)
        {
            Some(i) => dst_playlists.remove(i),
            None => dst_api.create_playlist(&src_playlist.name, false).await?,
        };

        let mut missing_songs = json!([]);
        let mut new_songs = json!([]);
        let no_albums_songs = json!([]);
        let mut dst_songs = vec![];
        let mut success = 0;
        let mut attempts = 0;

        if dedup_songs(&mut src_playlist.songs) {
            warn!(
                "duplicates found in source playlist \"{}\", they will be skipped",
                src_playlist.name
            );
        }

        info!("synchronizing playlist \"{}\" ...", src_playlist.name);

        // 1. Search for each song in the destination playlist
        for src_song in src_playlist.songs.iter() {
            // already in destination playlist
            if dst_playlist.songs.contains(src_song) {
                continue;
            }

            // YtMusic API rate limit workaround
            if dst_api.api_type() == MusicApiType::YtMusic {
                unsafe {
                    SONG_COUNTER += 1;
                    if SONG_COUNTER % 200 == 0 {
                        let sleep_duration = SLEEP_DURATION;
                        info!("Reached 200 songs, taking a {}-second break...", sleep_duration);
                        sleep(Duration::from_secs(sleep_duration)).await;
                        SLEEP_DURATION += 60; // Add 60 seconds to the sleep duration each time
                    }
                }
            }

            // no album metadata == youtube video
            /* Commented this part out, personal preference */
            // if src_song.album.is_none() {
            //     warn!(
            //         "No album metadata for source song \"{}\", skipping",
            //         src_song
            //     );
            //     if config.debug {
            //         no_albums_songs
            //             .as_array_mut()
            //             .unwrap()
            //             .push(json!(src_song));
            //     }
            //     continue;
            // }

            attempts += 1;

            let dst_song = dst_api.search_song(src_song).await?;
            let Some(dst_song) = dst_song else {
                debug!("no match found for song: {}", src_song);
                if config.debug {
                    missing_songs.as_array_mut().unwrap().push(json!(src_song));
                }
                continue;
            };
            dst_songs.push(dst_song);
            success += 1;
        }

        // 2. Add missing songs to the destination playlist
        if !dst_songs.is_empty() {
            let mut to_sync = Vec::new();
            for dst_song in dst_songs.iter() {
                // HACK: takes into account discrepancy for YtMusic with no ISRC
                if dst_playlist.songs.contains(dst_song) {
                    debug!(
                        "discrepancy, song already in destination playlist: {}",
                        dst_song
                    );
                    continue;
                }
                // Edge case: same song on different album/single that all resolve to the same
                // song on the destination platform resulting in duplicates
                if to_sync.contains(dst_song) {
                    debug!(
                        "discrepancy, duplicate song in songs to synchronize: {}",
                        dst_song
                    );
                    continue;
                }
                if config.debug {
                    new_songs.as_array_mut().unwrap().push(json!(dst_song));
                }
                to_sync.push(dst_song.clone());
            }
            dst_api
                .add_songs_to_playlist(&mut dst_playlist, &to_sync)
                .await?;

            // like all songs that were added
            if config.like_all {
                let new_likes = to_sync
                    .iter()
                    .filter(|s| !dst_likes.contains(s))
                    .cloned()
                    .collect::<Vec<Song>>();
                dst_api.add_likes(&new_likes).await?;
            }
        }

        let mut conversion_rate = 1.0;
        if attempts != 0 {
            conversion_rate = success as f64 / attempts as f64;
            info!(
                "synchronizing playlist \"{}\" [ok], {}/{} songs ({}%)",
                src_playlist.name,
                success,
                attempts,
                conversion_rate * 100.0
            );
        } else {
            info!(
                "synchronizing playlist \"{}\" [ok], no new songs to add",
                src_playlist.name
            );
        }

        if config.debug {
            stats.as_object_mut().unwrap().insert(
                src_playlist.name.clone(),
                json!({
                    "percentage": conversion_rate,
                    "number": format!("{}/{}", success, attempts),
                }),
            );
            std::fs::write(
                "debug/conversion_rate.json",
                serde_json::to_string_pretty(&stats)?,
            )?;

            if !new_songs.as_array().unwrap().is_empty() {
                all_new_songs
                    .as_object_mut()
                    .unwrap()
                    .insert(src_playlist.name.clone(), new_songs);
                std::fs::write(
                    "debug/new_songs.json",
                    serde_json::to_string_pretty(&all_new_songs)?,
                )?;
            }

            if !missing_songs.as_array().unwrap().is_empty() {
                all_missing_songs
                    .as_object_mut()
                    .unwrap()
                    .insert(src_playlist.name.clone(), missing_songs);
                std::fs::write(
                    "debug/missing_songs.json",
                    serde_json::to_string_pretty(&all_missing_songs)?,
                )?;
            }

            if !no_albums_songs.as_array().unwrap().is_empty() {
                no_albums
                    .as_object_mut()
                    .unwrap()
                    .insert(src_playlist.name.clone(), no_albums_songs);
                std::fs::write(
                    "debug/song_with_no_albums.json",
                    serde_json::to_string_pretty(&no_albums)?,
                )?;
            }
        }
    }

    info!("Synchronization complete!");

    Ok(())
}
