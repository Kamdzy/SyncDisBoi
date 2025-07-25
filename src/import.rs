use std::path::Path;

use color_eyre::eyre::Result;
use tracing::info;

use crate::ConfigArgs;
use crate::music_api::{DynMusicApi, Playlist};
use crate::sync::synchronize_playlists;

pub async fn import(src_json: &Path, mut dst_api: DynMusicApi, config: ConfigArgs, skip_playlists: Vec<String>, dst_owner: String) -> Result<()> {
    let src_playlists: Vec<Playlist> = serde_json::from_reader(std::fs::File::open(src_json)?)?;

    info!("importing playlists...");
    synchronize_playlists(src_playlists, &mut dst_api, &config, skip_playlists, dst_owner).await?;
    info!(
        "successfully imported playlists to {:?}",
        dst_api.api_type()
    );

    Ok(())
}
