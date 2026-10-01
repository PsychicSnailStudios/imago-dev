use md5;
use reqwest::Client;
use rusqlite::Connection;
use serde::Deserialize;
use std::collections::BTreeMap;

use crate::db::settings_manager::{get_setting, set_setting};
use crate::db::track_manager::get_track_by_uid;

const API_URL: &str = "https://ws.audioscrobbler.com/2.0/";

pub fn lastfm_auth_url(api_key: &str) -> String {
    format!(
        "https://www.last.fm/api/auth/?api_key={}&cb=imago://lastfm-callback",
        api_key
    )
}

#[derive(Deserialize)]
struct SessionResponse {
    session: SessionBody,
}

#[derive(Deserialize)]
struct SessionBody {
    key: String,
}

pub async fn lastfm_exchange_token(profile_uid: &str, token: &str) -> Result<(), String> {
    let conn = crate::open_settings_conn(profile_uid);
    let api_key = get_setting(&conn, "api_lastfm_key")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    if api_key.is_empty() {
        return Err("Last.fm API key not configured".into());
    }

    let secret = get_setting(&conn, "api_lastfm_secret")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    let sig = sign_params(
        &[
            ("api_key", api_key.as_str()),
            ("method", "auth.getSession"),
            ("token", token),
        ],
        &secret,
    );

    let client = Client::new();
    let res = client
        .get(API_URL)
        .query(&[
            ("method", "auth.getSession"),
            ("api_key", &api_key),
            ("token", token),
            ("api_sig", &sig),
            ("format", "json"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: SessionResponse = res.json().await.map_err(|e| e.to_string())?;
    set_setting(&conn, "lastfm_session_key", &body.session.key).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn lastfm_disconnect(profile_uid: &str) -> Result<(), String> {
    let conn = crate::open_settings_conn(profile_uid);
    set_setting(&conn, "lastfm_session_key", "").map_err(|e| e.to_string())
}

pub fn lastfm_is_connected(conn: &Connection) -> bool {
    get_setting(conn, "lastfm_session_key")
        .ok()
        .flatten()
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

pub async fn update_now_playing(profile_uid: &str, track_uid: &str) -> Result<(), String> {
    let settings_conn = crate::open_settings_conn(profile_uid);
    if !lastfm_is_connected(&settings_conn) {
        return Ok(());
    }

    let (api_key, secret, session_key) = get_lastfm_credentials(&settings_conn)?;
    let lib_conn = crate::open_lib_conn(profile_uid);
    let track = get_track_by_uid(&lib_conn, track_uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Track not found: {}", track_uid))?;

    let title = track.title.unwrap_or_default();
    let artist = first_artist(track.artists.as_deref());
    let album = first_album_name(track.albums.as_deref());

    let mut params: Vec<(&str, String)> = vec![
        ("method", "track.updateNowPlaying".into()),
        ("track", title.clone()),
        ("artist", artist.clone()),
        ("api_key", api_key.clone()),
        ("sk", session_key.clone()),
    ];
    if let Some(ref al) = album {
        params.push(("album", al.clone()));
    }

    let sig = sign_params_owned(&params, &secret);
    params.push(("api_sig", sig));
    params.push(("format", "json".into()));

    let client = Client::new();
    client
        .post(API_URL)
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn scrobble_track(profile_uid: &str, track_uid: &str) -> Result<(), String> {
    let settings_conn = crate::open_settings_conn(profile_uid);
    if !lastfm_is_connected(&settings_conn) {
        return Ok(());
    }

    let (api_key, secret, session_key) = get_lastfm_credentials(&settings_conn)?;
    let lib_conn = crate::open_lib_conn(profile_uid);
    let track = get_track_by_uid(&lib_conn, track_uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Track not found: {}", track_uid))?;

    let title = track.title.unwrap_or_default();
    let artist = first_artist(track.artists.as_deref());
    let album = first_album_name(track.albums.as_deref());
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();

    let mut params: Vec<(&str, String)> = vec![
        ("method", "track.scrobble".into()),
        ("track[0]", title),
        ("artist[0]", artist),
        ("timestamp[0]", timestamp),
        ("api_key", api_key.clone()),
        ("sk", session_key.clone()),
    ];
    if let Some(ref al) = album {
        params.push(("album[0]", al.clone()));
    }

    let sig = sign_params_owned(&params, &secret);
    params.push(("api_sig", sig));
    params.push(("format", "json".into()));

    let client = Client::new();
    client
        .post(API_URL)
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn get_lastfm_credentials(conn: &Connection) -> Result<(String, String, String), String> {
    let api_key = get_setting(conn, "api_lastfm_key")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let secret = get_setting(conn, "api_lastfm_secret")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let session_key = get_setting(conn, "lastfm_session_key")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    Ok((api_key, secret, session_key))
}

fn sign_params(params: &[(&str, &str)], secret: &str) -> String {
    let mut sorted: BTreeMap<&str, &str> = BTreeMap::new();
    for (k, v) in params {
        sorted.insert(k, v);
    }
    let mut base = String::new();
    for (k, v) in &sorted {
        base.push_str(k);
        base.push_str(v);
    }
    base.push_str(secret);
    format!("{:x}", md5::compute(base.as_bytes()))
}

fn sign_params_owned(params: &[(&str, String)], secret: &str) -> String {
    let mut sorted: BTreeMap<&str, &str> = BTreeMap::new();
    for (k, v) in params {
        if *k == "format" || *k == "api_sig" {
            continue;
        }
        sorted.insert(k, v.as_str());
    }
    let mut base = String::new();
    for (k, v) in &sorted {
        base.push_str(k);
        base.push_str(v);
    }
    base.push_str(secret);
    format!("{:x}", md5::compute(base.as_bytes()))
}

fn first_artist(artists_json: Option<&str>) -> String {
    artists_json
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .and_then(|v| v.into_iter().next())
        .unwrap_or_default()
}

fn first_album_name(albums_json: Option<&str>) -> Option<String> {
    #[derive(Deserialize)]
    struct AlbumEntry {
        name: String,
    }
    albums_json
        .and_then(|s| serde_json::from_str::<Vec<AlbumEntry>>(s).ok())
        .and_then(|v| v.into_iter().next())
        .map(|e| e.name)
}
