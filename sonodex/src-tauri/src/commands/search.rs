use crate::db::album_manager::Album;
use crate::db::artist_manager::Artist;
use crate::db::search_manager;
use crate::db::track_manager::Track;
use crate::open_merged_conn;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn search_tracks(state: State<AppState>, query: String) -> Result<Vec<Track>, String> {
    let uid = state.get_uid();
    let conn = open_merged_conn(&uid);
    search_manager::search_tracks(&conn, &query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_albums(state: State<AppState>, query: String) -> Result<Vec<Album>, String> {
    let uid = state.get_uid();
    let conn = open_merged_conn(&uid);
    search_manager::search_albums(&conn, &query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_artists(state: State<AppState>, query: String) -> Result<Vec<Artist>, String> {
    let uid = state.get_uid();
    let conn = open_merged_conn(&uid);
    search_manager::search_artists(&conn, &query).map_err(|e| e.to_string())
}
