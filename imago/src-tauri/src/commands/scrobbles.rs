use crate::db::analytics_manager::{self, Scrobble, ScrobbleUpdate};
use crate::open_analytics_conn;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_scrobbles(state: State<'_, AppState>) -> Result<Vec<Scrobble>, String> {
    let uid = state.get_uid();
    crate::library_manager::run_blocking(move || {
        let conn = open_analytics_conn(&uid).map_err(|e| e.to_string())?;
        analytics_manager::get_all_scrobbles(&conn).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn get_scrobbles_since(
    state: State<'_, AppState>,
    since: i64,
) -> Result<Vec<Scrobble>, String> {
    let uid = state.get_uid();
    crate::library_manager::run_blocking(move || {
        let conn = open_analytics_conn(&uid).map_err(|e| e.to_string())?;
        analytics_manager::get_scrobbles_since(&conn, since).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn get_recent_scrobbles(
    state: State<'_, AppState>,
    limit: i64,
) -> Result<Vec<Scrobble>, String> {
    let uid = state.get_uid();
    crate::library_manager::run_blocking(move || {
        let conn = open_analytics_conn(&uid).map_err(|e| e.to_string())?;
        analytics_manager::get_recent_scrobbles(&conn, limit).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn get_scrobbles_for_artist(
    state: State<'_, AppState>,
    artist_uid: String,
    min_duration: i64,
) -> Result<Vec<Scrobble>, String> {
    let uid = state.get_uid();
    crate::library_manager::run_blocking(move || {
        let conn = open_analytics_conn(&uid).map_err(|e| e.to_string())?;
        analytics_manager::get_scrobbles_for_artist(&conn, &artist_uid, min_duration)
            .map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub fn get_scrobbles_for_track(
    state: State<AppState>,
    track_uid: String,
) -> Result<Vec<Scrobble>, String> {
    let uid = state.get_uid();
    let conn = open_analytics_conn(&uid).map_err(|e| e.to_string())?;
    analytics_manager::get_scrobbles_for_track(&conn, &track_uid).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn log_scrobble(
    state: State<AppState>,
    track_uid: String,
    artist_uid: String,
    reason_start: Option<String>,
    shuffle: Option<bool>,
    offline: Option<bool>,
    playing_local: Option<bool>,
    track_name: Option<String>,
    track_artist: Option<String>,
    track_album: Option<String>,
    album_uid: Option<String>,
    source_uid: Option<String>,
) -> Result<String, String> {
    let uid = state.get_uid();
    let conn = open_analytics_conn(&uid).map_err(|e| e.to_string())?;
    let scrobble_uid = analytics_manager::new_scrobble_uid();
    let scrobble = Scrobble {
        uid: scrobble_uid.clone(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
        track_uid,
        artist_uid,
        duration_played: 0,
        did_seek: false,
        did_pause: false,
        reason_start,
        reason_end: None,
        shuffle,
        skipped: None,
        offline,
        playing_local,
        track_name,
        track_artist,
        track_album,
        album_uid,
        source_uid,
    };
    analytics_manager::log_scrobble(&conn, &scrobble).map_err(|e| e.to_string())?;
    Ok(scrobble_uid)
}

#[tauri::command]
pub fn update_scrobble(
    state: State<AppState>,
    uid: String,
    duration_played: i64,
    did_seek: bool,
    did_pause: bool,
    reason_end: Option<String>,
    skipped: Option<bool>,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let conn = open_analytics_conn(&profile_uid).map_err(|e| e.to_string())?;
    let update = ScrobbleUpdate {
        duration_played,
        did_seek,
        did_pause,
        reason_end,
        skipped,
    };
    analytics_manager::update_scrobble(&conn, &uid, &update).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_scrobble(state: State<AppState>, uid: String) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let conn = open_analytics_conn(&profile_uid).map_err(|e| e.to_string())?;
    analytics_manager::delete_scrobble(&conn, &uid).map_err(|e| e.to_string())
}

pub fn relink_unmatched_scrobbles(profile_uid: &str) -> Result<usize, String> {
    use std::collections::HashMap;

    let analytics = open_analytics_conn(profile_uid).map_err(|e| e.to_string())?;

    type Row = (String, Option<String>, Option<String>);
    let rows: Vec<Row> = {
        let mut stmt = analytics
            .prepare(
                "SELECT uid, track_name, track_artist FROM scrobbles
                 WHERE ((track_uid IS NULL OR track_uid = '') AND track_name IS NOT NULL AND track_name != '')
                    OR ((artist_uid IS NULL OR artist_uid = '') AND track_artist IS NOT NULL AND track_artist != '')",
            )
            .map_err(|e| e.to_string())?;
        let collected: Vec<Row> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        collected
    };

    if rows.is_empty() {
        return Ok(0);
    }

    let merged = crate::open_merged_conn(profile_uid);
    let tracks = crate::db::get_all_tracks(&merged).map_err(|e| e.to_string())?;
    let artists = crate::db::get_all_artists(&merged).map_err(|e| e.to_string())?;
    drop(merged);

    struct Candidate {
        uid: String,
        album_artist_lower: Option<String>,
        artists_lower: Vec<String>,
        first_album_uid: Option<String>,
    }

    let mut track_index: HashMap<String, Vec<Candidate>> = HashMap::new();
    for t in &tracks {
        let Some(title) = t.title.as_deref() else { continue };
        let artists_lower: Vec<String> = t
            .artists
            .as_deref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .map(|v| v.iter().map(|a| a.to_lowercase()).collect())
            .unwrap_or_default();
        track_index
            .entry(title.to_lowercase())
            .or_default()
            .push(Candidate {
                uid: t.uid.clone(),
                album_artist_lower: t.album_artist.as_ref().map(|s| s.to_lowercase()),
                artists_lower,
                first_album_uid: crate::db::first_album_uid(&t.albums),
            });
    }

    let mut artist_index: HashMap<String, String> = HashMap::new();
    for a in &artists {
        artist_index
            .entry(a.name.to_lowercase())
            .or_insert_with(|| a.uid.clone());
    }

    let tx = analytics.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut updated = 0usize;

    for (uid, track_name, track_artist) in rows {
        let artist_lower = track_artist.as_deref().map(|s| s.to_lowercase());

        let matched = track_name.as_deref().and_then(|name| {
            track_index.get(&name.to_lowercase()).and_then(|candidates| {
                candidates.iter().find(|c| match artist_lower.as_deref() {
                    None => true,
                    Some(an) => {
                        c.album_artist_lower.as_deref() == Some(an)
                            || c.artists_lower.iter().any(|a| a == an)
                    }
                })
            })
        });

        let artist_uid = artist_lower
            .as_deref()
            .and_then(|an| artist_index.get(an).cloned())
            .unwrap_or_default();

        let track_uid = matched.map(|c| c.uid.clone()).unwrap_or_default();
        let album_uid = matched.and_then(|c| c.first_album_uid.clone());

        if track_uid.is_empty() && artist_uid.is_empty() {
            continue;
        }

        let changed = tx
            .execute(
                "UPDATE scrobbles SET
                    track_uid = CASE WHEN ?1 != '' THEN ?1 ELSE track_uid END,
                    artist_uid = CASE WHEN ?2 != '' AND (artist_uid IS NULL OR artist_uid = '') THEN ?2 ELSE artist_uid END,
                    album_uid = COALESCE(?3, album_uid)
                 WHERE uid = ?4",
                rusqlite::params![track_uid, artist_uid, album_uid, uid],
            )
            .map_err(|e| e.to_string())?;
        updated += changed;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(updated)
}

#[tauri::command]
pub async fn relink_scrobbles_cmd(state: State<'_, AppState>) -> Result<usize, String> {
    let uid = state.get_uid();
    crate::library_manager::run_blocking(move || relink_unmatched_scrobbles(&uid)).await
}
