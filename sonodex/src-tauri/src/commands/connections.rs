use crate::state::AppState;
use crate::open_settings_conn;
use crate::{lastfm_auth, spotify_auth};
use tauri::{AppHandle, State};

// ─── Last.fm ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn lastfm_get_auth_url(state: State<'_, AppState>) -> Result<String, String> {
	let uid = state.get_uid();
	let conn = open_settings_conn(&uid);
	let api_key = crate::db::settings_manager::get_setting(&conn, "api_lastfm_key")
		.map_err(|e| e.to_string())?
		.unwrap_or_default();
	if api_key.is_empty() {
		return Err(
			"Last.fm API key not configured. Add it in Settings → Metadata APIs.".into(),
		);
	}
	Ok(lastfm_auth::lastfm_auth_url(&api_key))
}

#[tauri::command]
pub async fn lastfm_exchange_token_cmd(
	token: String,
	state: State<'_, AppState>,
) -> Result<(), String> {
	let uid = state.get_uid();
	lastfm_auth::lastfm_exchange_token(&uid, &token).await
}

#[tauri::command]
pub fn lastfm_disconnect_cmd(state: State<'_, AppState>) -> Result<(), String> {
	let uid = state.get_uid();
	lastfm_auth::lastfm_disconnect(&uid)
}

#[tauri::command]
pub fn lastfm_connection_status(state: State<'_, AppState>) -> Result<bool, String> {
	let uid = state.get_uid();
	let conn = open_settings_conn(&uid);
	Ok(lastfm_auth::lastfm_is_connected(&conn))
}

#[tauri::command]
pub async fn scrobble_track(uid: String, state: State<'_, AppState>) -> Result<(), String> {
	let profile_uid = state.get_uid();
	let timestamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_default()
		.as_secs() as i64;
	let _ = lastfm_auth::scrobble_track(&profile_uid, &uid).await;
	let _ =
		crate::connections::listenbrainz::submit_listen(&profile_uid, &uid, timestamp).await;
	Ok(())
}

#[tauri::command]
pub async fn update_now_playing(uid: String, state: State<'_, AppState>) -> Result<(), String> {
	let profile_uid = state.get_uid();
	let _ = lastfm_auth::update_now_playing(&profile_uid, &uid).await;
	let _ = crate::connections::listenbrainz::update_now_playing(&profile_uid, &uid).await;
	Ok(())
}

// ─── ListenBrainz ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn listenbrainz_connection_status(state: State<AppState>) -> Result<bool, String> {
	let uid = state.get_uid();
	let conn = open_settings_conn(&uid);
	Ok(crate::connections::listenbrainz::listenbrainz_is_connected(&conn))
}

#[tauri::command]
pub async fn listenbrainz_validate_token_cmd(token: String) -> Result<bool, String> {
	crate::connections::listenbrainz::listenbrainz_validate_token(&token).await
}

#[tauri::command]
pub fn listenbrainz_connect_cmd(state: State<AppState>, token: String) -> Result<(), String> {
	let uid = state.get_uid();
	crate::connections::listenbrainz::listenbrainz_connect(&uid, token)
}

#[tauri::command]
pub fn listenbrainz_disconnect_cmd(state: State<AppState>) -> Result<(), String> {
	let uid = state.get_uid();
	crate::connections::listenbrainz::listenbrainz_disconnect(&uid)
}

// ─── Spotify ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn spotify_get_auth_url(state: State<'_, AppState>) -> Result<(String, String), String> {
	let uid = state.get_uid();
	let conn = open_settings_conn(&uid);
	let client_id = crate::db::settings_manager::get_setting(&conn, "spotify_client_id")
		.map_err(|e| e.to_string())?
		.unwrap_or_default();
	if client_id.is_empty() {
		return Err(
			"Spotify client ID not configured. Add it in Settings → Connected Accounts.".into(),
		);
	}
	let (url, verifier): (String, String) = spotify_auth::spotify_auth_url(&client_id);
	state.set_spotify_verifier(verifier.clone());
	Ok((url, verifier))
}

#[tauri::command]
pub async fn spotify_exchange_code_cmd(
	code: String,
	verifier: String,
	state: State<'_, AppState>,
) -> Result<(), String> {
	let uid = state.get_uid();
	spotify_auth::spotify_exchange_code(&uid, &code, &verifier).await
}

#[tauri::command]
pub fn spotify_disconnect_cmd(state: State<'_, AppState>) -> Result<(), String> {
	let uid = state.get_uid();
	spotify_auth::spotify_disconnect(&uid)
}

#[tauri::command]
pub fn spotify_connection_status(state: State<'_, AppState>) -> Result<bool, String> {
	let uid = state.get_uid();
	let conn = open_settings_conn(&uid);
	Ok(spotify_auth::spotify_is_connected(&conn))
}

#[tauri::command]
pub async fn spotify_get_playlists_cmd(
	state: State<'_, AppState>,
) -> Result<Vec<spotify_auth::SpotifyPlaylistSummary>, String> {
	let uid = state.get_uid();
	spotify_auth::spotify_get_playlists(&uid).await
}

#[tauri::command]
pub async fn spotify_import_playlist_cmd(
	spotify_playlist_id: String,
	playlist_name: String,
	owner: Option<String>,
	state: State<'_, AppState>,
) -> Result<String, String> {
	let uid = state.get_uid();
	spotify_auth::spotify_import_playlist(&uid, &spotify_playlist_id, &playlist_name, owner).await
}

#[tauri::command]
pub async fn spotify_get_playlist_info_cmd(
	playlist_id: String,
	state: State<'_, AppState>,
) -> Result<spotify_auth::SpotifyPlaylistInfo, String> {
	let uid = state.get_uid();
	spotify_auth::spotify_get_playlist_info(&uid, &playlist_id).await
}

// ─── Spotify history import ───────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct SpotifyStreamEntry {
	ts: Option<String>,
	ms_played: Option<i64>,
	master_metadata_track_name: Option<String>,
	master_metadata_album_artist_name: Option<String>,
	master_metadata_album_album_name: Option<String>,
	reason_start: Option<String>,
	reason_end: Option<String>,
	shuffle: Option<bool>,
	skipped: Option<bool>,
	offline: Option<bool>,
}

#[derive(serde::Serialize)]
pub struct ImportResult {
	imported: usize,
	skipped: usize,
}

#[tauri::command]
pub fn import_spotify_history_cmd(
	state: State<AppState>,
	zip_path: String,
) -> Result<ImportResult, String> {
	use std::io::Read;

	let uid = state.get_uid();
	let file =
		std::fs::File::open(&zip_path).map_err(|e| format!("open zip: {}", e))?;
	let mut archive =
		zip::ZipArchive::new(file).map_err(|e| format!("read zip: {}", e))?;

	let merged_conn = crate::open_merged_conn(&uid);
	let analytics_conn = crate::open_analytics_conn(&uid).map_err(|e| e.to_string())?;

	let all_tracks = crate::db::get_all_tracks(&merged_conn).unwrap_or_default();
	let all_artists = crate::db::get_all_artists(&merged_conn).unwrap_or_default();

	let mut imported = 0usize;
	let mut skipped = 0usize;

	for i in 0..archive.len() {
		let mut entry = archive
			.by_index(i)
			.map_err(|e| format!("zip entry: {}", e))?;

		if entry.is_dir() {
			continue;
		}

		let name = entry.name().to_string();
		if !name.contains("Streaming_History_Audio") || !name.ends_with(".json") {
			continue;
		}

		let mut content = String::new();
		if entry.read_to_string(&mut content).is_err() {
			continue;
		}

		let entries: Vec<SpotifyStreamEntry> = match serde_json::from_str(&content) {
			Ok(v) => v,
			Err(_) => continue,
		};

		for item in entries {
			let ms_played = item.ms_played.unwrap_or(0);
			if ms_played < 5000 {
				skipped += 1;
				continue;
			}

			let timestamp = item
				.ts
				.as_deref()
				.and_then(|ts| {
					chrono::DateTime::parse_from_rfc3339(ts)
						.ok()
						.map(|dt| dt.timestamp())
				})
				.unwrap_or(0);

			let track_name = item.master_metadata_track_name.clone();
			let artist_name = item.master_metadata_album_artist_name.clone();
			let album_name = item.master_metadata_album_album_name.clone();

			let track_uid = track_name
				.as_deref()
				.and_then(|tn| {
					let tn_lower = tn.to_lowercase();
					all_tracks
						.iter()
						.find(|t| {
							t.title.as_deref().map(|s| s.to_lowercase())
								== Some(tn_lower.clone())
								&& artist_name.as_deref().map_or(true, |an| {
									let an_lower = an.to_lowercase();
									t.album_artist
										.as_ref()
										.map(|s| s.to_lowercase())
										== Some(an_lower.clone())
										|| t.artists
											.as_deref()
											.and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
											.map_or(false, |v| {
												v.iter().any(|a| a.to_lowercase() == an_lower)
											})
								})
						})
						.map(|t| t.uid.clone())
				})
				.unwrap_or_default();

			let artist_uid = artist_name
				.as_deref()
				.and_then(|an| {
					let an_lower = an.to_lowercase();
					all_artists
						.iter()
						.find(|a| a.name.to_lowercase() == an_lower)
						.map(|a| a.uid.clone())
				})
				.unwrap_or_default();

			let album_uid = track_uid.is_empty().then_some(None).unwrap_or_else(|| {
				all_tracks.iter()
					.find(|t| t.uid == track_uid)
					.and_then(|t| crate::db::first_album_uid(&t.albums))
			});

			let scrobble_uid = crate::db::analytics_manager::new_scrobble_uid();
			let scrobble = crate::db::analytics_manager::Scrobble {
				uid: scrobble_uid,
				timestamp,
				track_uid,
				artist_uid,
				album_uid,
				source_uid: None,
				duration_played: ms_played,
				did_seek: false,
				did_pause: false,
				reason_start: item.reason_start,
				reason_end: item.reason_end,
				shuffle: item.shuffle,
				skipped: item.skipped,
				offline: item.offline,
				playing_local: Some(false),
				track_name,
				track_artist: artist_name,
				track_album: album_name,
			};

			match crate::db::analytics_manager::log_scrobble(&analytics_conn, &scrobble) {
				Ok(_) => imported += 1,
				Err(_) => skipped += 1,
			}
		}
	}

	Ok(ImportResult { imported, skipped })
}