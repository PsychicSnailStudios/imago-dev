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

struct RelinkCandidate {
    uid: String,
    album_artist_lower: Option<String>,
    artists_lower: Vec<String>,
    albums: Vec<(String, String)>,
}

fn pick_candidate<'a>(
    pool: &[&'a RelinkCandidate],
    artist_lower: Option<&str>,
    album_lower: Option<&str>,
) -> Option<&'a RelinkCandidate> {
    let filtered: Vec<&'a RelinkCandidate> = pool
        .iter()
        .copied()
        .filter(|c| match artist_lower {
            None => true,
            Some(an) => {
                c.album_artist_lower.as_deref() == Some(an)
                    || c.artists_lower.iter().any(|a| a == an)
            }
        })
        .collect();
    if let Some(album) = album_lower {
        if let Some(c) = filtered
            .iter()
            .copied()
            .find(|c| c.albums.iter().any(|(name, _)| name == album))
        {
            return Some(c);
        }
    }
    filtered.first().copied()
}

fn album_uid_for(candidate: &RelinkCandidate, album_lower: Option<&str>) -> Option<String> {
    if let Some(album) = album_lower {
        if let Some((_, uid)) = candidate.albums.iter().find(|(name, _)| name == album) {
            return Some(uid.clone());
        }
    }
    candidate.albums.first().map(|(_, uid)| uid.clone())
}

pub fn relink_unmatched_scrobbles(profile_uid: &str) -> Result<usize, String> {
    use std::collections::{HashMap, HashSet};

    let analytics = open_analytics_conn(profile_uid).map_err(|e| e.to_string())?;

    type Row = (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let rows: Vec<Row> = {
        let mut stmt = analytics
            .prepare(
                "SELECT uid, track_uid, artist_uid, album_uid, track_name, track_artist, track_album
                 FROM scrobbles",
            )
            .map_err(|e| e.to_string())?;
        let collected: Vec<Row> = stmt
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            })
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
    let album_uids: HashSet<String> = merged
        .prepare("SELECT uid FROM albums")
        .and_then(|mut stmt| {
            let found: rusqlite::Result<HashSet<String>> =
                stmt.query_map([], |r| r.get::<_, String>(0))?.collect();
            found
        })
        .unwrap_or_default();
    drop(merged);

    let mut candidates: Vec<RelinkCandidate> = Vec::with_capacity(tracks.len());
    let mut by_title: HashMap<String, Vec<usize>> = HashMap::new();
    let mut by_uid: HashMap<String, usize> = HashMap::new();
    for t in &tracks {
        let Some(title) = t.title.as_deref() else { continue };
        let artists_lower: Vec<String> = t
            .artists
            .as_deref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .map(|v| v.iter().map(|a| a.to_lowercase()).collect())
            .unwrap_or_default();
        let albums: Vec<(String, String)> = t
            .albums
            .as_deref()
            .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
            .unwrap_or_default()
            .iter()
            .filter_map(|e| {
                let uid = e.get("uid").and_then(|v| v.as_str()).unwrap_or("");
                if uid.is_empty() {
                    return None;
                }
                let name = e
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_lowercase();
                Some((name, uid.to_string()))
            })
            .collect();
        let index = candidates.len();
        candidates.push(RelinkCandidate {
            uid: t.uid.clone(),
            album_artist_lower: t.album_artist.as_ref().map(|s| s.to_lowercase()),
            artists_lower,
            albums,
        });
        by_title.entry(title.trim().to_lowercase()).or_default().push(index);
        by_uid.insert(t.uid.clone(), index);
    }

    let artist_uids: HashSet<&str> = artists.iter().map(|a| a.uid.as_str()).collect();
    let mut artist_index: HashMap<String, String> = HashMap::new();
    for a in &artists {
        artist_index
            .entry(a.name.trim().to_lowercase())
            .or_insert_with(|| a.uid.clone());
    }

    let mut match_cache: HashMap<(String, String, String), Option<(String, Option<String>)>> =
        HashMap::new();

    let tx = analytics.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut updated = 0usize;

    for (uid, track_uid, artist_uid, album_uid, track_name, track_artist, track_album) in rows {
        let track_uid = track_uid.unwrap_or_default();
        let artist_uid = artist_uid.unwrap_or_default();
        let album_uid = album_uid.unwrap_or_default();
        let name_lower = track_name.as_deref().map(|s| s.trim().to_lowercase()).unwrap_or_default();
        let artist_lower = track_artist
            .as_deref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());
        let album_lower = track_album
            .as_deref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());

        let track_stale = !name_lower.is_empty()
            && (track_uid.is_empty() || !by_uid.contains_key(&track_uid));
        let artist_stale = artist_lower.is_some()
            && (artist_uid.is_empty() || !artist_uids.contains(artist_uid.as_str()));
        let album_stale = album_lower.is_some()
            && (album_uid.is_empty() || !album_uids.contains(&album_uid));

        if !track_stale && !artist_stale && !album_stale {
            continue;
        }

        let mut new_track: Option<String> = None;
        let mut new_album: Option<String> = None;
        let mut new_artist: Option<String> = None;

        if track_stale {
            let key = (
                name_lower.clone(),
                artist_lower.clone().unwrap_or_default(),
                album_lower.clone().unwrap_or_default(),
            );
            let found = match_cache.entry(key).or_insert_with(|| {
                by_title.get(&name_lower).and_then(|indexes| {
                    let subset: Vec<&RelinkCandidate> =
                        indexes.iter().map(|&i| &candidates[i]).collect();
                    pick_candidate(&subset, artist_lower.as_deref(), album_lower.as_deref())
                        .map(|c| (c.uid.clone(), album_uid_for(c, album_lower.as_deref())))
                })
            });
            if let Some((t_uid, a_uid)) = found.clone() {
                new_track = Some(t_uid);
                new_album = a_uid;
            }
        } else if album_stale {
            if let Some(&index) = by_uid.get(&track_uid) {
                new_album = album_uid_for(&candidates[index], album_lower.as_deref());
            }
        }

        if artist_stale {
            new_artist = artist_lower
                .as_deref()
                .and_then(|an| artist_index.get(an).cloned());
        }

        if new_track.is_none() && new_artist.is_none() && new_album.is_none() {
            continue;
        }

        let changed = tx
            .execute(
                "UPDATE scrobbles SET
                    track_uid = COALESCE(?1, track_uid),
                    artist_uid = COALESCE(?2, artist_uid),
                    album_uid = COALESCE(?3, album_uid)
                 WHERE uid = ?4",
                rusqlite::params![new_track, new_artist, new_album, uid],
            )
            .map_err(|e| e.to_string())?;
        updated += changed;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(updated)
}

static RELINK_PENDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn schedule_scrobble_relink(app: tauri::AppHandle, profile_uid: String) {
    use std::sync::atomic::Ordering;
    use tauri::Emitter;

    if RELINK_PENDING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(3));
        RELINK_PENDING.store(false, Ordering::SeqCst);
        match relink_unmatched_scrobbles(&profile_uid) {
            Ok(n) if n > 0 => {
                app.emit("scrobbles:relinked", n).ok();
            }
            Ok(_) => {}
            Err(e) => eprintln!("[scrobbles] relink failed: {e}"),
        }
    });
}

#[tauri::command]
pub async fn relink_scrobbles_cmd(state: State<'_, AppState>) -> Result<usize, String> {
    let uid = state.get_uid();
    crate::library_manager::run_blocking(move || relink_unmatched_scrobbles(&uid)).await
}
