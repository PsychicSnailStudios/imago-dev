use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use reqwest::Client;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::generate_uid;
use crate::db::playlist_manager::create_playlist;
use crate::db::settings_manager::{get_setting, set_setting};
use crate::db::track_manager::{get_all_tracks, upsert_track};
use crate::db::{Playlist, PlaylistTrackEntry, Track};

const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const API_BASE: &str = "https://api.spotify.com/v1";
const REDIRECT_URI: &str = "sonodex://spotify-callback";
const SCOPES: &str = "playlist-read-private playlist-read-collaborative user-library-read playlist-modify-public playlist-modify-private";

pub fn generate_code_verifier() -> String {
    let mut bytes = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn generate_code_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

pub fn spotify_auth_url(client_id: &str) -> (String, String) {
    let verifier = generate_code_verifier();
    let challenge = generate_code_challenge(&verifier);

    let url = format!(
        "https://accounts.spotify.com/authorize\
		 ?client_id={}\
		 &response_type=code\
		 &redirect_uri={}\
		 &scope={}\
		 &code_challenge_method=S256\
		 &code_challenge={}",
        client_id,
        urlencoding::encode(REDIRECT_URI),
        urlencoding::encode(SCOPES),
        challenge,
    );

    (url, verifier)
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

pub async fn spotify_exchange_code(
    profile_uid: &str,
    code: &str,
    verifier: &str,
) -> Result<(), String> {
    let conn = crate::open_settings_conn(profile_uid);
    let client_id = get_setting(&conn, "spotify_client_id")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    if client_id.is_empty() {
        return Err("Spotify client ID not configured".into());
    }

    let client = Client::new();
    let res = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", &client_id),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let text = res.text().await.unwrap_or_default();
        return Err(format!("Spotify token exchange failed: {}", text));
    }

    let body: TokenResponse = res.json().await.map_err(|e| e.to_string())?;
    store_tokens(&conn, &body)?;
    Ok(())
}

pub async fn spotify_refresh_token(profile_uid: &str) -> Result<String, String> {
    let conn = crate::open_settings_conn(profile_uid);
    let client_id = get_setting(&conn, "spotify_client_id")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let refresh_token = get_setting(&conn, "spotify_refresh_token")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    if refresh_token.is_empty() {
        return Err("No Spotify refresh token stored".into());
    }

    let client = Client::new();
    let res = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
            ("client_id", &client_id),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let text = res.text().await.unwrap_or_default();
        return Err(format!("Spotify token refresh failed: {}", text));
    }

    let body: TokenResponse = res.json().await.map_err(|e| e.to_string())?;
    let access_token = body.access_token.clone();
    store_tokens(&conn, &body)?;
    Ok(access_token)
}

pub fn spotify_disconnect(profile_uid: &str) -> Result<(), String> {
    let conn = crate::open_settings_conn(profile_uid);
    set_setting(&conn, "spotify_access_token", "").map_err(|e| e.to_string())?;
    set_setting(&conn, "spotify_refresh_token", "").map_err(|e| e.to_string())?;
    set_setting(&conn, "spotify_token_expiry", "0").map_err(|e| e.to_string())?;
    Ok(())
}

pub fn spotify_is_connected(conn: &Connection) -> bool {
    let token = get_setting(conn, "spotify_access_token")
        .ok()
        .flatten()
        .unwrap_or_default();
    if token.is_empty() {
        return false;
    }
    let expiry: u64 = get_setting(conn, "spotify_token_expiry")
        .ok()
        .flatten()
        .unwrap_or_default()
        .parse()
        .unwrap_or(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    expiry > now + 60
}

fn store_tokens(conn: &Connection, body: &TokenResponse) -> Result<(), String> {
    let expiry = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + body.expires_in;

    set_setting(conn, "spotify_access_token", &body.access_token).map_err(|e| e.to_string())?;
    if let Some(ref rt) = body.refresh_token {
        set_setting(conn, "spotify_refresh_token", rt).map_err(|e| e.to_string())?;
    }
    set_setting(conn, "spotify_token_expiry", &expiry.to_string()).map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_valid_token(profile_uid: &str) -> Result<String, String> {
    let conn = crate::open_settings_conn(profile_uid);
    if spotify_is_connected(&conn) {
        return Ok(get_setting(&conn, "spotify_access_token")
            .map_err(|e| e.to_string())?
            .unwrap_or_default());
    }
    spotify_refresh_token(profile_uid).await
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpotifyPlaylistSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub track_count: u32,
    pub owner: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpotifyPlaylistInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner: String,
    pub track_count: u32,
    pub image_url: Option<String>,
}

#[derive(Deserialize)]
struct SpotifyPlaylistsPage {
    items: Vec<Option<SpotifyPlaylistItem>>,
    next: Option<String>,
}

#[derive(Deserialize)]
struct SpotifyPlaylistItem {
    id: String,
    name: String,
    description: Option<String>,
    #[serde(default)]
    tracks: Option<TrackCount>,
    owner: Owner,
    #[serde(default)]
    images: Option<Vec<SpotifyImage>>,
}

#[derive(Deserialize)]
struct TrackCount {
    #[serde(default)]
    total: u32,
}

#[derive(Deserialize)]
struct Owner {
    display_name: Option<String>,
}

#[derive(Deserialize, Clone)]
struct SpotifyImage {
    url: String,
}

pub async fn spotify_get_playlists(
    profile_uid: &str,
) -> Result<Vec<SpotifyPlaylistSummary>, String> {
    let token = get_valid_token(profile_uid).await?;
    let client = Client::new();
    let mut results = Vec::new();
    let mut url = format!("{}/me/playlists?limit=50", API_BASE);

    loop {
        let res = client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(format!("Spotify playlists error: {}", text));
        }

        let text = res.text().await.unwrap_or_default();
        // eprintln!("[spotify] playlists response: {}", &text[..500.min(text.len())]);
        let page: SpotifyPlaylistsPage = serde_json::from_str(&text).map_err(|e| e.to_string())?;

        for item in page.items.into_iter().flatten() {
            let image_url = item
                .images
                .as_ref()
                .and_then(|imgs| imgs.first())
                .map(|img| img.url.clone());
            results.push(SpotifyPlaylistSummary {
                id: item.id,
                name: item.name,
                description: item.description,
                track_count: item.tracks.map(|t| t.total).unwrap_or(0),
                owner: item.owner.display_name.unwrap_or_default(),
                image_url,
            });
        }

        match page.next {
            Some(next_url) => url = next_url,
            None => break,
        }
    }

    Ok(results)
}

pub async fn spotify_get_playlist_info(
    profile_uid: &str,
    playlist_id: &str,
) -> Result<SpotifyPlaylistInfo, String> {
    let token = get_valid_token(profile_uid).await?;
    let client = Client::new();

    let url = format!("{}/playlists/{}", API_BASE, playlist_id);
    let res = client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!(
            "Spotify playlist info error: status={} body={}",
            status, text
        ));
    }

    #[derive(Deserialize)]
    struct PlaylistInfoResponse {
        id: String,
        name: String,
        description: Option<String>,
        owner: Owner,
        images: Option<Vec<SpotifyImage>>,
        #[serde(default)]
        items: Option<ItemsCount>,
    }

    #[derive(Deserialize)]
    struct ItemsCount {
        #[serde(default)]
        total: u32,
    }

    let info: PlaylistInfoResponse = res.json().await.map_err(|e| e.to_string())?;
    let image_url = info
        .images
        .as_ref()
        .and_then(|imgs| imgs.first())
        .map(|img| img.url.clone());

    Ok(SpotifyPlaylistInfo {
        id: info.id,
        name: info.name,
        description: info.description,
        owner: info.owner.display_name.unwrap_or_default(),
        track_count: info.items.map(|i| i.total).unwrap_or(0),
        image_url,
    })
}

async fn download_image(client: &Client, url: &str) -> Option<Vec<u8>> {
    let res = client.get(url).send().await.ok()?;
    if res.status().is_success() {
        res.bytes().await.ok().map(|b| b.to_vec())
    } else {
        None
    }
}

#[derive(Deserialize)]
struct SpotifyPlaylistFull {
    #[serde(default)]
    items: Option<SpotifyItemsPage>,
    description: Option<String>,
    images: Option<Vec<SpotifyImage>>,
}

#[derive(Deserialize, Default)]
struct SpotifyItemsPage {
    #[serde(default)]
    items: Vec<SpotifyPlaylistEntry>,
    next: Option<String>,
}

#[derive(Deserialize)]
struct SpotifyPlaylistEntry {
    item: Option<SpotifyTrack>,
}

#[derive(Deserialize)]
struct SpotifyTrack {
    id: String,
    name: String,
    #[serde(default)]
    duration_ms: u64,
    #[serde(default)]
    track_number: u32,
    #[serde(default)]
    artists: Vec<SpotifyArtist>,
    album: Option<SpotifyAlbumRef>,
}

#[derive(Deserialize)]
struct SpotifyArtist {
    name: String,
}

#[derive(Deserialize)]
struct SpotifyAlbumRef {
    name: String,
    release_date: Option<String>,
    images: Option<Vec<SpotifyImage>>,
}

pub async fn spotify_import_playlist(
    profile_uid: &str,
    spotify_playlist_id: &str,
    playlist_name: &str,
    owner: Option<String>,
) -> Result<String, String> {
    let token = get_valid_token(profile_uid).await?;
    let client = Client::new();
    let lib_conn = crate::open_lib_conn(profile_uid);

    let mut spotify_tracks: Vec<SpotifyTrack> = Vec::new();

    let initial_url = format!("{}/playlists/{}", API_BASE, spotify_playlist_id);
    let res = client
        .get(&initial_url)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!(
            "Spotify playlist error: status={} body={}",
            status, text
        ));
    }

    let text = res.text().await.map_err(|e| e.to_string())?;
    let full: SpotifyPlaylistFull = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let description = full.description.clone();
    let artwork_blob = if let Some(ref images) = full.images {
        if let Some(img) = images.first() {
            download_image(&client, &img.url).await
        } else {
            None
        }
    } else {
        None
    };

    let first_page = full.items.unwrap_or_default();

    for entry in first_page.items {
        if let Some(track) = entry.item {
            spotify_tracks.push(track);
        }
    }

    let mut next = first_page.next;

    while let Some(url) = next {
        let res = client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(format!(
                "Spotify tracks error: status={} url={} body={}",
                status, url, text
            ));
        }

        let page: SpotifyItemsPage = res.json().await.map_err(|e| e.to_string())?;

        for entry in page.items {
            if let Some(track) = entry.item {
                spotify_tracks.push(track);
            }
        }

        next = page.next;
    }

    let existing_tracks = get_all_tracks(&lib_conn).map_err(|e| e.to_string())?;
    let mut playlist_entries: Vec<PlaylistTrackEntry> = Vec::new();

    for (order, sp_track) in spotify_tracks.iter().enumerate() {
        let sp_title = sp_track.name.to_lowercase();
        let sp_artist = sp_track
            .artists
            .first()
            .map(|a| a.name.to_lowercase())
            .unwrap_or_default();

        let matched = existing_tracks.iter().find(|t| {
            let title_match = t
                .title
                .as_deref()
                .map(|s| s.to_lowercase() == sp_title)
                .unwrap_or(false);

            let artist_match = t
                .artists
                .as_deref()
                .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                .map(|v| v.iter().any(|a| a.to_lowercase() == sp_artist))
                .unwrap_or(false);

            title_match && artist_match
        });

        let uid = if let Some(existing) = matched {
            existing.uid.clone()
        } else {
            let stub_uid = generate_uid("t-");

            let year = sp_track
                .album
                .as_ref()
                .and_then(|a| a.release_date.as_deref())
                .and_then(|d| d.get(..4))
                .map(|s| s.to_string());

            let artwork_blob = if let Some(ref album) = sp_track.album {
                if let Some(ref images) = album.images {
                    if let Some(img) = images.first() {
                        download_image(&client, &img.url).await
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let stub = Track {
                id: None,
                uid: stub_uid.clone(),
                path: String::new(),
                last_modified: 0,
                title: Some(sp_track.name.clone()),
                artists: serde_json::to_string(
                    &sp_track
                        .artists
                        .iter()
                        .map(|a| a.name.clone())
                        .collect::<Vec<String>>(),
                )
                .ok(),
                album_artist: sp_track.artists.first().map(|a| a.name.clone()),
                albums: serde_json::to_string(&serde_json::json!([{
                    "uid": "",
                    "name": sp_track.album.as_ref().map(|a| a.name.clone()).unwrap_or_default(),
                    "track_number": sp_track.track_number,
                    "disc": 0,
                }]))
                .ok(),
                tags: serde_json::to_string(&Vec::<String>::new()).ok(),
                genres: None,
                year,
                rating: None,
                duration_ms: Some(sp_track.duration_ms as i64),
                bpm: None,
                key: None,
                credits: None,
                label: None,
                artwork_blob: artwork_blob,
                artwork_path: None,
                user_options: None,
                format: None,
                bitrate: None,
                remote_path: None,
                remote_data: None,
                track_data: None,
                artwork_thumb: None,
                date_added: None,
            };

            match upsert_track(&lib_conn, &stub) {
                Ok(_) => {
                    crate::scanner::process_track(&lib_conn, &stub);
                }
                Err(e) => {
                    eprintln!("[spotify] upsert_track failed for {}: {}", sp_track.name, e);
                }
            }

            stub_uid
        };

        playlist_entries.push(PlaylistTrackEntry {
            uid,
            name: sp_track.name.clone(),
            order: (order + 1) as u32,
        });
    }

    let playlist_uid = generate_uid("p-");
    let tracks_json = serde_json::to_string(&playlist_entries).map_err(|e| e.to_string())?;

    let playlist = Playlist {
        id: None,
        uid: playlist_uid.clone(),
        title: playlist_name.to_string(),
        description,
        owner,
        tracks: Some(tracks_json),
        artwork_blob,
        artwork_path: None,
        folder: None,
        version: 1,
        versions_data: None,
        link_url: Some(format!(
            "https://open.spotify.com/playlist/{}",
            spotify_playlist_id
        )),
        emulate_type: None,
        emulate_settings: None,
        pending_tracks: None,
        share_settings: None,
        artwork_thumb: None,
    };

    create_playlist(&lib_conn, &playlist).map_err(|e| e.to_string())?;
    Ok(playlist_uid)
}
