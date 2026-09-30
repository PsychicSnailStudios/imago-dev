use crate::db::analytics_manager::{self, Scrobble, ScrobbleUpdate};
use crate::state::AppState;
use crate::open_analytics_conn;
use tauri::State;

#[tauri::command]
pub fn get_scrobbles(state: State<AppState>) -> Result<Vec<Scrobble>, String> {
	let uid = state.get_uid();
	let conn = open_analytics_conn(&uid).map_err(|e| e.to_string())?;
	analytics_manager::get_all_scrobbles(&conn).map_err(|e| e.to_string())
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