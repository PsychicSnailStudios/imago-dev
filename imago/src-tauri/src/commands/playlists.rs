use crate::db::{
    self, create_playlist, delete_playlist_by_uid, get_all_playlists, get_playlist_by_uid,
    update_playlist_by_uid, Playlist, PlaylistUpdate,
};
use crate::profiles::get_playlists_db_path;
use crate::state::AppState;
use crate::{open_lib_conn, open_settings_conn};
use rusqlite::Connection;
use tauri::State;

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