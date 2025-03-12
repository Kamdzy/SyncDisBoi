use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename = "MyPlex")]
#[allow(dead_code)]
pub struct PlexUserResponse {
    #[serde(rename = "authToken")]
    pub auth_token: String,

    #[serde(rename = "username")]
    pub username: String,

    #[serde(rename = "mappingState")]
    pub mapping_state: String
}

#[derive(Debug, Deserialize)]
#[serde(rename = "MediaContainer")]
#[allow(dead_code)]
pub struct PlexUriResponse {
    #[serde(rename = "size")]
    pub size: u32,

    #[serde(rename = "machineIdentifier")]
    pub machine_identifier: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "MediaContainer")]
#[allow(dead_code)]
pub struct PlexPlaylistsResponse {
    #[serde(rename = "size", default)]
    pub size: Option<u32>,

    #[serde(rename = "Playlist", default)]
    pub playlists: Vec<PlexPlaylist>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename = "Playlist")]
#[allow(dead_code)]
pub struct PlexPlaylist {
    #[serde(rename = "ratingKey", default)]
    pub rating_key: String,

    #[serde(rename = "key", default)]
    pub key: String,

    #[serde(rename = "guid", default)]
    pub guid: String,

    #[serde(rename = "type", default)]
    pub playlist_type: String,

    #[serde(rename = "title", default)]
    pub title: String,

    #[serde(rename = "titleSort", default)]
    pub title_sort: String,

    #[serde(rename = "summary", default)]
    pub summary: String,

    #[serde(rename = "smart", default)]
    pub smart: String,

    #[serde(rename = "playlistType", default)]
    pub playlist_subtype: String,

    #[serde(rename = "composite", default)]
    pub composite: String,

    #[serde(rename = "icon", default)]
    pub icon: String,

    #[serde(rename = "viewCount", default)]
    pub view_count: String,

    #[serde(rename = "lastViewedAt", default)]
    pub last_viewed_at: String,

    #[serde(rename = "thumb", default)]
    pub thumb: String,

    #[serde(rename = "duration", default)]
    pub duration: String,

    #[serde(rename = "leafCount", default)]
    pub leaf_count: String,

    #[serde(rename = "addedAt", default)]
    pub added_at: String,

    #[serde(rename = "updatedAt", default)]
    pub updated_at: String,

    #[serde(rename = "Image", default)]
    pub images: Option<Vec<Image>>,

    #[serde(rename = "UltraBlurColors")]
    pub ultra_blur_colors: Option<UltraBlurColors>
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Image {
    #[serde(rename = "alt", default)]
    pub alt: String,

    #[serde(rename = "type", default)]
    pub image_type: String,

    #[serde(rename = "url", default)]
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct UltraBlurColors {
    #[serde(rename = "topLeft", default)]
    pub top_left: String,

    #[serde(rename = "topRight", default)]
    pub top_right: String,

    #[serde(rename = "bottomRight", default)]
    pub bottom_right: String,

    #[serde(rename = "bottomLeft", default)]
    pub bottom_left: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "MediaContainer")]
#[allow(dead_code)]
pub struct PlexPlaylistSongsResponse {
    #[serde(rename = "size", default)]
    pub size: u32,

    #[serde(rename = "composite", default)]
    pub composite: String,

    #[serde(rename = "duration", default)]
    pub duration: u32,

    #[serde(rename = "leafCount", default)]
    pub leaf_count: u32,

    #[serde(rename = "playlistType", default)]
    pub playlist_type: String,

    #[serde(rename = "ratingKey", default)]
    pub rating_key: String,

    #[serde(rename = "smart", default)]
    pub smart: u32,

    #[serde(rename = "title", default)]
    pub title: String,

    #[serde(rename = "Track")]
    pub tracks: Option<Vec<Track>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename = "Track")]
#[allow(dead_code)]
pub struct Track {
    #[serde(rename = "ratingKey", default)]
    pub rating_key: String,

    #[serde(rename = "key", default)]
    pub key: String,

    #[serde(rename = "parentRatingKey", default)]
    pub parent_rating_key: String,

    #[serde(rename = "grandparentRatingKey", default)]
    pub grandparent_rating_key: String,

    #[serde(rename = "guid", default)]
    pub guid: String,

    #[serde(rename = "parentGuid", default)]
    pub parent_guid: String,

    #[serde(rename = "grandparentGuid", default)]
    pub grandparent_guid: String,

    #[serde(rename = "parentStudio", default)]
    pub parent_studio: String,

    #[serde(rename = "type", default)]
    pub track_type: String,

    #[serde(rename = "title", default)]
    pub title: String,

    #[serde(rename = "titleSort", default)]
    pub title_sort: String,

    #[serde(rename = "grandparentKey", default)]
    pub grandparent_key: String,

    #[serde(rename = "parentKey", default)]
    pub parent_key: String,

    #[serde(rename = "librarySectionTitle", default)]
    pub library_section_title: String,

    #[serde(rename = "librarySectionID", default)]
    pub library_section_id: u32,

    #[serde(rename = "librarySectionKey", default)]
    pub library_section_key: String,

    #[serde(rename = "grandparentTitle", default)]
    pub grandparent_title: String,

    #[serde(rename = "grandparentType", default)]
    pub grandparent_type: String,

    #[serde(rename = "parentTitle", default)]
    pub parent_title: String,

    #[serde(rename = "parentType", default)]
    pub parent_type: String,

    #[serde(rename = "summary", default)]
    pub summary: String,

    #[serde(rename = "index", default)]
    pub index: u32,

    #[serde(rename = "parentIndex", default)]
    pub parent_index: u32,

    #[serde(rename = "ratingCount", default)]
    pub rating_count: u32,

    #[serde(rename = "parentYear", default)]
    pub parent_year: u32,

    #[serde(rename = "thumb", default)]
    pub thumb: String,

    #[serde(rename = "art", default)]
    pub art: String,

    #[serde(rename = "parentThumb", default)]
    pub parent_thumb: String,

    #[serde(rename = "grandparentThumb", default)]
    pub grandparent_thumb: String,

    #[serde(rename = "grandparentArt", default)]
    pub grandparent_art: String,

    #[serde(rename = "playlistItemID", default)]
    pub playlist_item_id: u32,

    #[serde(rename = "duration", default)]
    pub duration: u32,

    #[serde(rename = "addedAt", default)]
    pub added_at: u32,

    #[serde(rename = "updatedAt", default)]
    pub updated_at: u32,

    #[serde(rename = "musicAnalysisVersion", default)]
    pub music_analysis_version: u32,

    #[serde(rename = "Media")]
    pub media: Vec<Media>,

    #[serde(rename = "Image")]
    pub images: Option<Vec<Image>>,

    #[serde(rename = "Genre")]
    pub genres: Option<Vec<Genre>>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Media {
    #[serde(rename = "id", default)]
    pub id: u32,

    #[serde(rename = "duration", default)]
    pub duration: u32,

    #[serde(rename = "bitrate", default)]
    pub bitrate: u32,

    #[serde(rename = "audioChannels", default)]
    pub audio_channels: u32,

    #[serde(rename = "audioCodec", default)]
    pub audio_codec: String,

    #[serde(rename = "container", default)]
    pub container: String,

    #[serde(rename = "hasVoiceActivity", default)]
    pub has_voice_activity: u32,

    #[serde(rename = "Part")]
    pub parts: Vec<Part>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Part {
    #[serde(rename = "id", default)]
    pub id: u32,

    #[serde(rename = "key", default)]
    pub key: String,

    #[serde(rename = "duration", default)]
    pub duration: u32,

    #[serde(rename = "file", default)]
    pub file: String,

    #[serde(rename = "size", default)]
    pub size: u32,

    #[serde(rename = "container", default)]
    pub container: String,

    #[serde(rename = "hasThumbnail", default)]
    pub has_thumbnail: u32,
}
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Genre {
    #[serde(rename = "tag", default)]
    pub tag: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "MediaContainer")]
#[allow(dead_code)]
pub struct PlexSearchTrackResponse {
    #[serde(rename = "size", default)]
    pub size: Option<u32>,

    #[serde(rename = "Track", default)]
    pub tracks: Option<Vec<Track>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "MediaContainer")]
#[allow(dead_code)]
pub struct PlexLibrarySectionsResponse {
    #[serde(rename = "size", default)]
    pub size: Option<u32>,

    #[serde(rename = "allowSync", default)]
    pub allow_sync: Option<u32>,

    #[serde(rename = "title1", default)]
    pub title1: Option<String>,

    #[serde(rename = "Directory", default)]
    pub directories: Option<Vec<Directory>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename = "Directory")]
#[allow(dead_code)]
pub struct Directory {
    #[serde(rename = "allowSync", default)]
    pub allow_sync: Option<u32>,

    #[serde(rename = "art", default)]
    pub art: Option<String>,

    #[serde(rename = "composite", default)]
    pub composite: Option<String>,

    #[serde(rename = "filters", default)]
    pub filters: Option<u32>,

    #[serde(rename = "refreshing", default)]
    pub refreshing: Option<u32>,

    #[serde(rename = "thumb", default)]
    pub thumb: Option<String>,

    #[serde(rename = "key", default)]
    pub key: Option<String>,

    #[serde(rename = "type", default)]
    pub directory_type: Option<String>,

    #[serde(rename = "title", default)]
    pub title: Option<String>,

    #[serde(rename = "agent", default)]
    pub agent: Option<String>,

    #[serde(rename = "scanner", default)]
    pub scanner: Option<String>,

    #[serde(rename = "language", default)]
    pub language: Option<String>,

    #[serde(rename = "uuid", default)]
    pub uuid: Option<String>,

    #[serde(rename = "updatedAt", default)]
    pub updated_at: Option<u64>,

    #[serde(rename = "createdAt", default)]
    pub created_at: Option<u64>,

    #[serde(rename = "scannedAt", default)]
    pub scanned_at: Option<u64>,

    #[serde(rename = "content", default)]
    pub content: Option<u32>,

    #[serde(rename = "directory", default)]
    pub directory: Option<u32>,

    #[serde(rename = "contentChangedAt", default)]
    pub content_changed_at: Option<u64>,

    #[serde(rename = "hidden", default)]
    pub hidden: Option<u32>,

    #[serde(rename = "Location", default)]
    pub locations: Option<Vec<Location>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename = "Location")]
#[allow(dead_code)]
pub struct Location {
    #[serde(rename = "id", default)]
    pub id: Option<u32>,

    #[serde(rename = "path", default)]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "MediaContainer")]
#[allow(dead_code)]
pub struct PlexCreatePlaylistResponse {
    #[serde(rename = "size", default)]
    pub size: Option<u32>,

    #[serde(rename = "Playlist", default)]
    pub playlists: Vec<PlexPlaylist>,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "MediaContainer")]
#[allow(dead_code)]
pub struct PlexHubSearchResponse {
    #[serde(rename = "size", default)]
    pub size: u32,

    #[serde(rename = "SearchResult", default)]
    pub search_results: Vec<SearchResult>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SearchResult {
    #[serde(rename = "score", default)]
    pub score: f32,

    #[serde(rename = "Directory", default)]
    pub directories: Option<Vec<Directory>>,

    #[serde(rename = "Track", default)]
    pub tracks: Option<Vec<Track>>,
}