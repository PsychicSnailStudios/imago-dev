use reqwest::Client;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::db::settings_manager::{get_setting, set_setting};
use crate::db::track_manager::{first_album_name, first_artist_name, get_track_by_uid};

const LB_API: &str = "https://api.listenbrainz.org/1";

pub fn listenbrainz_is_connected(conn: &Connection) -> bool {
    get_setting(conn, "listenbrainz_token")
        .ok()
        .flatten()
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

pub fn listenbrainz_connect(profile_uid: &str, token: String) -> Result<(), String> {
    let conn = crate::open_settings_conn(profile_uid);
    set_setting(&conn, "listenbrainz_token", &token).map_err(|e| e.to_string())
}

pub fn listenbrainz_disconnect(profile_uid: &str) -> Result<(), String> {
    let conn = crate::open_settings_conn(profile_uid);
    set_setting(&conn, "listenbrainz_token", "").map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct TrackMetadata {
    artist_name: String,
    track_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    release_name: Option<String>,
}

#[derive(Serialize)]
struct Listen {
    listened_at: i64,
    track_metadata: TrackMetadata,
}

#[derive(Serialize)]
struct SubmitListens {
    listen_type: String,
    payload: Vec<Listen>,
}

#[derive(Deserialize)]
struct ValidateResponse {
    code: Option<u16>,
}

pub async fn listenbrainz_validate_token(token: &str) -> Result<bool, String> {
    let client = Client::new();
    let res = client
        .get(format!("{}/validate-token", LB_API))
        .header("Authorization", format!("Token {}", token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(res.status().is_success())
}

pub async fn submit_listen(
    profile_uid: &str,
    track_uid: &str,
    timestamp: i64,
) -> Result<(), String> {
    let settings_conn = crate::open_settings_conn(profile_uid);
    let token = get_setting(&settings_conn, "listenbrainz_token")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    if token.is_empty() {
        return Ok(());
    }
    let settings_conn = crate::open_settings_conn(profile_uid);
    let token = get_setting(&settings_conn, "listenbrainz_token")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    if token.is_empty() {
        return Ok(());
    }

    let lib_conn = crate::open_lib_conn(profile_uid);
    let track = get_track_by_uid(&lib_conn, track_uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Track not found: {}", track_uid))?;

    let track_name = track.title.unwrap_or_default();
    let artist_name = first_artist_name(&track.artists).unwrap_or_default();
    let release_name = first_album_name(&track.albums);

    let body = SubmitListens {
        listen_type: "single".to_string(),
        payload: vec![Listen {
            listened_at: timestamp,
            track_metadata: TrackMetadata {
                artist_name,
                track_name,
                release_name,
            },
        }],
    };

    let client = Client::new();
    let res = client
        .post(format!("{}/submit-listens", LB_API))
        .header("Authorization", format!("Token {}", token))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body_text = res.text().await.unwrap_or_default();

    Ok(())
}

pub async fn update_now_playing(profile_uid: &str, track_uid: &str) -> Result<(), String> {
    let settings_conn = crate::open_settings_conn(profile_uid);
    let token = get_setting(&settings_conn, "listenbrainz_token")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    if token.is_empty() {
        return Ok(());
    }

    let lib_conn = crate::open_lib_conn(profile_uid);
    let track = get_track_by_uid(&lib_conn, track_uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Track not found: {}", track_uid))?;

    let track_name = track.title.unwrap_or_default();
    let artist_name = first_artist_name(&track.artists).unwrap_or_default();
    let release_name = first_album_name(&track.albums);

    let body = SubmitListens {
        listen_type: "playing_now".to_string(),
        payload: vec![Listen {
            listened_at: 0,
            track_metadata: TrackMetadata {
                artist_name,
                track_name,
                release_name,
            },
        }],
    };

    let client = Client::new();
    client
        .post(format!("{}/submit-listens", LB_API))
        .header("Authorization", format!("Token {}", token))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
