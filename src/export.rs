use std::path::Path;

use color_eyre::eyre::Result;
use tracing::info;

use crate::music_api::DynMusicApi;

pub async fn export(mut src_api: DynMusicApi, path: &Path, minify: bool) -> Result<()> {
    info!("retrieving playlists...");
    let src_playlists = src_api.get_playlists_full().await?;

    info!("exporting playlists...");
    if !minify {
        serde_json::to_writer_pretty(std::fs::File::create(path)?, &src_playlists)?;
    } else {
        serde_json::to_writer(std::fs::File::create(path)?, &src_playlists)?;
    }
    info!("successfully exported playlists to: {:?}", path);

    Ok(())
}
