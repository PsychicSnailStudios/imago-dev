pub mod lrclib;

// ─────────────────────────────────────────────
// SHARED TYPES
// ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LyricsResult {
    pub plain: Option<String>,
    pub synced: Option<String>,
    pub source: String,
    pub instrumental: bool,
}

// ─────────────────────────────────────────────
// FETCH
// ─────────────────────────────────────────────

/// Fetches lyrics for a track. lrclib is the only source currently.
/// Returns None if the track is instrumental or no lyrics are found.
pub async fn fetch_lyrics(
    client: &reqwest::Client,
    title: &str,
    artist: &str,
    album: Option<&str>,
    duration_secs: Option<u64>,
) -> Option<LyricsResult> {
    lrclib::search(client, title, artist, album, duration_secs).await
}
