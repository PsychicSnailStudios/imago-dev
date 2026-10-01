use reqwest::Client;
use serde::Deserialize;

use crate::db::settings_manager::get_setting;
use crate::enrichment::{EnrichedAlbum, EnrichedArtist, EnrichedMetadata};

const API_BASE: &str = "https://api.spotify.com/v1";

async fn get_valid_token(profile_uid: &str) -> Result<String, String> {
    crate::connections::spotify_auth::get_valid_token(profile_uid).await
}

pub fn is_connected(profile_uid: &str) -> bool {
    let conn = crate::open_settings_conn(profile_uid);
    crate::connections::spotify_auth::spotify_is_connected(&conn)
}
// ─────────────────────────────────────────────
// SHARED HELPERS
// ─────────────────────────────────────────────

#[derive(Deserialize)]
struct SpotifyImage {
    url: String,
    width: Option<u32>,
}

async fn download_image(client: &Client, url: &str) -> Option<Vec<u8>> {
    let res = client.get(url).send().await.ok()?;
    if res.status().is_success() {
        res.bytes().await.ok().map(|b| b.to_vec())
    } else {
        None
    }
}

fn best_image_url(images: Vec<SpotifyImage>) -> Option<String> {
    images
        .into_iter()
        .max_by_key(|i| i.width.unwrap_or(0))
        .map(|i| i.url)
}

// ─────────────────────────────────────────────
// TRACK ENRICHMENT
// ─────────────────────────────────────────────

pub async fn enrich_track(
    client: &Client,
    profile_uid: &str,
    title: &str,
    artist: &str,
) -> Option<EnrichedMetadata> {
    let token = get_valid_token(profile_uid).await.ok()?;

    #[derive(Deserialize)]
    struct SearchResult {
        tracks: TracksWrapper,
    }
    #[derive(Deserialize)]
    struct TracksWrapper {
        items: Vec<SpotifyTrackFull>,
    }
    #[derive(Deserialize)]
    struct SpotifyTrackFull {
        id: String,
        name: String,
        artists: Vec<SpotifyArtistRef>,
        album: SpotifyAlbumRef,
        duration_ms: i64,
        track_number: u32,
    }
    #[derive(Deserialize)]
    struct SpotifyArtistRef {
        name: String,
    }
    #[derive(Deserialize)]
    struct SpotifyAlbumRef {
        name: String,
        release_date: Option<String>,
        genres: Option<Vec<String>>,
    }

    let query = format!("track:{} artist:{}", title, artist);
    let res = client
        .get(format!("{}/search", API_BASE))
        .bearer_auth(&token)
        .query(&[
            ("q", &query),
            ("type", &"track".to_string()),
            ("limit", &"1".to_string()),
        ])
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        return None;
    }

    let body: SearchResult = res.json().await.ok()?;
    let item = body.tracks.items.into_iter().next()?;

    let artist_names: Vec<String> = item.artists.iter().map(|a| a.name.clone()).collect();
    let album_artist = artist_names.first().cloned();
    let artists_json = serde_json::to_string(&artist_names).ok();

    let year = item
        .album
        .release_date
        .as_deref()
        .and_then(|d| d.get(..4))
        .map(|s| s.to_string());

    let album_json = serde_json::to_string(&[serde_json::json!({
        "uid": "",
        "name": item.album.name,
        "track_number": item.track_number
    })])
    .ok();

    let genres = item
        .album
        .genres
        .filter(|g| !g.is_empty())
        .map(|g| serde_json::to_string(&g).unwrap_or_default());

    let audio_features = fetch_audio_features(client, &token, &item.id).await;

    Some(EnrichedMetadata {
        title: Some(item.name),
        artists: artists_json,
        album_artist,
        albums: album_json,
        year,
        genres,
        bpm: audio_features.as_ref().and_then(|f| f.tempo),
        key: audio_features.as_ref().and_then(|f| f.musical_key.clone()),
        artwork: None,
        label: None,
        format: None,
        mbid: None,
    })
}

struct AudioFeatures {
    tempo: Option<f32>,
    musical_key: Option<String>,
}

async fn fetch_audio_features(
    client: &Client,
    token: &str,
    track_id: &str,
) -> Option<AudioFeatures> {
    #[derive(Deserialize)]
    struct SpotifyAudioFeatures {
        tempo: Option<f32>,
        key: Option<i32>,
        mode: Option<i32>,
    }

    let res = client
        .get(format!("{}/audio-features/{}", API_BASE, track_id))
        .bearer_auth(token)
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        return None;
    }

    let f: SpotifyAudioFeatures = res.json().await.ok()?;

    let musical_key = match (f.key, f.mode) {
        (Some(k), Some(m)) if k >= 0 => {
            let notes = [
                "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
            ];
            let note = notes.get(k as usize)?;
            let mode = if m == 1 { "" } else { "m" };
            Some(format!("{}{}", note, mode))
        }
        _ => None,
    };

    Some(AudioFeatures {
        tempo: f.tempo,
        musical_key,
    })
}

// ─────────────────────────────────────────────
// ALBUM ENRICHMENT
// ─────────────────────────────────────────────

pub async fn enrich_album(
    client: &Client,
    profile_uid: &str,
    title: &str,
    artist: &str,
) -> Option<EnrichedAlbum> {
    let token = get_valid_token(profile_uid).await.ok()?;

    #[derive(Deserialize)]
    struct SearchResult {
        albums: AlbumsWrapper,
    }
    #[derive(Deserialize)]
    struct AlbumsWrapper {
        items: Vec<SpotifyAlbumResult>,
    }
    #[derive(Deserialize)]
    struct SpotifyAlbumResult {
        id: String,
        release_date: Option<String>,
        images: Vec<SpotifyImage>,
    }
    #[derive(Deserialize)]
    struct SpotifyAlbumFull {
        genres: Option<Vec<String>>,
        label: Option<String>,
    }

    let query = format!("album:{} artist:{}", title, artist);
    let res = client
        .get(format!("{}/search", API_BASE))
        .bearer_auth(&token)
        .query(&[
            ("q", &query),
            ("type", &"album".to_string()),
            ("limit", &"1".to_string()),
        ])
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        return None;
    }

    let body: SearchResult = res.json().await.ok()?;
    let item = body.albums.items.into_iter().next()?;

    let artwork_url = best_image_url(item.images);
    let artwork = match artwork_url {
        Some(ref url) => download_image(client, url).await,
        None => None,
    };

    let full_res = client
        .get(format!("{}/albums/{}", API_BASE, item.id))
        .bearer_auth(&token)
        .send()
        .await
        .ok()?;

    let (genres, label) = if full_res.status().is_success() {
        let full: SpotifyAlbumFull = full_res.json().await.ok().unwrap_or(SpotifyAlbumFull {
            genres: None,
            label: None,
        });
        let genres_json = full
            .genres
            .filter(|g| !g.is_empty())
            .map(|g| serde_json::to_string(&g).unwrap_or_default());
        (genres_json, full.label)
    } else {
        (None, None)
    };

    Some(EnrichedAlbum {
        release_date: item.release_date,
        genres,
        label,
        format: None,
        description: None,
        artwork,
        mbid: None,
    })
}

// ─────────────────────────────────────────────
// ARTIST ENRICHMENT
// ─────────────────────────────────────────────

pub async fn enrich_artist(
    client: &Client,
    profile_uid: &str,
    name: &str,
) -> Option<EnrichedArtist> {
    let token = get_valid_token(profile_uid).await.ok()?;

    #[derive(Deserialize)]
    struct SearchResult {
        artists: ArtistsWrapper,
    }
    #[derive(Deserialize)]
    struct ArtistsWrapper {
        items: Vec<SpotifyArtistResult>,
    }
    #[derive(Deserialize)]
    struct SpotifyArtistResult {
        genres: Option<Vec<String>>,
        images: Vec<SpotifyImage>,
    }

    let res = client
        .get(format!("{}/search", API_BASE))
        .bearer_auth(&token)
        .query(&[("q", name), ("type", &"artist"), ("limit", &"1")])
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        return None;
    }

    let body: SearchResult = res.json().await.ok()?;
    let item = body.artists.items.into_iter().next()?;

    let image_url = best_image_url(item.images);
    let profile_art = match image_url {
        Some(ref url) => download_image(client, url).await,
        None => None,
    };

    let genres_json = item
        .genres
        .filter(|g| !g.is_empty())
        .map(|g| serde_json::to_string(&g).unwrap_or_default());

    Some(EnrichedArtist {
        about: None,
        genres: genres_json,
        websites: None,
        profile_art,
        banner_art: None,
    })
}
