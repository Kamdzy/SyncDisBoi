pub mod model;
mod response;

use std::collections::HashMap;
use std::fmt::Write;
use std::io::{self, Read, BufRead};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use atty::Stream;

use async_trait::async_trait;
use color_eyre::eyre::{Result, eyre};
use model::{YtMusicAddLikeResponse, YtMusicOAuthDeviceRes};
use reqwest::header::{HeaderMap, HeaderName};
use serde::de::DeserializeOwned;
use serde_json::json;
use sha1::{Sha1, Digest};
use tokio::time::Instant;
use tracing::{debug, info, warn};

use self::model::{YtMusicContinuationResponse, YtMusicPlaylistEditResponse, YtMusicResponse};
use crate::ConfigArgs;
use crate::music_api::{
    MusicApi, MusicApiType, OAuthRefreshToken, OAuthToken, PLAYLIST_DESC, Playlist, Playlists,
    Song, Songs,
};
use crate::utils::debug_response_json;
use crate::yt_music::model::{YtMusicPlaylistCreateResponse, YtMusicPlaylistDeleteResponse};
use crate::yt_music::response::{SearchSongUnique, SearchSongs};

static CONTEXT: LazyLock<serde_json::Value> = LazyLock::new(|| {
    json!({
        "client": {
            "clientName": "WEB_REMIX",
            "clientVersion": "1.20251006.01.00",
            "hl": "en"
        },
        "user": {}
    })
});

#[derive(Debug, Clone)]
pub enum YtMusicAuthType {
    OAuth {
        client_id: String,
        client_secret: String,
        oauth_token_path: PathBuf,
    },
    Browser {
        headers_path: PathBuf,
        sapisid: String,
        origin: String,
    },
}

pub struct YtMusicApi {
    client: reqwest::Client,
    auth_type: YtMusicAuthType,
    last_token_refresh: Instant,
    config: ConfigArgs,
}

/// Action to take after rate limit detection
enum RateLimitAction {
    /// Retry the request after the specified backoff duration
    Retry(Duration),
    /// Rate limit was not detected, continue with normal processing
    Continue,
    /// Maximum retries exceeded, abort the request
    MaxRetriesExceeded,
}

impl YtMusicApi {
    const BASE_API: &'static str = "https://music.youtube.com/youtubei/v1/";
    const BASE_PARAMS: &'static str = "?alt=json&key=AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX30";

    const OAUTH_SCOPE: &'static str = "https://www.googleapis.com/auth/youtube";
    const OAUTH_CODE_URL: &'static str = "https://www.youtube.com/o/oauth2/device/code";
    const OAUTH_TOKEN_URL: &'static str = "https://oauth2.googleapis.com/token";
    const OAUTH_GRANT_TYPE: &'static str = "http://oauth.net/grant_type/device/1.0";
    const OAUTH_USER_AGENT: &'static str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:88.0) Gecko/20100101 Firefox/88.0 Cobalt/Version";
    const RES_DEBUG_FILENAME: &'static str = MusicApiType::YtMusic.short_name();
    
    // Retry configuration for rate limiting
    const MAX_RETRIES: u32 = 5;  // 6 total attempts (0-5)
    const MAX_BACKOFF_SECS: u64 = 900;  // Cap exponential backoff at 120 seconds

    /// Create a new YtMusicApi instance using browser authentication
    pub async fn new_browser(headers_path: PathBuf, config: ConfigArgs) -> Result<Self> {
        let header_data = std::fs::read_to_string(&headers_path)?;
        let header_json: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&header_data)?;
        
        // Validate required headers
        let has_cookie = header_json.contains_key("cookie") || header_json.contains_key("Cookie");
        
        if !has_cookie {
            return Err(eyre!("Missing 'cookie' header in browser authentication file. Please run the setup again."));
        }
        
        // Extract cookie string
        let cookie_str = header_json
            .get("cookie")
            .or_else(|| header_json.get("Cookie"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| eyre!("Cookie header is not a string"))?;
        
        // Extract __Secure-3PAPISID from cookies for SAPISIDHASH generation
        let sapisid = Self::extract_sapisid(cookie_str)?;
        
        // Get origin URL
        let origin = header_json
            .get("origin")
            .or_else(|| header_json.get("x-origin"))
            .and_then(|v| v.as_str())
            .unwrap_or("https://music.youtube.com")
            .to_string();
        
        info!("Browser auth initialized with origin: {}", origin);
        
        let mut headers = HeaderMap::new();
        
        for (key, val) in header_json.into_iter() {
            if let serde_json::Value::String(s) = val {
                // Skip the authorization header - we'll generate it dynamically
                if key.to_lowercase() == "authorization" {
                    continue;
                }
                
                // Skip X-Goog-Visitor-Id - we'll always fetch it fresh
                if key.to_lowercase() == "x-goog-visitor-id" {
                    continue;
                }
                
                // For cookie header, ensure SOCS=CAI is always present
                if key.to_lowercase() == "cookie" {
                    let enhanced_cookies = Self::ensure_socs_cookie(&s);
                    headers.insert(
                        HeaderName::from_bytes(key.to_lowercase().as_bytes())?,
                        enhanced_cookies.parse()?,
                    );
                    continue;
                }
                
                headers.insert(
                    HeaderName::from_bytes(key.to_lowercase().as_bytes())?,
                    s.parse()?,
                );
            }
        }
        
        // Always fetch X-Goog-Visitor-Id fresh from YouTube Music on initialization
        debug!("Fetching X-Goog-Visitor-Id from YouTube Music...");
        
        // Build a temporary client with base headers only
        let temp_client = reqwest::Client::builder()
            .cookie_store(true)
            .default_headers(headers.clone())
            .build()?;
        
        if let Ok(visitor_id) = Self::fetch_visitor_id(&temp_client, &origin).await {
            debug!("Fetched X-Goog-Visitor-Id: {}", visitor_id);
            headers.insert("x-goog-visitor-id", visitor_id.parse()?);
        } else {
            warn!("Failed to fetch X-Goog-Visitor-Id, stopping initialization");
            return Err(eyre!("Failed to fetch X-Goog-Visitor-Id, cannot continue."));
        }
        
        // Remove encoding headers that can cause issues
        headers.remove("accept-encoding");
        headers.remove("content-encoding");
        headers.remove("content-length");

        let mut client = reqwest::ClientBuilder::new()
            .cookie_store(true)
            .default_headers(headers);

        if let Some(proxy) = &config.proxy {
            client = client
                .proxy(reqwest::Proxy::all(proxy)?)
                .danger_accept_invalid_certs(true)
        }
        let client = client.build()?;

        let auth_type = YtMusicAuthType::Browser { 
            headers_path,
            sapisid,
            origin,
        };

        Ok(YtMusicApi { 
            client, 
            auth_type,
            last_token_refresh: Instant::now(), 
            config 
        })
    }
    
    /// Ensure SOCS=CAI cookie is present in cookie string
    /// SOCS (Sign Out Cookie Status) is required by Google to acknowledge cookie policy
    /// See: https://policies.google.com/technologies/cookies
    fn ensure_socs_cookie(cookie_str: &str) -> String {
        // Check if SOCS is already present
        let has_socs = cookie_str
            .split(';')
            .any(|pair| pair.trim().starts_with("SOCS="));
        
        if has_socs {
            // SOCS already present, return as-is
            cookie_str.to_string()
        } else {
            // Add SOCS=CAI to cookies
            // CAI = Cookie Acknowledgement Information
            if cookie_str.trim().is_empty() {
                "SOCS=CAI".to_string()
            } else {
                format!("{}; SOCS=CAI", cookie_str)
            }
        }
    }
    
    /// Fetch X-Goog-Visitor-Id from YouTube Music homepage
    async fn fetch_visitor_id(client: &reqwest::Client, origin: &str) -> Result<String> {
        // reqwest with gzip feature automatically handles decompression
        let response = client
            .get(origin)
            .send()
            .await?
            .text()
            .await?;
        
        // Look for ytcfg.set({...}) in the response
        let re = regex::Regex::new(r"ytcfg\.set\s*\(\s*(\{.+?\})\s*\)\s*;")?;
        
        for captures in re.captures_iter(&response) {
            if let Some(json_str) = captures.get(1) {
                // Try to parse as JSON
                if let Ok(ytcfg) = serde_json::from_str::<serde_json::Value>(json_str.as_str()) {
                    if let Some(visitor_data) = ytcfg.get("VISITOR_DATA") {
                        if let Some(visitor_id) = visitor_data.as_str() {
                            if !visitor_id.is_empty() {
                                return Ok(visitor_id.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        Err(eyre!("Could not extract VISITOR_DATA from YouTube Music response"))
    }
    
    /// Extract __Secure-3PAPISID from cookie string
    fn extract_sapisid(cookie_str: &str) -> Result<String> {
        for cookie_pair in cookie_str.split(';') {
            let cookie_pair = cookie_pair.trim();
            if let Some((key, value)) = cookie_pair.split_once('=') {
                if key.trim() == "__Secure-3PAPISID" {
                    return Ok(value.trim().to_string());
                }
            }
        }
        Err(eyre!(
            "Missing '__Secure-3PAPISID' cookie. This cookie is required for YouTube Music authentication.\n\
            Please ensure you're logged into YouTube Music and copy headers from an authenticated request."
        ))
    }
    
    /// Generate SAPISIDHASH authorization header
    /// This must be regenerated for each request with the current timestamp
    fn generate_sapisidhash(sapisid: &str, origin: &str) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let hash_input = format!("{} {} {}", timestamp, sapisid, origin);
        let mut hasher = Sha1::new();
        hasher.update(hash_input.as_bytes());
        let hash_result = hasher.finalize();
        let hash_hex = format!("{:x}", hash_result);
        
        format!("SAPISIDHASH {}_{}", timestamp, hash_hex)
    }

    /// Create a new YtMusicApi instance using OAuth2 authentication
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
            Self::request_token(&client, client_id, client_secret, &config).await?
        } else {
            info!("refreshing token");
            Self::refresh_token(
                &client,
                client_id,
                client_secret,
                &oauth_token_path,
                &config,
            )
            .await?
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
                .danger_accept_invalid_certs(true);
        }
        let client = client.build()?;

        let auth_type = YtMusicAuthType::OAuth {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            oauth_token_path,
        };

        Ok(YtMusicApi { 
            client, 
            auth_type,
            last_token_refresh: Instant::now(), 
            config 
        })
    }

    /// Setup browser authentication from raw headers text (non-interactive)
    /// 
    /// This is the recommended method for Docker/non-interactive environments.
    /// Headers can be provided as a multi-line string with "Header: Value" format.
    /// 
    /// # Arguments
    /// * `headers_raw` - Raw headers string with each header on a new line
    /// * `filepath` - Optional path to save the processed headers JSON
    /// 
    /// # Returns
    /// JSON string of processed headers
    pub fn setup_browser_from_raw(headers_raw: &str, filepath: Option<PathBuf>) -> Result<String> {
        let contents: Vec<String> = headers_raw.split('\n').map(|s| s.to_string()).collect();
        Self::parse_and_save_headers(contents, filepath)
    }

    /// Setup browser authentication from a raw headers file (non-interactive)
    /// 
    /// Reads headers from a text file containing raw browser headers.
    /// Perfect for Docker containers where you can mount a headers file.
    /// 
    /// # Arguments
    /// * `input_file` - Path to file containing raw headers
    /// * `output_file` - Optional path to save the processed headers JSON
    /// 
    /// # Returns
    /// JSON string of processed headers
    pub fn setup_browser_from_file(input_file: &PathBuf, output_file: Option<PathBuf>) -> Result<String> {
        let headers_raw = std::fs::read_to_string(input_file)?;
        Self::setup_browser_from_raw(&headers_raw, output_file)
    }

    /// Setup browser authentication interactively
    /// 
    /// This method prompts the user to paste headers from stdin.
    /// For non-interactive environments (like Docker), use `setup_browser_from_raw` or 
    /// `setup_browser_from_file` instead.
    /// 
    /// # Arguments
    /// * `filepath` - Optional path to save the headers JSON
    /// * `headers_raw` - Optional raw headers string (if provided, skips interactive input)
    pub fn setup_browser(filepath: Option<PathBuf>, headers_raw: Option<String>) -> Result<String> {
        let contents = if let Some(raw) = headers_raw {
            raw.split('\n').map(|s| s.to_string()).collect::<Vec<_>>()
        } else {
            // Check if running in non-interactive mode
            if !atty::is(Stream::Stdin) {
                return Err(eyre!(
                    "Running in non-interactive mode. Please use setup_browser_from_raw() or \
                    setup_browser_from_file() instead, or provide headers_raw parameter."
                ));
            }
            
            let eof = if cfg!(windows) { "'Enter, Ctrl-Z, Enter'" } else { "Ctrl-D" };
            println!("Please paste the request headers from Firefox and press {} to continue:", eof);
            
            let stdin = io::stdin();
            let mut contents = Vec::new();
            
            for line in stdin.lock().lines() {
                match line {
                    Ok(l) => contents.push(l),
                    Err(_) => break,
                }
            }
            contents
        };

        Self::parse_and_save_headers(contents, filepath)
    }

    /// Internal helper to parse headers and save to file
    fn parse_and_save_headers(contents: Vec<String>, filepath: Option<PathBuf>) -> Result<String> {
        let mut user_headers: HashMap<String, String> = HashMap::new();
        let mut chrome_remembered_key = String::new();

        for content in contents {
            let parts: Vec<&str> = content.splitn(2, ": ").collect();
            
            // Skip chromium-style headers starting with ':'
            if parts[0].starts_with(':') {
                continue;
            }
            
            // Handle Chrome's "copy-paste in separate lines" format
            if parts[0].ends_with(':') {
                chrome_remembered_key = parts[0].trim_end_matches(':').to_string();
                continue;
            }
            
            if parts.len() == 1 {
                if !chrome_remembered_key.is_empty() {
                    user_headers.insert(chrome_remembered_key.clone(), parts[0].to_string());
                    chrome_remembered_key.clear();
                }
                continue;
            }
            
            if parts.len() == 2 {
                user_headers.insert(parts[0].to_lowercase(), parts[1].to_string());
            }
        }

        // Validate required headers
        let missing_headers: Vec<&str> = ["cookie", "x-goog-authuser"]
            .iter()
            .filter(|&key| !user_headers.contains_key(*key))
            .copied()
            .collect();

        if !missing_headers.is_empty() {
            return Err(eyre!(
                "The following entries are missing in your headers: {}. \
                Please try a different request (such as /browse) and make sure you are logged in.",
                missing_headers.join(", ")
            ));
        }

        // Remove headers that should be ignored
        let ignore_headers = ["host", "content-length", "accept-encoding"];
        for key in &ignore_headers {
            user_headers.remove(*key);
        }
        
        // Remove all headers starting with "sec-"
        user_headers.retain(|key, _| !key.starts_with("sec-"));

        // Add default headers
        user_headers.insert("user-agent".to_string(), 
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:88.0) Gecko/20100101 Firefox/88.0".to_string());
        user_headers.insert("accept".to_string(), "*/*".to_string());
        user_headers.insert("accept-language".to_string(), "en-US,en;q=0.5".to_string());
        user_headers.insert("content-type".to_string(), "application/json".to_string());
        user_headers.insert("x-goog-visitor-id".to_string(), 
            user_headers.get("x-goog-visitor-id").cloned().unwrap_or_default());

        let headers_json = serde_json::to_string_pretty(&user_headers)?;

        if let Some(path) = filepath {
            std::fs::write(path, &headers_json)?;
        }

        Ok(headers_json)
    }

    async fn refresh_token(
        client: &reqwest::Client,
        client_id: &str,
        client_secret: &str,
        oauth_token_path: &PathBuf,
        config: &ConfigArgs,
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
        let status = res.status();
        let refresh_token: OAuthRefreshToken =
            debug_response_json(config, res, Self::RES_DEBUG_FILENAME).await?;
        if !status.is_success() {
            return Err(eyre!("Invalid HTTP status: {}", status));
        }

        oauth_token.access_token = refresh_token.access_token;
        oauth_token.expires_in = refresh_token.expires_in;
        Ok(oauth_token)
    }

    async fn request_token(
        client: &reqwest::Client,
        client_id: &str,
        client_secret: &str,
        config: &ConfigArgs,
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
        let status = res.status();
        let oauth_res: YtMusicOAuthDeviceRes =
            debug_response_json(config, res, Self::RES_DEBUG_FILENAME).await?;
        if !status.is_success() {
            return Err(eyre!("Invalid HTTP status: {}", status));
        }

        let auth_url = format!(
            "{}?user_code={}",
            oauth_res.verification_url, oauth_res.user_code
        );
        if webbrowser::open(&auth_url).is_err() {
            info!("Please authorize the app by visiting the following URL: {}", auth_url);
        } else {
            info!("Please authorize the app in your browser and press enter");
        }

        if atty::is(Stream::Stdin) {
            // Interactive: wait for user input
            std::io::stdin().read_exact(&mut [0])?;
        } else {
            for i in (0..60).step_by(5) {
                info!("Waiting for user authorization... {} seconds remaining", 60 - i);
                sleep(Duration::from_secs(5));
            }
            info!("Countdown finished. Proceeding with next steps.");
        }

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
        let status = res.status();
        let token: OAuthToken = debug_response_json(config, res, Self::RES_DEBUG_FILENAME).await?;
        if !status.is_success() {
            return Err(eyre!("Invalid HTTP status: {}", status));
        }

        Ok(token)
    }

    /*pub fn new_headers(headers: &PathBuf, config: ConfigArgs) -> Result<Self> {
        let header_data = std::fs::read_to_string(headers)?;
        let header_json: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&header_data)?;
        let mut headers = HeaderMap::new();
        for (key, val) in header_json {
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
                .danger_accept_invalid_certs(true);
        }
        let client = client.build()?;

        Ok(YtMusicApi { client, auth_type, last_token_refresh: Instant::now(), config  })
    }*/

    fn build_endpoint(path: &str, ctoken: Option<&str>) -> String {
        let mut endpoint = format!("{}{}{}", Self::BASE_API, path, Self::BASE_PARAMS,);
        if let Some(c) = ctoken {
            std::write!(&mut endpoint, "&ctoken={c}&continuation={c}", c = c).unwrap();
        }
        endpoint
    }

    fn add_context(body: &serde_json::Value) -> serde_json::Value {
        let mut body = body.clone();
        match body.as_object_mut() {
            Some(o) => o.insert("context".to_string(), CONTEXT.clone()),
            _ => unreachable!(),
        };
        body
    }

    /// Check for authentication errors in the response
    fn check_authentication_errors(&self, text: &str) -> Result<()> {
        // YouTube Music can return not-logged-in status in two formats:
        // 1. Direct: "logged_in": "0" 
        // 2. ServiceTrackingParams: {"key": "logged_in", "value": "0"}
        let is_not_logged_in = 
            // Check for direct format
            text.contains(r#""logged_in":"0"#) || text.contains(r#""logged_in": "0"#)
            // Check for key-value format in serviceTrackingParams
            || ((text.contains(r#""key":"logged_in""#) || text.contains(r#""key": "logged_in""#)) 
                && (text.contains(r#""value":"0""#) || text.contains(r#""value": "0""#)));
        
        if is_not_logged_in {
            if matches!(self.auth_type, YtMusicAuthType::Browser { .. }) {
                return Err(eyre!(
                    "Authentication failed: Not logged in (logged_in=0).\n\
                    Your browser authentication tokens have expired or are invalid.\n\
                    This usually means your cookies or SAPISID have expired.\n\n\
                    Please refresh your headers by running:\n  \
                    cargo run --example setup_ytmusic_browser\n\n\
                    Make sure you:\n\
                    1. Are logged into YouTube Music in your browser\n\
                    2. Copy headers from an authenticated request (like /browse)\n\
                    3. Include ALL cookies, especially __Secure-3PAPISID"
                ));
            } else {
                return Err(eyre!(
                    "Authentication failed: Not logged in (logged_in=0).\n\
                    OAuth authentication may have expired or been revoked."
                ));
            }
        }
        
        // Check for sign-in prompts in response
        if text.contains(r#""text":"Sign in""#) || 
           (text.contains("signInEndpoint") && text.contains("messageRenderer")) {
            if matches!(self.auth_type, YtMusicAuthType::Browser { .. }) {
                return Err(eyre!(
                    "YouTube Music is requesting sign-in.\n\
                    Your browser cookies have expired or are invalid.\n\
                    Please refresh your headers by running:\n  \
                    cargo run --example setup_ytmusic_browser"
                ));
            } else {
                return Err(eyre!(
                    "YouTube Music is requesting sign-in.\n\
                    OAuth authentication needs to be refreshed."
                ));
            }
        }
        
        Ok(())
    }

    /// Handle rate limiting detection and retry logic
    async fn handle_rate_limit_with_retry(
        status: reqwest::StatusCode,
        text: &str,
        retry_count: u32,
    ) -> Result<RateLimitAction> {
        // Detect rate limiting: HTTP 429 or Google's HTML "automated queries" response
        let is_rate_limited = status.as_u16() == 429 
            || (status.is_client_error() && text.contains("automated queries"));
        
        if !is_rate_limited {
            return Ok(RateLimitAction::Continue);
        }
        
        if retry_count >= Self::MAX_RETRIES {
            return Ok(RateLimitAction::MaxRetriesExceeded);
        }
        
        // Calculate exponential backoff: 3^(retry_count + 1) seconds, capped at MAX_BACKOFF_SECS
        let backoff_secs = 3u64.pow(retry_count + 1).min(Self::MAX_BACKOFF_SECS);
        warn!(
            "Rate limit hit (attempt {}/{}). Waiting {} seconds before retry...",
            retry_count + 1,
            Self::MAX_RETRIES + 1,
            backoff_secs
        );
        
        Ok(RateLimitAction::Retry(Duration::from_secs(backoff_secs)))
    }

    /// Save HTTP error diagnostic data with auto-detected file type and return the file path
    fn save_http_error_diagnostic(status: reqwest::StatusCode, text: &str) -> Result<String> {
        // Ensure debug directory exists
        let debug_dir = std::path::Path::new("debug");
        if !debug_dir.exists() {
            let _ = std::fs::create_dir("debug");
        }
        
        // Detect file type based on content
        let extension = if text.trim_start().to_lowercase().starts_with("<html") {
            "html"
        } else if serde_json::from_str::<serde_json::Value>(text).is_ok() {
            "json"
        } else {
            "txt"
        };
        
        // Save diagnostic data with timestamp and detected extension
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let error_file = format!("debug/error_{}_{}.{}", status.as_u16(), timestamp, extension);
        std::fs::write(&error_file, text)?;
        
        Ok(error_file)
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
        // Refresh the token if more than 5 minutes have passed (OAuth only)
        if matches!(self.auth_type, YtMusicAuthType::OAuth { .. }) 
            && self.last_token_refresh.elapsed() > Duration::from_secs(300) {
            info!("Refreshing token");
            self.update_refresh_token().await?;
        }

        let body = Self::add_context(body);
        let endpoint = Self::build_endpoint(path, ctoken);

        /* Switch to info for dev env */
        debug!("Requesting: {}", endpoint);

        // Retry loop with exponential backoff for rate limiting
        let mut retry_count = 0;
        loop {
            // For browser auth, generate a fresh authorization header with current timestamp
            let mut request = self.client.post(&endpoint).json(&body);
            
            if let YtMusicAuthType::Browser { sapisid, origin, .. } = &self.auth_type {
                let auth_header = Self::generate_sapisidhash(sapisid, origin);
                request = request.header("authorization", auth_header);
            }
            
            let res = request.send().await?;
            
            // For browser auth, capture and update cookies from response headers
            let response_headers = res.headers().clone();
            
            // Extract status and text
            let status = res.status();
            let text = res.text().await?;
            
            // Debug mode: save ALL responses
            if self.config.debug {
                std::fs::write(Self::RES_DEBUG_FILENAME, &text)?;
            }
            
            // Check for authentication errors
            self.check_authentication_errors(&text)?;
            
            // Handle rate limiting with retry
            match Self::handle_rate_limit_with_retry(status, &text, retry_count).await? {
                RateLimitAction::Retry(backoff_duration) => {
                    tokio::time::sleep(backoff_duration).await;
                    retry_count += 1;
                    continue;
                }
                RateLimitAction::MaxRetriesExceeded => {
                    let error_file = Self::save_http_error_diagnostic(status, &text)?;
                    return Err(eyre!(
                        "Rate limit exceeded after {} attempts. Please wait before retrying manually.\n\
                        Response saved to: {}",
                        Self::MAX_RETRIES + 1,
                        error_file
                    ));
                }
                RateLimitAction::Continue => {
                    // Not rate limited, continue with normal processing
                }
            }
            
            // Check for HTTP errors and save diagnostic data
            if status.is_client_error() || status.is_server_error() {
                let error_file = Self::save_http_error_diagnostic(status, &text)?;
                warn!("HTTP Error {} - Response saved to: {}", status, error_file);
                return Err(eyre!(
                    "HTTP Error {}: {}\n\
                    Diagnostic data saved to: {}\n\
                    Check this file for detailed error information.",
                    status, 
                    text.chars().take(200).collect::<String>(),
                    error_file
                ));
            }
            
            // Parse the JSON response
            let obj: T = serde_json::from_str(&text)?;
            
            // For browser auth, update cookies from response headers (after parsing JSON)
            if matches!(self.auth_type, YtMusicAuthType::Browser { .. }) {
                if let Err(e) = self.update_browser_cookies(&response_headers).await {
                    warn!("Failed to update browser cookies: {}", e);
                }
            }
            
            return Ok(obj);
        }
    }

    pub fn clean_playlist_id(id: &str) -> String {
        if let Some(id) = id.strip_prefix("VL") {
            return id.to_string();
        }
        id.to_string()
    }

    async fn update_browser_cookies(&mut self, response_headers: &HeaderMap) -> Result<()> {
        if let YtMusicAuthType::Browser { headers_path, sapisid: _, origin } = &self.auth_type {
            // Check if response has any Set-Cookie headers
            let set_cookies: Vec<String> = response_headers
                .get_all(reqwest::header::SET_COOKIE)
                .iter()
                .filter_map(|v| v.to_str().ok())
                .map(|s| s.to_string())
                .collect();
            
            if set_cookies.is_empty() {
                // No cookies to update
                return Ok(());
            }
            
            debug!("Updating {} cookies from response", set_cookies.len());
            
            // Read existing headers
            let header_data = std::fs::read_to_string(headers_path)?;
            let mut header_json: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&header_data)?;
            
            // Parse existing cookie string
            let existing_cookies = header_json
                .get("cookie")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            let mut cookie_map: HashMap<String, String> = HashMap::new();
            
            // Parse existing cookies into map
            for cookie_pair in existing_cookies.split("; ") {
                if let Some((key, value)) = cookie_pair.split_once('=') {
                    cookie_map.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
            
            // Track if SAPISID was updated
            let mut sapisid_updated = false;
            
            // Update with new cookies from response
            for set_cookie in set_cookies {
                // Parse Set-Cookie header (format: "name=value; Path=/; ...")
                if let Some((name_value, _)) = set_cookie.split_once(';') {
                    if let Some((name, value)) = name_value.split_once('=') {
                        let name = name.trim();
                        let value = value.trim();
                        
                        // Update or insert the cookie
                        if !value.is_empty() && value != "deleted" {
                            debug!("Updating cookie: {}", name);
                            cookie_map.insert(name.to_string(), value.to_string());
                            
                            if name == "__Secure-3PAPISID" {
                                sapisid_updated = true;
                            }
                        } else {
                            // Remove expired/deleted cookies
                            debug!("Removing expired cookie: {}", name);
                            cookie_map.remove(name);
                        }
                    }
                }
            }
            
            // Rebuild cookie string
            let updated_cookies: Vec<String> = cookie_map
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            let cookie_string = updated_cookies.join("; ");
            
            // Ensure SOCS=CAI is present in the cookie string
            let cookie_string = Self::ensure_socs_cookie(&cookie_string);
            
            // Update the JSON
            header_json.insert("cookie".to_string(), serde_json::Value::String(cookie_string.clone()));
            
            // Save updated headers back to file
            let updated_json = serde_json::to_string_pretty(&header_json)?;
            std::fs::write(headers_path, updated_json)?;
            
            // If SAPISID was updated, we need to update our auth_type
            if sapisid_updated {
                let new_sapisid = Self::extract_sapisid(&cookie_string)?;
                self.auth_type = YtMusicAuthType::Browser {
                    headers_path: headers_path.clone(),
                    sapisid: new_sapisid,
                    origin: origin.clone(),
                };
                info!("SAPISID cookie was refreshed");
            }
            
            // Rebuild client with updated headers
            let mut headers = HeaderMap::new();
            for (key, val) in header_json.into_iter() {
                if let serde_json::Value::String(s) = val {
                    // Skip authorization - we generate it dynamically
                    if key.to_lowercase() == "authorization" {
                        continue;
                    }
                    
                    // For cookie header, ensure SOCS=CAI is always present
                    if key.to_lowercase() == "cookie" {
                        let enhanced_cookies = Self::ensure_socs_cookie(&s);
                        headers.insert(
                            HeaderName::from_bytes(key.to_lowercase().as_bytes())?,
                            enhanced_cookies.parse()?,
                        );
                        continue;
                    }
                    
                    headers.insert(
                        HeaderName::from_bytes(key.to_lowercase().as_bytes())?,
                        s.parse()?,
                    );
                }
            }
            
            // Remove encoding headers that can cause issues
            headers.remove("accept-encoding");
            headers.remove("content-encoding");
            headers.remove("content-length");
            
            let mut client = reqwest::ClientBuilder::new()
                .cookie_store(true)
                .default_headers(headers);
            
            if let Some(proxy) = &self.config.proxy {
                client = client
                    .proxy(reqwest::Proxy::all(proxy)?)
                    .danger_accept_invalid_certs(true)
            }
            
            self.client = client.build()?;
            self.last_token_refresh = Instant::now();
            
            debug!("Browser cookies updated and saved to file");
        }
        
        Ok(())
    }

    async fn update_refresh_token(&mut self) -> Result<()> {
        match &self.auth_type {
            YtMusicAuthType::OAuth { client_id, client_secret, oauth_token_path } => {
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
            },
            YtMusicAuthType::Browser { .. } => {
                // Browser auth doesn't need token refresh
                // But we still update the timestamp
                self.last_token_refresh = Instant::now();
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl MusicApi for YtMusicApi {
    fn api_type(&self) -> MusicApiType {
        MusicApiType::YtMusic
    }

    fn country_code(&self) -> &'static str {
        // TODO: it seems impossible to get the country code from YtMusic
        "UNKNOWN"
    }

    async fn create_playlist(&mut self, name: &str, public: bool) -> Result<Playlist> {
        let privacy_status = if public { "PUBLIC" } else { "PRIVATE" };
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
        
        // Deduplicate playlists by ID to handle YouTube Music API returning duplicates
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
        let browse_id = if id.starts_with("VL") {
            id.to_string()
        } else {
            format!("VL{}", id)
        };

        let body = json!({ "browseId": browse_id });
        
        // Make request with error handling for empty/inaccessible playlists
        let response = match self.paginated_request("browse", &body).await {
            Ok(r) => r,
            Err(e) => {
                // Check if this is a parsing error due to missing contents
                if e.to_string().contains("missing field `contents`") {
                    warn!("Playlist {} appears to be empty, inaccessible, or deleted. Skipping.", id);
                    return Ok(vec![]);
                }
                return Err(e);
            }
        };
        
        let songs: Songs = match response.try_into() {
            Ok(s) => s,
            Err(e) => {
                // Handle parsing errors gracefully
                if e.to_string().contains("missing field") || e.to_string().contains("contents") {
                    warn!("Could not parse songs from playlist {}. It may be empty, private, or deleted. Error: {}", id, e);
                    return Ok(vec![]);
                }
                return Err(e);
            }
        };
        
        Ok(songs.0)
    }

    async fn add_songs_to_playlist(&mut self, playlist: &mut Playlist, songs: &[Song]) -> Result<()> {
        for song in songs {
            playlist.songs.push(song.clone());
        }

        let mut actions = vec![];
        for song in songs {
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
                                tokio::time::sleep(Duration::from_secs(3)).await;
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
        for song in songs {
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
                res_song.isrc = Some(isrc.clone());
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

    async fn add_likes(&mut self, songs: &[Song]) -> Result<()> {
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
