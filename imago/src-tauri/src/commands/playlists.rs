use crate::db::{
    self, create_playlist, delete_playlist_by_uid, get_all_playlists, get_playlist_by_uid,
    update_playlist_by_uid, Playlist, PlaylistUpdate,
};
use crate::profiles::get_playlists_db_path;
use crate::state::AppState;
use crate::{open_lib_conn, open_settings_conn};
use rusqlite::{Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};
use tauri::State;

use crate::db::search_manager;
use crate::db::Track;

fn open_playlists_conn(profile_uid: &str) -> Connection {
    let path = get_playlists_db_path(profile_uid);
    let conn = Connection::open(&path).expect("Failed to open playlists db");
    crate::db::init_playlists_db(&conn).ok();
    conn
}

// Reads from playlists.db if it exists and has rows, otherwise falls back to lib.db.
// This handles the transition period before a full migration is done.
pub(crate) fn open_best_playlists_conn(profile_uid: &str) -> Connection {
    let playlists_path = get_playlists_db_path(profile_uid);
    if playlists_path.exists() {
        let conn = Connection::open(&playlists_path).expect("Failed to open playlists db");
        crate::db::init_playlists_db(&conn).ok();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM playlists", [], |r| r.get(0))
            .unwrap_or(0);
        if count > 0 {
            return conn;
        }
    }
    open_lib_conn(profile_uid)
}

#[tauri::command]
pub fn get_playlists(state: State<AppState>) -> Result<Vec<Playlist>, String> {
    let uid = state.get_uid();
    let conn = open_best_playlists_conn(&uid);
    get_all_playlists(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_playlist(state: State<AppState>, uid: String) -> Result<Option<Playlist>, String> {
    let profile_uid = state.get_uid();
    let conn = open_best_playlists_conn(&profile_uid);
    get_playlist_by_uid(&conn, &uid).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_playlist_artwork(
    state: State<AppState>,
    uid: String,
) -> Result<Option<Vec<u8>>, String> {
    let profile_uid = state.get_uid();
    let conn = open_best_playlists_conn(&profile_uid);
    db::get_playlist_artwork_by_uid(&conn, &uid).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_playlist_entry(state: State<AppState>, playlist: Playlist) -> Result<(), String> {
    let uid = state.get_uid();
    let conn = open_playlists_conn(&uid);
    create_playlist(&conn, &playlist).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_playlist_entry(
    state: State<AppState>,
    uid: String,
    update: PlaylistUpdate,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let conn = open_best_playlists_conn(&profile_uid);
    update_playlist_by_uid(&conn, &uid, &update).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_playlist_entry(state: State<AppState>, uid: String) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let conn = open_best_playlists_conn(&profile_uid);
    delete_playlist_by_uid(&conn, &uid).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_playlist_folder(
    state: State<AppState>,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    let uid = state.get_uid();
    let conn = open_best_playlists_conn(&uid);
    crate::db::playlist_manager::rename_folder(&conn, &old_path, &new_path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn move_playlists_to_folder(
    state: State<AppState>,
    old_folder: String,
    new_folder: String,
) -> Result<(), String> {
    let uid = state.get_uid();
    let conn = open_best_playlists_conn(&uid);
    crate::db::playlist_manager::move_playlists_to_folder(&conn, &old_folder, &new_folder)
        .map_err(|e| e.to_string())
}

fn entry_str(entry: &serde_json::Value, key: &str) -> Option<String> {
    entry
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn set_entry_field(entry: &mut serde_json::Value, key: &str, value: Option<String>) -> bool {
    let Some(value) = value.filter(|v| !v.trim().is_empty()) else {
        return false;
    };
    if entry_str(entry, key).as_deref() == Some(value.as_str()) {
        return false;
    }
    match entry.as_object_mut() {
        Some(obj) => {
            obj.insert(key.to_string(), serde_json::Value::String(value));
            true
        }
        None => false,
    }
}

fn artist_names(json: &Option<String>) -> Vec<String> {
    json.as_deref()
        .and_then(|j| serde_json::from_str::<Vec<String>>(j).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn album_names(json: &Option<String>) -> Vec<String> {
    json.as_deref()
        .and_then(|j| serde_json::from_str::<Vec<serde_json::Value>>(j).ok())
        .unwrap_or_default()
        .iter()
        .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

struct Candidate {
    uid: String,
    artists_lower: Vec<String>,
    albums_lower: Vec<String>,
    playable: bool,
}

fn load_candidates(
    conn: &Connection,
    titles: &HashSet<String>,
) -> Result<HashMap<String, Vec<Candidate>>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT uid, title, artists, album_artist, albums, path, remote_path
             FROM tracks WHERE title IS NOT NULL",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut out: HashMap<String, Vec<Candidate>> = HashMap::new();
    for row in rows.filter_map(|r| r.ok()) {
        let (uid, title, artists, album_artist, albums, path, remote_path) = row;
        let title_lower = title.trim().to_lowercase();
        if !titles.contains(&title_lower) {
            continue;
        }
        let mut artists_lower: Vec<String> =
            artist_names(&artists).iter().map(|a| a.to_lowercase()).collect();
        if let Some(aa) = album_artist {
            let aa = aa.trim().to_lowercase();
            if !aa.is_empty() {
                artists_lower.push(aa);
            }
        }
        let albums_lower: Vec<String> =
            album_names(&albums).iter().map(|a| a.to_lowercase()).collect();
        let playable = path.map(|p| !p.is_empty() && p != uid).unwrap_or(false)
            || remote_path.map(|p| !p.is_empty()).unwrap_or(false);
        out.entry(title_lower).or_default().push(Candidate {
            uid,
            artists_lower,
            albums_lower,
            playable,
        });
    }
    Ok(out)
}

fn pick_candidate(
    artist: Option<&str>,
    album: Option<&str>,
    candidates: &[Candidate],
    taken: &HashSet<String>,
) -> Option<String> {
    let mut pool: Vec<&Candidate> = candidates.iter().filter(|c| !taken.contains(&c.uid)).collect();
    match artist {
        Some(artist) => {
            let artist = artist.to_lowercase();
            pool.retain(|c| c.artists_lower.iter().any(|a| *a == artist));
            if let Some(album) = album {
                let album = album.to_lowercase();
                let narrowed: Vec<&Candidate> = pool
                    .iter()
                    .copied()
                    .filter(|c| c.albums_lower.iter().any(|a| *a == album))
                    .collect();
                if !narrowed.is_empty() {
                    pool = narrowed;
                }
            }
            pool.sort_by(|a, b| b.playable.cmp(&a.playable).then(a.uid.cmp(&b.uid)));
            pool.first().map(|c| c.uid.clone())
        }
        None => {
            if pool.len() == 1 {
                Some(pool[0].uid.clone())
            } else {
                None
            }
        }
    }
}

fn ghost_track(entry: &serde_json::Value, uid: &str) -> Track {
    let artist = entry_str(entry, "artist");
    let album = entry_str(entry, "album");
    Track {
        id: None,
        uid: uid.to_string(),
        path: String::new(),
        last_modified: 0,
        title: entry_str(entry, "name"),
        artists: artist.map(|a| serde_json::json!([a]).to_string()),
        album_artist: None,
        albums: album.map(|a| {
            serde_json::json!([{ "uid": "", "name": a, "track_number": null, "disc": null }])
                .to_string()
        }),
        genres: None,
        year: None,
        rating: None,
        tags: None,
        duration_ms: None,
        bpm: None,
        key: None,
        credits: None,
        label: None,
        artwork_blob: None,
        artwork_thumb: None,
        artwork_path: None,
        user_options: None,
        format: None,
        bitrate: None,
        remote_path: None,
        remote_data: None,
        track_data: Some("{\"is_ghost\":true}".to_string()),
        date_added: None,
    }
}

fn resolve_playlist_tracks(profile_uid: &str, playlist_uid: &str) -> Result<Vec<Track>, String> {
    let playlists_conn = open_best_playlists_conn(profile_uid);
    let raw: Option<String> = playlists_conn
        .query_row(
            "SELECT tracks FROM playlists WHERE uid = ?1",
            rusqlite::params![playlist_uid],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();
    let Some(raw) = raw else {
        return Ok(vec![]);
    };

    let mut entries: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap_or_default();
    if entries.is_empty() {
        return Ok(vec![]);
    }
    entries.sort_by_key(|e| e.get("order").and_then(|v| v.as_i64()).unwrap_or(0));

    let uids: Vec<String> = entries.iter().filter_map(|e| entry_str(e, "uid")).collect();
    let merged_conn = crate::open_merged_conn(profile_uid);
    let mut found: HashMap<String, Track> = search_manager::get_tracks_by_uids(&merged_conn, &uids)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|t| (t.uid.clone(), t))
        .collect();

    let mut dirty = false;
    let mut taken: HashSet<String> = found.keys().cloned().collect();

    let missing: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| entry_str(e, "uid").map(|u| !found.contains_key(&u)).unwrap_or(true))
        .map(|(i, _)| i)
        .collect();

    if !missing.is_empty() {
        let titles: HashSet<String> = missing
            .iter()
            .filter_map(|&i| entry_str(&entries[i], "name"))
            .map(|n| n.to_lowercase())
            .collect();
        let candidates = load_candidates(&merged_conn, &titles)?;

        let mut healed: Vec<(usize, String)> = Vec::new();
        for &i in &missing {
            let Some(name) = entry_str(&entries[i], "name") else {
                continue;
            };
            let Some(list) = candidates.get(&name.to_lowercase()) else {
                continue;
            };
            let artist = entry_str(&entries[i], "artist");
            let album = entry_str(&entries[i], "album");
            if let Some(new_uid) = pick_candidate(artist.as_deref(), album.as_deref(), list, &taken) {
                taken.insert(new_uid.clone());
                healed.push((i, new_uid));
            }
        }

        if !healed.is_empty() {
            let new_uids: Vec<String> = healed.iter().map(|(_, u)| u.clone()).collect();
            for track in search_manager::get_tracks_by_uids(&merged_conn, &new_uids)
                .map_err(|e| e.to_string())?
            {
                found.insert(track.uid.clone(), track);
            }
            for (i, new_uid) in healed {
                if set_entry_field(&mut entries[i], "uid", Some(new_uid)) {
                    dirty = true;
                }
            }
        }
    }

    for entry in entries.iter_mut() {
        let Some(uid) = entry_str(entry, "uid") else {
            continue;
        };
        let Some(track) = found.get(&uid) else {
            continue;
        };
        if set_entry_field(entry, "name", track.title.clone()) {
            dirty = true;
        }
        if set_entry_field(entry, "artist", artist_names(&track.artists).into_iter().next()) {
            dirty = true;
        }
        if set_entry_field(entry, "album", album_names(&track.albums).into_iter().next()) {
            dirty = true;
        }
    }

    if dirty {
        match serde_json::to_string(&entries) {
            Ok(json) => {
                if let Err(e) = playlists_conn.execute(
                    "UPDATE playlists SET tracks = ?1 WHERE uid = ?2",
                    rusqlite::params![json, playlist_uid],
                ) {
                    eprintln!("[playlists] failed to save resolved tracks: {e}");
                }
            }
            Err(e) => eprintln!("[playlists] failed to serialise resolved tracks: {e}"),
        }
    }

    let mut out: Vec<Track> = Vec::with_capacity(entries.len());
    for entry in &entries {
        let uid = entry_str(entry, "uid").unwrap_or_default();
        match found.get(&uid) {
            Some(track) => out.push(track.clone()),
            None => out.push(ghost_track(entry, &uid)),
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn get_playlist_tracks(
    state: State<'_, AppState>,
    uid: String,
) -> Result<Vec<Track>, String> {
    let profile_uid = state.get_uid();
    crate::library_manager::run_blocking(move || resolve_playlist_tracks(&profile_uid, &uid)).await
}