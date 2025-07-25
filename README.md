# SyncDisBoi - Sync this boy!

SyncDisBoi is a simple and efficient tool designed to synchronize playlists across different music streaming platforms. It currently supports:

- [Youtube Music](https://music.youtube.com/)
- [Spotify](https://open.spotify.com/)
- [Tidal](https://tidal.com/)
- [Plex](https://www.plex.tv/)

SyncDisBoi is the ideal tool for music enthusiasts who want to:

- Seamlessly migrate to a new music platform while preserving their playlists and likes
- Keep playlists in sync across multiple platforms and enjoy each platform's unique recommendation algorithms
- Export/Import existing playlists in a portable JSON format for easy backup or sharing

> **Disclaimer**: While SyncDisBoi doesn't perform any deletion operations, it's always a good practice to backup your playlists. I am not responsible for any unintended changes to your playlists.

## About this Fork

This fork is meant for personal use and includes several enhancements and new features compared to the original SyncDisBoi project. It includes changes that helped me to better manage my music playlists across different platforms, especially with the addition of Docker support and improved Plex integration.

In this fork, I've adjusted the song matching algorithm to be more lenient, allowing for a higher success rate when synchronizing playlists across different platforms. While this approach increases the possibility of occasional incorrect matches, it significantly improves the overall sync success rate, particularly for songs with inconsistent metadata across services. The 5-second duration tolerance (compared to the original 1-second) and other matching adjustments help overcome the metadata inconsistencies that commonly occur between music platforms.

## What's New in This Fork

This fork includes several enhancements and new features:

### 🐳 Docker Support

- **Docker containerization**: Added full Docker support with automated build and push to GitHub Container Registry
- **Environment variable configuration**: All settings can now be configured via environment variables
- **Smart entrypoint script**: Automatically loads configuration from `args.ini` file, allowing runtime configuration changes without rebuilding the Docker container
- **Multi-platform support**: Docker images built for linux/amd64

### 🎵 Plex Integration

- **Full Plex support**: Complete implementation for Plex Media Server as both source and destination
- **Plex API integration**: Direct communication with Plex servers using XML API
- **Music library management**: Create and manage playlists in your Plex music library
- **Smart search**: Advanced song matching using Plex's hub search capabilities

### 🔧 Enhanced Configuration

- **Environment variable support**: All command-line arguments can be set via environment variables
- **INI file configuration**: Use `args.ini` file for easy configuration management
- **Playlist filtering**: Skip specific playlists using `--skip-playlists` parameter
- **Owner filtering**: Filter playlists by owner to sync only your own playlists
- **Configurable callback settings**: Custom callback host and port for OAuth flows
- **Custom config directory**: Override default config directory location

### 🚀 Performance & Reliability Improvements

- **Rate limiting**: Intelligent rate limiting for YouTube Music API to prevent throttling
- **Retry logic**: Enhanced error handling with automatic retries for failed requests
- **Duplicate handling**: Better duplicate detection and handling across all platforms
- **Memory optimization**: Reduced memory usage for large playlist operations
- **Sequential processing**: More reliable sequential song processing instead of parallel requests
- **YT-Music**: Dynamic parsing of YouTube Music API responses to handle changes in the API structure

### 🎯 Enhanced Sync Features

- **Improved song matching**: More flexible duration matching (5-second tolerance instead of 1-second)
- **Better error handling**: More graceful handling of API errors and edge cases
- **Enhanced logging**: Better debugging information and progress tracking
- **Playlist ownership**: Track and respect playlist ownership across platforms

## Tool workflow

For the best experience, it is recommended to use a single "source of truth" for your playlists, i.e. a primary platform where all playlist modifications are made and then replicated downstream.
This ensures consistency across platforms when syncing.

Note that YouTube Music is not ideal for this role due to its limited and often inaccurate song metadata (most notably missing ISRC codes), which are essential for precise song matching when synchronizing to other platforms.

SyncDisBoi synchronization workflow:

- if the destination playlist does not exist, SyncDisBoi will create a new playlist containing the synchronized songs
- if the destination playlist already exists, SyncDisBoi will only add songs that are not already present
- if the `--sync-likes` option is specified, SyncDisBoi will also synchronize likes
- if the `--like-all` option is specified, SyncDisBoi will like all synchonized songs on the destination platform
- if the `--debug` option is specified, [debug mode](https://github.com/SilentVoid13/SyncDisBoi#debug-mode) will be enabled

By default, SyncDisBoi does not remove songs. This is a safety measure to prevent accidental data loss.
Consequently, deleting a song on the source platform and syncing will not remove it from the destination playlist.

## Accuracy

SyncDisBoi focuses on synchronization accuracy, ensuring that each track on the source playlist accurately matches the corresponding track on the destination playlist. This feature is particularly important when dealing with different versions of the same song (such as remastered versions, deluxe editions, live recordings, etc.).

If available, SyncDisBoi uses the [International Standard Recording Code (ISRC)](https://en.wikipedia.org/wiki/International_Standard_Recording_Code) to guarantee correct track matching.

When ISRC codes are not available on the platform API, SyncDisBoi falls back to verifying the following properties to ensure that the two tracks match:

- Song name resemblance score ([Levenshtein distance](https://en.wikipedia.org/wiki/Levenshtein_distance))
- Album name resemblance score ([Levenshtein distance](https://en.wikipedia.org/wiki/Levenshtein_distance))
- Song duration (with 5-second tolerance in this fork)

Notes:

- The artist names are not used because the metadata is inconsistent across platforms.
- For Youtube Music, videos without album metadata are now included in sync operations (configurable behavior) (fork only)

## Download and Build

Pre-built binaries of SyncDisBoi for Linux, Windows, and macOS are available under the [releases](https://github.com/SilentVoid13/SyncDisBoi/releases) section.

### Docker Usage

Pull and run the Docker image:

```bash
docker pull ghcr.io/kamdzy/sync_dis_boi:latest

# Run with environment variables
docker run -v ~/.config/SyncDisBoi:/root/.config/SyncDisBoi \
  -e SRC_PLATFORM="spotify" \
  -e DST_PLATFORM="plex" \
  -e SPOTIFY_CLIENT_ID="your_client_id" \
  -e SPOTIFY_CLIENT_SECRET="your_client_secret" \
  -e PLEX_SERVER_URL="http://your-plex-server:32400" \
  -e PLEX_TOKEN="your_plex_token" \
  -e PLEX_MUSIC_LIBRARY="Music" \
  ghcr.io/kamdzy/sync_dis_boi:latest
```

### Configuration File (args.ini)

SyncDisBoi supports configuration through an `args.ini` file, which should be placed in your configuration directory (`~/.config/SyncDisBoi/` on Linux). This provides a convenient way to manage your settings without exposing sensitive information in command lines or environment variables.

#### Basic Structure

Create an `args.ini` file with the following structure:

```ini
# General Configuration
DEBUG=false
LOGGING_LEVEL=info
SYNC_LIKES=false
LIKE_ALL=false
SKIP_PLAYLISTS=Discover Weekly|Daily Mix 1|Radio Station
SRC_PLATFORM=spotify
DST_PLATFORM=plex

# Spotify Configuration
SPOTIFY_CLIENT_ID=your_spotify_client_id_here
SPOTIFY_CLIENT_SECRET=your_spotify_client_secret_here
SPOTIFY_OWNER=your_spotify_username
SPOTIFY_CALLBACK_HOST=0.0.0.0
SPOTIFY_CALLBACK_PORT=8888

# YouTube Music Configuration
YTMUSIC_CLIENT_ID=your_youtube_music_client_id_here
YTMUSIC_CLIENT_SECRET=your_youtube_music_client_secret_here
YTMUSIC_OWNER=your_youtube_account_name

# Tidal Configuration
TIDAL_CLIENT_ID=your_tidal_client_id_here
TIDAL_CLIENT_SECRET=your_tidal_client_secret_here
TIDAL_OWNER=your_tidal_username

# Plex Configuration
PLEX_SERVER_URL=http://localhost:32400
PLEX_TOKEN=your_plex_token_here
PLEX_MUSIC_LIBRARY=Music Library Name
PLEX_OWNER=your_plex_username
```

#### Configuration Examples

**Example 1: Spotify to Plex Sync**

```ini
# Basic Spotify to Plex configuration
SRC_PLATFORM=spotify
DST_PLATFORM=plex

SPOTIFY_CLIENT_ID=abcd1234567890
SPOTIFY_CLIENT_SECRET=xyz9876543210
SPOTIFY_OWNER=myspotifyusername

PLEX_SERVER_URL=http://192.168.1.100:32400
PLEX_TOKEN=xxxxxxxxxxxxxxxxxxxx
PLEX_MUSIC_LIBRARY=Music
PLEX_OWNER=myplexusername

# Optional: Skip certain playlists
SKIP_PLAYLISTS=Discover Weekly|Daily Mix 1|Daily Mix 2|Release Radar

# Optional: Enable debug mode
DEBUG=true
LOGGING_LEVEL=debug
```

**Example 2: YouTube Music to Spotify Sync**

```ini
# YouTube Music to Spotify configuration
SRC_PLATFORM=yt-music
DST_PLATFORM=spotify

YTMUSIC_CLIENT_ID=your_youtube_client_id.apps.googleusercontent.com
YTMUSIC_CLIENT_SECRET=your_youtube_client_secret
YTMUSIC_OWNER=your_youtube_account_name

SPOTIFY_CLIENT_ID=your_spotify_client_id
SPOTIFY_CLIENT_SECRET=your_spotify_client_secret
SPOTIFY_OWNER=your_spotify_username

# Sync likes as well
SYNC_LIKES=true

# Custom callback for Docker usage
SPOTIFY_CALLBACK_HOST=0.0.0.0
SPOTIFY_CALLBACK_PORT=8080
```

**Example 3: Multi-platform Configuration**

```ini
# Complete configuration for all platforms
SRC_PLATFORM=spotify
DST_PLATFORM=plex
DEBUG=false
LOGGING_LEVEL=info
SYNC_LIKES=true
LIKE_ALL=false

# Spotify
SPOTIFY_CLIENT_ID=spotify_client_id_here
SPOTIFY_CLIENT_SECRET=spotify_client_secret_here
SPOTIFY_OWNER=spotify_username

# YouTube Music
YTMUSIC_CLIENT_ID=youtube_client_id.apps.googleusercontent.com
YTMUSIC_CLIENT_SECRET=youtube_client_secret
YTMUSIC_OWNER=your_youtube_account_name

# Tidal
TIDAL_OWNER=tidal_username

# Plex
PLEX_SERVER_URL=http://plex.mydomain.com:32400
PLEX_TOKEN=plex_token_here
PLEX_MUSIC_LIBRARY=Music Library
PLEX_OWNER=plex_username

# Skip auto-generated playlists
SKIP_PLAYLISTS=Discover Weekly|Daily Mix|Release Radar|On Repeat|Repeat Rewind
```

#### Using args.ini with Docker

When using Docker, mount your configuration directory and the args.ini file will be automatically loaded by the entrypoint script:

```bash
# Create configuration directory
mkdir -p ~/.config/SyncDisBoi

# Create your args.ini file
cat > ~/.config/SyncDisBoi/args.ini << EOF
SRC_PLATFORM=spotify
DST_PLATFORM=plex
SPOTIFY_CLIENT_ID=your_client_id
SPOTIFY_CLIENT_SECRET=your_client_secret
SPOTIFY_OWNER=your_username
PLEX_SERVER_URL=http://localhost:32400
PLEX_TOKEN=your_plex_token
PLEX_MUSIC_LIBRARY=Music
PLEX_OWNER=your_plex_username
DEBUG=true
EOF

# Run Docker container with no arguments needed
docker run -v ~/.config/SyncDisBoi:/root/.config/SyncDisBoi \
  ghcr.io/kamdzy/sync_dis_boi:latest
```

The smart entrypoint script detects changes to the args.ini file at runtime, which means you can:

1. Modify the args.ini file on your host machine
2. Run the container again with the same command
3. The new settings will be applied without rebuilding the container

When using `SRC_PLATFORM` and `DST_PLATFORM` in your args.ini, you can run the container with a simplified command:

```bash
docker run -v ~/.config/SyncDisBoi:/root/.config/SyncDisBoi ghcr.io/kamdzy/sync_dis_boi:latest
```

This is particularly useful for:

- Switching between different music platform configurations
- Updating API tokens or credentials that expire
- Testing different synchronization options

#### Configuration Priority

The configuration is loaded in the following priority order (higher priority overrides lower):

1. **Command-line arguments** (highest priority)
2. **Environment variables**
3. **args.ini file** (lowest priority)

This means you can have a base configuration in `args.ini` and override specific values with environment variables or command-line arguments as needed.

#### Security Notes

- **Keep your `args.ini` file secure**: It contains sensitive API credentials
- **File permissions**: Set appropriate permissions on your config file:
  ```bash
  chmod 600 ~/.config/SyncDisBoi/args.ini
  ```
- **Use environment variables in CI/CD**: For automated environments, prefer environment variables over config files

If you prefer to build SyncDisBoi from source you simply need the rust toolchain, e.g. available via [rustup](https://rustup.rs/).
A [Nix flake](https://github.com/SilentVoid13/SyncDisBoi/blob/master/flake.nix) is also available with a pre-configured environment with support for cross-compilation.

## Usage

To use SyncDisBoi, you need to set up account access for the API of the corresponding music platform. Setup instructions available below.

Here are some command examples:

### Traditional Command Line Usage

```bash
# sync from Youtube Music to Spotify
./sync_dis_boi \
    yt-music --client-id "<CLIENT_ID>" --client-secret "<CLIENT_SECRET>" --owner "your_username" \
    spotify --client-id "<CLIENT_ID>" --client-secret "<CLIENT_SECRET>" --owner "your_username"

# sync from Spotify to Plex, sync likes as well
./sync_dis_boi --sync-likes \
    spotify --client-id "<CLIENT_ID>" --client-secret "<CLIENT_SECRET>" --owner "your_username" \
    plex --server-url "http://localhost:32400" --plex-token "<TOKEN>" --music-library "Music" --owner "your_username"

# sync from Plex to Youtube Music, skip specific playlists
./sync_dis_boi --skip-playlists "Discover Weekly|Daily Mix" \
    plex --server-url "http://localhost:32400" --plex-token "<TOKEN>" --music-library "Music" --owner "your_username" \
    yt-music --client-id "<CLIENT_ID>" --client-secret "<CLIENT_SECRET>" --owner "your_username"

# sync with custom callback settings for Docker/remote usage
./sync_dis_boi \
    spotify --client-id "<CLIENT_ID>" --client-secret "<CLIENT_SECRET>" --callback-host "0.0.0.0" --callback-port "8888" --owner "your_username" \
    plex --server-url "http://localhost:32400" --plex-token "<TOKEN>" --music-library "Music" --owner "your_username"
```

### Using Configuration File

```bash
# Simple sync using args.ini configuration with SRC_PLATFORM and DST_PLATFORM defined
./sync_dis_boi

# Alternatively, you can explicitly specify the platforms (overriding args.ini)
./sync_dis_boi spotify plex

# Override specific settings from args.ini
./sync_dis_boi --debug

# Use custom config directory
./sync_dis_boi --config-dir /path/to/config
```

### Environment Variable Usage

Set environment variables and run:

```bash
export SPOTIFY_CLIENT_ID="your_client_id"
export SPOTIFY_CLIENT_SECRET="your_client_secret"
export PLEX_SERVER_URL="http://localhost:32400"
export PLEX_TOKEN="your_plex_token"
export PLEX_MUSIC_LIBRARY="Music"
export SPOTIFY_OWNER="your_spotify_username"
export PLEX_OWNER="your_plex_username"
export SKIP_PLAYLISTS="Discover Weekly|Daily Mix"
export DEBUG="true"
export SRC_PLATFORM="spotify"
export DST_PLATFORM="plex"

# Now you can run without specifying the source and destination platforms
./sync_dis_boi
```

### Export/Import

```bash
# export Plex playlists to JSON
./sync_dis_boi \
    plex --server-url "http://localhost:32400" --plex-token "<TOKEN>" --music-library "Music" --owner "your_username" \
    export -o ./plex.json

# import playlists to Plex from JSON
./sync_dis_boi \
    plex --server-url "http://localhost:32400" --plex-token "<TOKEN>" --music-library "Music" --owner "your_username" \
    import -i ./spotify.json
```

### Docker Examples

#### Using args.ini (Recommended)

```bash
# Create and edit your configuration file
mkdir -p ~/.config/SyncDisBoi
nano ~/.config/SyncDisBoi/args.ini

# Run with mounted config directory
docker run -v ~/.config/SyncDisBoi:/root/.config/SyncDisBoi \
  ghcr.io/kamdzy/sync_dis_boi:latest spotify plex
```

#### Using Environment Variables

```bash
docker run -v ~/.config/SyncDisBoi:/root/.config/SyncDisBoi \
  -e SPOTIFY_CLIENT_ID="your_client_id" \
  -e SPOTIFY_CLIENT_SECRET="your_client_secret" \
  -e SPOTIFY_OWNER="your_username" \
  -e PLEX_SERVER_URL="http://host.docker.internal:32400" \
  -e PLEX_TOKEN="your_plex_token" \
  -e PLEX_MUSIC_LIBRARY="Music" \
  -e PLEX_OWNER="your_plex_username" \
  ghcr.io/kamdzy/sync_dis_boi:latest spotify plex
```

#### Docker Compose Example

```yaml
version: "3.8"
services:
  sync-dis-boi:
    image: ghcr.io/kamdzy/sync_dis_boi:latest
    volumes:
      - ~/.config/SyncDisBoi:/root/.config/SyncDisBoi
    environment:
      - SPOTIFY_CLIENT_ID=your_client_id
      - SPOTIFY_CLIENT_SECRET=your_client_secret
      - SPOTIFY_OWNER=your_username
      - PLEX_SERVER_URL=http://plex:32400
      - PLEX_TOKEN=your_plex_token
      - PLEX_MUSIC_LIBRARY=Music
      - PLEX_OWNER=your_plex_username
      - DEBUG=true
    command: ["spotify", "plex"]
    depends_on:
      - plex
```

### Spotify API setup

- Visit [https://developer.spotify.com/](https://developer.spotify.com/), go to your dashboard and create an application.
- Add `http://localhost:8888/callback` as a redirect URI in your application settings (or your custom callback URL if using Docker).
- Copy the application client id and client secret.

You will then need to provide the client id and client secret as arguments for SyncDisBoi.
After the first authorization, the OAuth token will be cached in `~/.config/SyncDisBoi/spotify_oauth.json` (on Linux) for future use.

Notes:

- The callback URL is now configurable using `--callback-host` and `--callback-port` parameters
- For Docker usage, use `--callback-host 0.0.0.0` to bind to all interfaces
- When using Docker on remote servers, you may need to set up port forwarding for OAuth callbacks

### Youtube Music API setup

The convenient OAuth "Android Auto" access has been removed by Youtube. You now have to create your own OAuth application:

- Sign in at [https://console.developers.google.com/](https://console.developers.google.com/)
- Create a new project
- Select the project
- Under "Enabled APIs & services" click "+ Enable APIs and services", select "Youtube Data API v3" and enable
- Under "OAuth consent screen" create an "external" user type (fill in the app name, set the developer email as your own)
- Add your own email for "Test users"
- Under "Credentials" click "+ Create credentials" > OAuth client ID > Set "TVs and Limited Input devices" as the application type
- Copy the Client ID and the Client secret

You will then need to provide the client id and client secret as arguments for SyncDisBoi.
After the first authorization, the OAuth token will be cached in `~/.config/SyncDisBoi/ytmusic_oauth.json` (on Linux) for future use.

Alternatively, you can use request headers to login:

- Follow [ytmusicapi's guide](https://ytmusicapi.readthedocs.io/en/stable/setup/browser.html) to generate a `browser.json` file.
- Pass the `browser.json` file as an argument for SyncDisBoi

Notes:

- Automatic token refresh is now implemented to prevent frequent re-authentication
- Enhanced rate limiting prevents API throttling during large sync operations

### Tidal API setup

- On the first run, SyncDisBoi will open up a browser tab to request OAuth access for your Tidal Account.
- Authorize the application in your browser, then press ENTER in the CLI to continue.

After the first authorization, the OAuth token will be cached in `~/.config/SyncDisBoi/tidal_oauth.json` (on Linux) for future use.

Notes:

- By default, SyncDisBoi uses Tidal's "Android Auto" application credentials to request OAuth access.
- However, you can also create your own Tidal application and then use it in SyncDisBoi by providing its client id and client secret.

### Plex API setup

- Obtain your Plex token from your Plex server:
  1. Sign in to your Plex Web App
  2. Go to Settings > Account > Privacy & Online Media Sources
  3. Click "Show" next to your Plex Pass or account info to reveal your token
- Ensure your Plex server is accessible from where you're running SyncDisBoi
- Identify your music library name (usually "Music" by default)

You will need to provide:

- `--server-url`: Your Plex server URL (e.g., `http://localhost:32400`)
- `--plex-token`: Your Plex authentication token
- `--music-library`: The name of your music library in Plex
- `--owner`: Your Plex username

#### Docker Network Considerations for Plex

When using Docker, you may need to adjust the Plex server URL:

- **Local Plex server**: Use `http://host.docker.internal:32400` (Docker Desktop) or `http://172.17.0.1:32400` (Linux)
- **Remote Plex server**: Use the actual IP address or domain name
- **Docker Compose**: Use the service name if Plex is also in Docker Compose

Notes:

- Plex integration supports both creating new playlists and adding songs to existing ones
- The tool uses Plex's advanced search capabilities for accurate song matching
- Ensure your Plex server has a properly configured music library with metadata

### Debug mode

You can enable debug mode (`--debug`) to generate detailed statistics about the synchronization process.

Files are saved in the `debug/` folder:

- `conversion_rate.json`: success rate of song synchronization
- `missing_songs.json`: list of tracks that couldn't be synchronized
- `new_songs.json`: list of tracks successfully synchronized
- `songs_with_no_albums.json`: list of songs skipped due to missing album metadata

## Environment Variables Reference

All command-line parameters can be set via environment variables:

### General Settings

- `CONFIG_DIR`: Custom configuration directory
- `DEBUG`: Enable debug mode (`true`/`false`)
- `LOGGING_LEVEL`: Logging level (`debug`, `info`, `warn`, `error`)
- `SYNC_LIKES`: Synchronize likes (`true`/`false`)
- `LIKE_ALL`: Like all synchronized songs on destination (`true`/`false`)
- `SKIP_PLAYLISTS`: Playlist names to skip, separated by `|`
- `SRC_PLATFORM`: Source platform (`spotify`, `yt-music`, `tidal`, or `plex`)
- `DST_PLATFORM`: Destination platform (`spotify`, `yt-music`, `tidal`, `plex`, `export`, or `import`)

### Spotify Settings

- `SPOTIFY_CLIENT_ID`: Spotify application client ID
- `SPOTIFY_CLIENT_SECRET`: Spotify application client secret
- `SPOTIFY_OWNER`: Spotify username
- `SPOTIFY_CALLBACK_HOST`: OAuth callback host (default: `localhost`)
- `SPOTIFY_CALLBACK_PORT`: OAuth callback port (default: `8888`)

### YouTube Music Settings

- `YTMUSIC_CLIENT_ID`: YouTube Music OAuth client ID
- `YTMUSIC_CLIENT_SECRET`: YouTube Music OAuth client secret
- `YTMUSIC_OWNER`: YouTube Music account owner
- `YTMUSIC_BROWSER_JSON`: Path to browser.json file (alternative auth method)

### Tidal Settings

- `TIDAL_CLIENT_ID`: Tidal OAuth client ID (optional)
- `TIDAL_CLIENT_SECRET`: Tidal OAuth client secret (optional)
- `TIDAL_OWNER`: Tidal account owner

### Plex Settings

- `PLEX_SERVER_URL`: Plex server URL
- `PLEX_TOKEN`: Plex authentication token
- `PLEX_MUSIC_LIBRARY`: Plex music library name
- `PLEX_OWNER`: Plex username

## Common Issues and Troubleshooting

### Docker Networking Issues

- **Plex connection fails**: Use `http://host.docker.internal:32400` instead of `localhost` on Docker Desktop
- **OAuth callbacks fail**: Set `CALLBACK_HOST=0.0.0.0` for container environments

### Authentication Issues

- **Token expired**: Delete cached token files in `~/.config/SyncDisBoi/` and re-authenticate
- **OAuth timeouts**: Check your redirect URIs match exactly (including trailing slashes)

### Sync Issues

- **Songs not found**: Enable debug mode to see which songs failed to match
- **Rate limiting**: The tool automatically handles rate limits, but large libraries may take time
- **Duplicate playlists**: Use `--skip-playlists` to avoid syncing auto-generated playlists

### Configuration Issues

- **Settings not loading**: Check file permissions on `args.ini` and ensure proper format
- **Environment variables ignored**: Remember that command-line args override environment variables

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to contribute to this project.

## License

SyncDisBoi is licensed under the GNU AGPLv3 license. Refer to [LICENSE](LICENSE.txt) for more information.

## Support the original author

Your support helps me continue to maintain and improve this project. If you find SyncDisBoi useful and want to show your appreciation, consider sponsoring or donating:

- GitHub Sponsors: Preferred method. You can sponsor me on [GitHub Sponsors](https://github.com/sponsors/SilentVoid13).
- PayPal: You can also make a donation via [PayPal](https://www.paypal.com/donate?hosted_button_id=U2SRGAFYXT32Q).

Every bit of support is greatly appreciated!

[![GitHub Sponsors](https://img.shields.io/github/sponsors/silentvoid13?label=Sponsor&logo=GitHub%20Sponsors&style=for-the-badge)](https://github.com/sponsors/silentvoid13)
[![Paypal](https://img.shields.io/badge/paypal-silentvoid13-yellow?style=social&logo=paypal)](https://www.paypal.com/donate?hosted_button_id=U2SRGAFYXT32Q)
