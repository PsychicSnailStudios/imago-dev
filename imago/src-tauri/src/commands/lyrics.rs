use crate::db::{self, Lyrics};
use crate::library_manager;
use crate::state::AppState;
use crate::{open_lib_conn, open_merged_conn};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn get_all_lyrics(state: State<AppState>) -> Result<Vec<Lyrics>, String> {
    let profile_uid = state.get_uid();
    let conn = open_merged_conn(&profile_uid);
    db::lyrics_manager::get_all_lyrics(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_track_lyrics(state: State<AppState>, uid: String) -> Result<Option<Lyrics>, String> {
    let profile_uid = state.get_uid();
    let conn = open_merged_conn(&profile_uid);
    let track = db::get_track_by_uid(&conn, &uid)
        .map_err(|e| e.to_string())?
        .ok_or("Track not found")?;
    let track_id = track.id.ok_or("Track has no id")?;
    db::lyrics_manager::get_lyrics(&conn, track_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fetch_track_lyrics(
    app: AppHandle,
    state: State<'_, AppState>,
    uid: String,
) -> Result<(), String> {
    let profile_uid = state.get_uid();

    let (title, artist, album, duration_secs) = {
        let conn = open_merged_conn(&profile_uid);
        let track = db::get_track_by_uid(&conn, &uid)
            .map_err(|e| e.to_string())?
            .ok_or("Track not found")?;
        let title = track.title.clone().unwrap_or_default();
        let artist =
            db::resolved_artist_name(&track.album_artist, &track.artists).unwrap_or_default();
        let album = db::first_album_name(&track.albums);
        let duration_secs = track.duration_ms.map(|ms| (ms / 1000) as u64);
        (title, artist, album, duration_secs)
    };

    let client = crate::enrichment::make_client()?;
    let result = crate::enrichment::lyrics::fetch_lyrics(
        &client,
        &title,
        &artist,
        album.as_deref(),
        duration_secs,
    )
    .await;

    if let Some(lyrics) = result {
        match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "tracks") {
            Ok((source_conn, lib)) => {
                let track = db::get_track_by_uid(&source_conn, &uid)
                    .map_err(|e| e.to_string())?
                    .ok_or("Track not found in source")?;
                let track_id = track.id.ok_or("Track has no id")?;
                db::lyrics_manager::upsert_lyrics(
                    &source_conn,
                    &Lyrics {
                        id: None,
                        track_uid: String::new(),
                        track_id,
                        source: lyrics.source,
                        plain: lyrics.plain,
                        synced: lyrics.synced,
                        instrumental: lyrics.instrumental,
                    },
                )
                .map_err(|e| e.to_string())?;
                library_manager::incremental_update(&profile_uid, &[lib.uid])
                    .map_err(|e| e.to_string())?;
            }
            Err(_) => {
                let conn = open_lib_conn(&profile_uid);
                let track = db::get_track_by_uid(&conn, &uid)
                    .map_err(|e| e.to_string())?
                    .ok_or("Track not found")?;
                let track_id = track.id.ok_or("Track has no id")?;
                db::lyrics_manager::upsert_lyrics(
                    &conn,
                    &Lyrics {
                        id: None,
                        track_uid: String::new(),
                        track_id,
                        source: lyrics.source,
                        plain: lyrics.plain,
                        synced: lyrics.synced,
                        instrumental: lyrics.instrumental,
                    },
                )
                .map_err(|e| e.to_string())?;
            }
        }
        app.emit("lyrics:updated", uid).ok();
    }

    Ok(())
}

#[tauri::command]
pub fn delete_track_lyrics(state: State<AppState>, uid: String) -> Result<(), String> {
    let profile_uid = state.get_uid();
    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "tracks") {
        Ok((source_conn, lib)) => {
            let track = db::get_track_by_uid(&source_conn, &uid)
                .map_err(|e| e.to_string())?
                .ok_or("Track not found")?;
            let track_id = track.id.ok_or("Track has no id")?;
            db::lyrics_manager::delete_lyrics(&source_conn, track_id).map_err(|e| e.to_string())?;
            library_manager::incremental_update(&profile_uid, &[lib.uid])
                .map_err(|e| e.to_string())?;
        }
        Err(_) => {
            let conn = open_lib_conn(&profile_uid);
            let track = db::get_track_by_uid(&conn, &uid)
                .map_err(|e| e.to_string())?
                .ok_or("Track not found")?;
            let track_id = track.id.ok_or("Track has no id")?;
            db::lyrics_manager::delete_lyrics(&conn, track_id).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub async fn fetch_missing_lyrics(app: AppHandle, profile_uid: String) {
	let candidates: Vec<(String, String, String, Option<String>, Option<u64>)> = {
		let conn = open_merged_conn(&profile_uid);
		db::get_all_tracks(&conn)
			.unwrap_or_default()
			.into_iter()
			.filter_map(|t| {
				let id = t.id?;
				let title = t.title.clone().unwrap_or_default();
				let artist = db::resolved_artist_name(&t.album_artist, &t.artists).unwrap_or_default();
				if title.is_empty() || artist.is_empty() {
					return None;
				}
				let has_lyrics = db::lyrics_manager::get_lyrics(&conn, id)
					.ok()
					.flatten()
					.is_some();
				if has_lyrics {
					return None;
				}
				let album = db::first_album_name(&t.albums);
				let duration_secs = t.duration_ms.map(|ms| (ms / 1000) as u64);
				Some((t.uid, title, artist, album, duration_secs))
			})
			.collect()
	};

	if candidates.is_empty() {
		return;
	}

	let client = match crate::enrichment::make_client() {
		Ok(c) => c,
		Err(_) => return,
	};

	let mut touched: std::collections::HashSet<String> = std::collections::HashSet::new();
	let mut since_refresh = 0usize;

	for (uid, title, artist, album, duration_secs) in candidates {
		let result = crate::enrichment::lyrics::fetch_lyrics(
			&client,
			&title,
			&artist,
			album.as_deref(),
			duration_secs,
		)
		.await;

		let Some(lyrics) = result else {
			continue;
		};

		let Ok((source_conn, lib)) =
			library_manager::open_source_conn_for_entity(&profile_uid, &uid, "tracks")
		else {
			continue;
		};
		let Some(track_id) = db::get_track_by_uid(&source_conn, &uid)
			.ok()
			.flatten()
			.and_then(|t| t.id)
		else {
			continue;
		};

		let saved = db::lyrics_manager::upsert_lyrics(
			&source_conn,
			&Lyrics {
				id: None,
				track_uid: String::new(),
				track_id,
				source: lyrics.source,
				plain: lyrics.plain,
				synced: lyrics.synced,
				instrumental: lyrics.instrumental,
			},
		)
		.is_ok();

		if saved {
			touched.insert(lib.uid);
			since_refresh += 1;
		}

		if since_refresh >= 50 {
			for lib_uid in touched.drain() {
				let _ = library_manager::incremental_update(&profile_uid, &[lib_uid]);
			}
			app.emit("library:updated", ()).ok();
			since_refresh = 0;
		}
	}

	for lib_uid in touched.drain() {
		let _ = library_manager::incremental_update(&profile_uid, &[lib_uid]);
	}
	app.emit("library:updated", ()).ok();
}