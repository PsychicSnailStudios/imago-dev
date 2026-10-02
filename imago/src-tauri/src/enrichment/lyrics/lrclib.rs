use super::LyricsResult;
use reqwest::Client;
use serde::Deserialize;

const LRCLIB_BASE: &str = "https://lrclib.net/api";

// ─────────────────────────────────────────────
// RESPONSE TYPES
// ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LrcLibResponse {
    #[serde(rename = "trackName")]
    track_name: Option<String>,
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    instrumental: Option<bool>,
}

// ─────────────────────────────────────────────
// SEARCH
// ─────────────────────────────────────────────

pub async fn search(
    client: &Client,
    title: &str,
    artist: &str,
    album: Option<&str>,
    duration_secs: Option<u64>,
) -> Option<LyricsResult> {
    // Try the exact-match /get endpoint first — requires duration for best results.
    if let Some(duration) = duration_secs {
        let url = format!("{}/get", LRCLIB_BASE);
        let mut params = vec![
            ("track_name", title.to_string()),
            ("artist_name", artist.to_string()),
            ("duration", duration.to_string()),
        ];
        if let Some(album) = album {
            params.push(("album_name", album.to_string()));
        }

        if let Some(result) = try_get(client, &url, &params).await {
            return Some(result);
        }
    }

    // Fall back to the search endpoint, which is fuzzier.
    let url = format!("{}/search", LRCLIB_BASE);
    let mut params = vec![
        ("track_name", title.to_string()),
        ("artist_name", artist.to_string()),
    ];
    if let Some(album) = album {
        params.push(("album_name", album.to_string()));
    }

    let resp = client.get(&url).query(&params).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let results: Vec<LrcLibResponse> = resp.json().await.ok()?;
    let best = results.into_iter().next()?;
    build_result(best)
}

// ─────────────────────────────────────────────
// HELPERS
// ─────────────────────────────────────────────

async fn try_get(client: &Client, url: &str, params: &[(&str, String)]) -> Option<LyricsResult> {
    let resp = client.get(url).query(params).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let data: LrcLibResponse = resp.json().await.ok()?;
    build_result(data)
}

fn build_result(data: LrcLibResponse) -> Option<LyricsResult> {
    let instrumental = data.instrumental.unwrap_or(false);

    // If the track is marked instrumental and has no lyrics text, return a
    // result indicating that rather than returning None (which would cause
    // unnecessary fallback attempts).
    if instrumental && data.plain_lyrics.is_none() && data.synced_lyrics.is_none() {
        return Some(LyricsResult {
            plain: None,
            synced: None,
            source: "lrclib".to_string(),
            instrumental: true,
        });
    }

    // If neither lyrics field has content, don't return a result.
    if data.plain_lyrics.is_none() && data.synced_lyrics.is_none() {
        return None;
    }

    Some(LyricsResult {
        plain: data.plain_lyrics,
        synced: data.synced_lyrics,
        source: "lrclib".to_string(),
        instrumental,
    })
}
