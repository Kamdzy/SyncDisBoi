use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use sync_dis_boi::ConfigArgs;
use tracing::Level;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct RootArgs {
    /// The source music platform
    #[command(subcommand)]
    pub src: MusicPlatformSrc,

    #[command(flatten)]
    pub config: ConfigArgs,

    /// Logging level
    #[arg(short, long, value_enum, default_value_t = LoggingLevel::Info)]
    pub logging: LoggingLevel,

    /// List of playlist names to skip, separated by '|'
    #[arg(long, use_value_delimiter = true, value_delimiter = '|')]
    pub skip_playlists: Vec<String>,
}

#[derive(Subcommand, Clone, Debug)]
#[command(subcommand_value_name = "SRC_PLATFORM")]
pub enum MusicPlatformSrc {
    YtMusic {
        /// The path to the headers JSON file
        #[arg(short, long)]
        headers: Option<PathBuf>,
        // FIXME: Android Auto Oauth is broken, probably forever
        // https://github.com/sigma67/ytmusicapi/discussions/682
        // https://github.com/yt-dlp/yt-dlp/issues/11462
        /// The client ID for the Youtube API application
        #[arg(
            long,
            env = "YTMUSIC_CLIENT_ID",
            conflicts_with = "headers",
            requires = "client_secret"
            //default_value = "861556708454-d6dlm3lh05idd8npek18k6be8ba3oc68.apps.googleusercontent.com"
        )]
        client_id: Option<String>,
        /// The client secret for the Youtube API application
        #[arg(
            long,
            env = "YTMUSIC_CLIENT_SECRET",
            conflicts_with = "headers",
            requires = "client_secret"
            //default_value = "SboVhoG9s0rNafixCSGGKXAT"
        )]
        client_secret: Option<String>,
        /// Clear the cached ytmusic_oauth.json file
        #[arg(long, requires = "client_id", requires = "client_secret")]
        clear_cache: bool,
        /// The owner of the playlists, this is required to know which playlists to skip
        #[arg(long,
            env = "YTMUSIC_OWNER")]
        owner: String,
        /// The destination music platform
        #[command(subcommand)]
        dst: MusicPlatformDst,
    },
    Spotify {
        /// The client ID for the Spotify API application
        #[arg(long, env = "SPOTIFY_CLIENT_ID")]
        client_id: String,
        /// The client secret for the Spotify API application
        #[arg(long, env = "SPOTIFY_CLIENT_SECRET")]
        client_secret: String,
        /// Clear the cached spotify_oauth.json file
        #[arg(long)]
        clear_cache: bool,
        /// The owner of the playlists, this is required to know which playlists to skip
        #[arg(long,
            env = "SPOTIFY_OWNER")]
        owner: String,
        /// The destination music platform
        #[command(subcommand)]
        dst: MusicPlatformDst,
    },
    Tidal {
        /// The client ID for the Tidal API application
        #[arg(long, env = "TIDAL_CLIENT_ID", default_value = "zU4XHVVkc2tDPo4t")]
        client_id: String,
        /// The client secret for the Tidal API application
        #[arg(
            long,
            env = "TIDAL_CLIENT_SECRET",
            default_value = "VJKhDFqJPqvsPVNBV6ukXTJmwlvbttP7wlMlrc72se4="
        )]
        client_secret: String,
        /// Clear the cached tidal_oauth.json file
        #[arg(long)]
        clear_cache: bool,
        /// The owner of the playlists, this is required to know which playlists to skip
        #[arg(long,
            env = "TIDAL_OWNER")]
        owner: String,
        /// The destination music platform
        #[command(subcommand)]
        dst: MusicPlatformDst,
    },
    Plex {
        #[arg(
            long,
            env = "PLEX_SERVER_URL",
            //default_value = "http://localhost:32400"
        )]
        server_url: String,
        /// The plex token to authenticate with the Plex server
        #[arg(
            long,
            env = "PLEX_TOKEN",
            //default_value = "SboVhoG9s0rNafixCSGGKXAT"
        )]
        plex_token: String,
        /// Music library to create playlists in
        #[arg(
            long,
            env = "MUSIC_LIBRARY",
            //default_value = "Music"
        )]
        music_library: String,
        /// The owner of the playlists, this is required to know which playlists to skip
        #[arg(long,
            env = "PLEX_OWNER")]
        owner: String,
        /// The destination music platform
        #[command(subcommand)]
        dst: MusicPlatformDst,
    },
}

// INFO: Hack to support command chaining with clap
// related issue: https://github.com/clap-rs/clap/issues/2222
#[derive(Subcommand, Clone, Debug)]
#[command(subcommand_value_name = "DST_PLATFORM")]
pub enum MusicPlatformDst {
    YtMusic {
        /// The path to the headers JSON file
        #[arg(short, long)]
        headers: Option<PathBuf>,
        // FIXME: Android Auto Oauth is broken, probably forever
        // https://github.com/sigma67/ytmusicapi/discussions/682
        // https://github.com/yt-dlp/yt-dlp/issues/11462
        /// The client ID for the Youtube API application
        #[arg(
            long,
            env = "YTMUSIC_CLIENT_ID",
            conflicts_with = "headers",
            requires = "client_secret"
            //default_value = "861556708454-d6dlm3lh05idd8npek18k6be8ba3oc68.apps.googleusercontent.com"
        )]
        client_id: Option<String>,
        /// The client secret for the Youtube API application
        #[arg(
            long,
            env = "YTMUSIC_CLIENT_SECRET",
            conflicts_with = "headers",
            requires = "client_secret"
            //default_value = "SboVhoG9s0rNafixCSGGKXAT"
        )]
        client_secret: Option<String>,
        /// Clear the cached ytmusic_oauth.json file
        #[arg(long, requires = "client_id", requires = "client_secret")]
        clear_cache: bool,
        /// The owner of the playlists, this is required to know which playlists to skip
        #[arg(long,
            env = "YTMUSIC_OWNER")]
        owner: String,
    },
    Spotify {
        /// The client ID for the Spotify API application
        #[arg(long, env = "SPOTIFY_CLIENT_ID")]
        client_id: String,
        /// The client secret for the Spotify API application
        #[arg(long, env = "SPOTIFY_CLIENT_SECRET")]
        client_secret: String,
        /// Clear the cached spotify_oauth.json file
        #[arg(long)]
        clear_cache: bool,
        /// The owner of the playlists, this is required to know which playlists to skip
        #[arg(long,
            env = "SPOTIFY_OWNER")]
        owner: String,
    },
    Tidal {
        /// The client ID for the Tidal API application
        #[arg(long, env = "TIDAL_CLIENT_ID", default_value = "zU4XHVVkc2tDPo4t")]
        client_id: String,
        #[arg(
            long,
            env = "TIDAL_CLIENT_SECRET",
            default_value = "VJKhDFqJPqvsPVNBV6ukXTJmwlvbttP7wlMlrc72se4="
        )]
        /// The client secret for the Tidal API application
        client_secret: String,
        /// Clear the cached tidal_oauth.json file
        #[arg(long)]
        clear_cache: bool,
        /// The owner of the playlists, this is required to know which playlists to skip
        #[arg(long,
            env = "TIDAL_OWNER")]
        owner: String,
    },
    
    Plex {
        #[arg(
            long,
            env = "PLEX_SERVER_URL",
            //default_value = "http://localhost:32400"
        )]
        server_url: String,
        /// The plex token to authenticate with the Plex server
        #[arg(
            long,
            env = "PLEX_TOKEN",
            //default_value = "SboVhoG9s0rNafixCSGGKXAT"
        )]
        plex_token: String,
        /// Music library name to create playlists in
        #[arg(
            long,
            env = "MUSIC_LIBRARY",
            //default_value = "Music"
        )]
        music_library: String,
        /// The owner of the playlists, this is required to know which playlists to skip
        #[arg(long,
            env = "PLEX_OWNER")]
        owner: String,
    },
    Export {
        /// The path to the file to export the playlists to
        #[arg(short, long)]
        dest: PathBuf,
        /// Minify the exported JSON file
        #[arg(long, default_value = "false")]
        minify: bool,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum LoggingLevel {
    /// Only log errors
    Error,
    /// Log errors and warnings
    Warn,
    /// Log errors, warnings and info
    Info,
    /// Log errors, warnings, info and debug (very verbose)
    Debug,
}

impl From<LoggingLevel> for Level {
    fn from(level: LoggingLevel) -> Self {
        match level {
            LoggingLevel::Warn => Level::WARN,
            LoggingLevel::Error => Level::ERROR,
            LoggingLevel::Info => Level::INFO,
            LoggingLevel::Debug => Level::DEBUG,
        }
    }
}
