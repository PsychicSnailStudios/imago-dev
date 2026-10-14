use crate::db::album_manager::{update_album_by_uid, AlbumUpdate};
use crate::db::artist_manager::{update_artist_by_uid, ArtistUpdate};
use crate::db::library_registry::{get_library_by_uid, Library};
use crate::library_manager;
use crate::state::AppState;
use crate::{open_merged_conn, open_settings_conn};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tauri::State;

#[derive(Serialize, Default)]
pub struct DeleteFailure {
	pub uid: String,
	pub error: String,
}

#[derive(Serialize)]
pub struct ReadOnlyItem {
	pub uid: String,
	pub source_lib_uid: String,
}

#[derive(Serialize, Default)]
pub struct DeleteResult {
	pub deleted: usize,
	pub read_only: Vec<ReadOnlyItem>,
	pub failed: Vec<DeleteFailure>,
}

fn group_by_source(
	merged: &Connection,
	table: &str,
	uids: &[String],
) -> Result<HashMap<String, Vec<String>>, String> {
	let mut groups: HashMap<String, Vec<String>> = HashMap::new();
	let mut stmt = merged
		.prepare_cached(&format!("SELECT source_lib_uid FROM {} WHERE uid = ?1", table))
		.map_err(|e| e.to_string())?;
	for uid in uids {
		let source: Option<String> = stmt.query_row(params![uid], |r| r.get(0)).ok();
		if let Some(s) = source {
			groups.entry(s).or_default().push(uid.clone());
		}
	}
	Ok(groups)
}

fn fail_all(result: &mut DeleteResult, uids: &[String], error: &str) {
	for uid in uids {
		result.failed.push(DeleteFailure {
			uid: uid.clone(),
			error: error.to_string(),
		});
	}
}

fn is_remote(path: &str) -> bool {
	path.starts_with("http://")
		|| path.starts_with("https://")
		|| path.starts_with("\\\\")
		|| path.starts_with("//")
}

fn parse_list(json: &str) -> Option<Vec<serde_json::Value>> {
	serde_json::from_str::<Vec<serde_json::Value>>(json).ok()
}

fn entry_uid(entry: &serde_json::Value) -> &str {
	entry.get("uid").and_then(|u| u.as_str()).unwrap_or("")
}

fn delete_tracks_in_library(
	profile_uid: &str,
	lib: &Library,
	group: &[String],
	delete_files: bool,
	result: &mut DeleteResult,
) -> Result<(), String> {
	let conn = library_manager::open_tuned(&lib.file_path)?;

	let mut to_delete: Vec<String> = Vec::new();
	if delete_files {
		for uid in group {
			let row: Option<(Option<String>, Option<String>)> = conn
				.query_row(
					"SELECT path, remote_path FROM tracks WHERE uid = ?1",
					params![uid],
					|r| Ok((r.get(0)?, r.get(1)?)),
				)
				.ok();
			let path = row.and_then(|(p, _)| p).unwrap_or_default();
			if !path.is_empty() && !is_remote(&path) && std::path::Path::new(&path).exists() {
				if let Err(e) = std::fs::remove_file(&path) {
					result.failed.push(DeleteFailure {
						uid: uid.clone(),
						error: format!("Could not delete file: {}", e),
					});
					continue;
				}
			}
			to_delete.push(uid.clone());
		}
	} else {
		to_delete = group.to_vec();
	}

	if to_delete.is_empty() {
		return Ok(());
	}

	let delete_set: HashSet<&str> = to_delete.iter().map(|s| s.as_str()).collect();

	let album_rows: Vec<(String, Option<String>)> = {
		let mut stmt = conn
			.prepare("SELECT uid, tracks FROM albums WHERE tracks IS NOT NULL AND tracks != '[]'")
			.map_err(|e| e.to_string())?;
		let collected: Vec<(String, Option<String>)> = stmt
			.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
			.map_err(|e| e.to_string())?
			.filter_map(|r| r.ok())
			.collect();
		collected
	};

	let mut album_updates: Vec<(String, Option<String>)> = Vec::new();
	for (album_uid, json) in album_rows {
		let Some(json) = json else { continue };
		let Some(mut list) = parse_list(&json) else { continue };
		let before = list.len();
		list.retain(|e| !delete_set.contains(entry_uid(e)));
		if list.len() == before {
			continue;
		}
		if list.is_empty() {
			album_updates.push((album_uid, None));
		} else {
			let out = serde_json::to_string(&list).map_err(|e| e.to_string())?;
			album_updates.push((album_uid, Some(out)));
		}
	}

	{
		let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
		for (album_uid, new_json) in &album_updates {
			match new_json {
				Some(json) => {
					conn.execute(
						"UPDATE albums SET tracks = ?1 WHERE uid = ?2",
						params![json, album_uid],
					)
					.map_err(|e| e.to_string())?;
				}
				None => {
					conn.execute("DELETE FROM albums WHERE uid = ?1", params![album_uid])
						.map_err(|e| e.to_string())?;
				}
			}
		}
		for uid in &to_delete {
			conn.execute(
				"DELETE FROM lyrics WHERE track_id IN (SELECT id FROM tracks WHERE uid = ?1)",
				params![uid],
			)
			.map_err(|e| e.to_string())?;
			conn.execute("DELETE FROM tracks WHERE uid = ?1", params![uid])
				.map_err(|e| e.to_string())?;
		}
		tx.commit().map_err(|e| e.to_string())?;
	}

	let affected_albums: Vec<String> = album_updates.into_iter().map(|(u, _)| u).collect();
	library_manager::sync_entities(profile_uid, &lib.uid, &to_delete, &affected_albums, &[])?;
	result.deleted += to_delete.len();
	Ok(())
}

fn delete_albums_in_library(
	profile_uid: &str,
	lib: &Library,
	group: &[String],
	result: &mut DeleteResult,
) -> Result<(), String> {
	let conn = library_manager::open_tuned(&lib.file_path)?;
	let delete_set: HashSet<&str> = group.iter().map(|s| s.as_str()).collect();

	let track_rows: Vec<(String, String)> = {
		let mut stmt = conn
			.prepare("SELECT uid, albums FROM tracks WHERE albums IS NOT NULL AND albums != '[]'")
			.map_err(|e| e.to_string())?;
		let collected: Vec<(String, String)> = stmt
			.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
			.map_err(|e| e.to_string())?
			.filter_map(|r| r.ok())
			.collect();
		collected
	};

	let mut track_updates: Vec<(String, String)> = Vec::new();
	for (track_uid, json) in track_rows {
		let Some(mut list) = parse_list(&json) else { continue };
		let before = list.len();
		list.retain(|e| !delete_set.contains(entry_uid(e)));
		if list.len() == before {
			continue;
		}
		let out = serde_json::to_string(&list).map_err(|e| e.to_string())?;
		track_updates.push((track_uid, out));
	}

	{
		let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
		for (track_uid, json) in &track_updates {
			conn.execute(
				"UPDATE tracks SET albums = ?1 WHERE uid = ?2",
				params![json, track_uid],
			)
			.map_err(|e| e.to_string())?;
		}
		for uid in group {
			conn.execute("DELETE FROM albums WHERE uid = ?1", params![uid])
				.map_err(|e| e.to_string())?;
		}
		tx.commit().map_err(|e| e.to_string())?;
	}

	let affected_tracks: Vec<String> = track_updates.into_iter().map(|(u, _)| u).collect();
	library_manager::sync_entities(profile_uid, &lib.uid, &affected_tracks, group, &[])?;
	result.deleted += group.len();
	Ok(())
}

fn delete_artists_in_library(
	profile_uid: &str,
	lib: &Library,
	group: &[String],
	result: &mut DeleteResult,
) -> Result<(), String> {
	let conn = library_manager::open_tuned(&lib.file_path)?;
	{
		let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
		for uid in group {
			conn.execute("DELETE FROM artists WHERE uid = ?1", params![uid])
				.map_err(|e| e.to_string())?;
		}
		tx.commit().map_err(|e| e.to_string())?;
	}
	library_manager::sync_entities(profile_uid, &lib.uid, &[], &[], group)?;
	result.deleted += group.len();
	Ok(())
}

#[derive(Clone, Copy)]
enum Kind {
	Tracks,
	Albums,
	Artists,
}

fn delete_entities(
	profile_uid: &str,
	kind: Kind,
	uids: &[String],
	delete_files: bool,
) -> Result<DeleteResult, String> {
	let table = match kind {
		Kind::Tracks => "tracks",
		Kind::Albums => "albums",
		Kind::Artists => "artists",
	};

	let merged = open_merged_conn(profile_uid);
	let settings = open_settings_conn(profile_uid);
	let groups = group_by_source(&merged, table, uids)?;
	drop(merged);

	let mut result = DeleteResult::default();

	for (lib_uid, group) in groups {
		let lib = match get_library_by_uid(&settings, &lib_uid) {
			Ok(Some(l)) => l,
			_ => {
				fail_all(&mut result, &group, "Source library not found");
				continue;
			}
		};

		if !(lib.is_default || lib.has_write_permission) {
			result
				.read_only
				.extend(group.into_iter().map(|uid| ReadOnlyItem {
					uid,
					source_lib_uid: lib_uid.clone(),
				}));
			continue;
		}

		let outcome = match kind {
			Kind::Tracks => {
				delete_tracks_in_library(profile_uid, &lib, &group, delete_files, &mut result)
			}
			Kind::Albums => delete_albums_in_library(profile_uid, &lib, &group, &mut result),
			Kind::Artists => delete_artists_in_library(profile_uid, &lib, &group, &mut result),
		};

		if let Err(e) = outcome {
			fail_all(&mut result, &group, &e);
		}
	}

	Ok(result)
}

#[tauri::command]
pub async fn delete_tracks_cmd(
	state: State<'_, AppState>,
	uids: Vec<String>,
	delete_files: bool,
) -> Result<DeleteResult, String> {
	let profile_uid = state.get_uid();
	library_manager::run_blocking(move || {
		delete_entities(&profile_uid, Kind::Tracks, &uids, delete_files)
	})
	.await
}

#[tauri::command]
pub async fn delete_albums_cmd(
	state: State<'_, AppState>,
	uids: Vec<String>,
) -> Result<DeleteResult, String> {
	let profile_uid = state.get_uid();
	library_manager::run_blocking(move || delete_entities(&profile_uid, Kind::Albums, &uids, false))
		.await
}

#[tauri::command]
pub async fn delete_artists_cmd(
	state: State<'_, AppState>,
	uids: Vec<String>,
) -> Result<DeleteResult, String> {
	let profile_uid = state.get_uid();
	library_manager::run_blocking(move || delete_entities(&profile_uid, Kind::Artists, &uids, false))
		.await
}

fn ensure_writable(lib: &Library, what: &str) -> Result<(), String> {
	if lib.is_default || lib.has_write_permission {
		Ok(())
	} else {
		Err(format!("{} belongs to a read-only library", what))
	}
}

fn copy_image_columns(
	profile_uid: &str,
	table: &str,
	columns: &[&str],
	from_uid: &str,
	target: &Connection,
	target_uid: &str,
) -> Result<(), String> {
	let (from_conn, _) = library_manager::open_source_conn_for_entity(profile_uid, from_uid, table)
		.map_err(|e| e.to_string())?;
	let select = format!("SELECT {} FROM {} WHERE uid = ?1", columns.join(", "), table);
	let mut values: Vec<rusqlite::types::Value> = from_conn
		.query_row(&select, params![from_uid], |row| {
			(0..columns.len())
				.map(|i| row.get::<_, rusqlite::types::Value>(i))
				.collect::<rusqlite::Result<Vec<_>>>()
		})
		.map_err(|e| e.to_string())?;
	let assignments: Vec<String> = columns
		.iter()
		.enumerate()
		.map(|(i, c)| format!("{} = ?{}", c, i + 1))
		.collect();
	values.push(rusqlite::types::Value::Text(target_uid.to_string()));
	target
		.execute(
			&format!(
				"UPDATE {} SET {} WHERE uid = ?{}",
				table,
				assignments.join(", "),
				columns.len() + 1
			),
			rusqlite::params_from_iter(values.iter()),
		)
		.map_err(|e| e.to_string())?;
	Ok(())
}

fn merged_text(merged: &Connection, table: &str, column: &str, uid: &str) -> Option<String> {
	merged
		.query_row(
			&format!("SELECT {} FROM {} WHERE uid = ?1", column, table),
			params![uid],
			|r| r.get::<_, Option<String>>(0),
		)
		.ok()
		.flatten()
}

fn finish_delete(result: DeleteResult) -> Result<(), String> {
	if let Some(f) = result.failed.first() {
		return Err(f.error.clone());
	}
	if let Some(r) = result.read_only.first() {
		return Err(format!("{} belongs to a read-only library", r.uid));
	}
	Ok(())
}

fn merge_artists_blocking(
	profile_uid: &str,
	keep_uid: &str,
	drop_uids: &[String],
	update: &ArtistUpdate,
	image_from_uid: Option<&str>,
) -> Result<(), String> {
	if drop_uids.is_empty() || drop_uids.iter().any(|u| u == keep_uid) {
		return Err("Select a primary artist and at least one other artist to merge".to_string());
	}

	let merged = open_merged_conn(profile_uid);
	let keep_old_name = merged_text(&merged, "artists", "name", keep_uid)
		.ok_or_else(|| "Primary artist not found".to_string())?;
	let drop_names: Vec<String> = drop_uids
		.iter()
		.filter_map(|u| merged_text(&merged, "artists", "name", u))
		.collect();
	drop(merged);

	let new_name = update
		.name
		.clone()
		.filter(|n| !n.trim().is_empty())
		.unwrap_or_else(|| keep_old_name.clone());

	let (keep_conn, keep_lib) =
		library_manager::open_source_conn_for_entity(profile_uid, keep_uid, "artists")
			.map_err(|e| e.to_string())?;
	ensure_writable(&keep_lib, "The primary artist")?;
	for uid in drop_uids {
		let (_, lib) = library_manager::open_source_conn_for_entity(profile_uid, uid, "artists")
			.map_err(|e| e.to_string())?;
		ensure_writable(&lib, "A merged artist")?;
	}

	update_artist_by_uid(&keep_conn, keep_uid, update).map_err(|e| e.to_string())?;

	if let Some(from) = image_from_uid.filter(|u| *u != keep_uid) {
		copy_image_columns(
			profile_uid,
			"artists",
			&[
				"profile_art_blob",
				"profile_art_path",
				"profile_art_thumb",
				"banner_art_blob",
				"banner_art_path",
			],
			from,
			&keep_conn,
			keep_uid,
		)?;
	}
	library_manager::sync_artist(profile_uid, &keep_lib.uid, keep_uid)?;

	let mut old_names: Vec<String> = drop_names.clone();
	old_names.push(keep_old_name.clone());
	for name in old_names {
		if name.trim().to_lowercase() != new_name.trim().to_lowercase() {
			library_manager::rename_artist_everywhere(profile_uid, &name, &new_name)?;
		}
	}

	let settings = open_settings_conn(profile_uid);
	for uid in drop_uids {
		crate::db::settings_manager::add_uid_remap(&settings, uid, keep_uid, "artist")
			.map_err(|e| e.to_string())?;
	}

	finish_delete(delete_entities(profile_uid, Kind::Artists, drop_uids, false)?)
}

fn merge_albums_blocking(
	profile_uid: &str,
	keep_uid: &str,
	drop_uids: &[String],
	update: &AlbumUpdate,
	image_from_uid: Option<&str>,
) -> Result<(), String> {
	if drop_uids.is_empty() || drop_uids.iter().any(|u| u == keep_uid) {
		return Err("Select a primary album and at least one other album to merge".to_string());
	}

	let merged = open_merged_conn(profile_uid);
	let keep_old_title = merged_text(&merged, "albums", "title", keep_uid)
		.ok_or_else(|| "Primary album not found".to_string())?;
	let keep_old_artist = merged_text(&merged, "albums", "album_artist", keep_uid).unwrap_or_default();

	let mut union: Vec<serde_json::Value> = Vec::new();
	let mut seen: HashSet<String> = HashSet::new();
	let mut all_uids: Vec<&str> = vec![keep_uid];
	all_uids.extend(drop_uids.iter().map(|s| s.as_str()));
	for uid in all_uids {
		if let Some(json) = merged_text(&merged, "albums", "tracks", uid) {
			if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&json) {
				for entry in list {
					let tuid = entry_uid(&entry).to_string();
					if tuid.is_empty() || seen.insert(tuid) {
						union.push(entry);
					}
				}
			}
		}
	}
	drop(merged);

	let new_title = update
		.title
		.clone()
		.filter(|t| !t.trim().is_empty())
		.unwrap_or_else(|| keep_old_title.clone());

	let (keep_conn, keep_lib) =
		library_manager::open_source_conn_for_entity(profile_uid, keep_uid, "albums")
			.map_err(|e| e.to_string())?;
	ensure_writable(&keep_lib, "The primary album")?;
	for uid in drop_uids {
		let (_, lib) = library_manager::open_source_conn_for_entity(profile_uid, uid, "albums")
			.map_err(|e| e.to_string())?;
		ensure_writable(&lib, "A merged album")?;
	}

	for uid in drop_uids {
		library_manager::repoint_album_in_tracks(profile_uid, uid, keep_uid, &new_title)?;
	}

	let mut final_update = AlbumUpdate {
		format: update.format.clone(),
		title: update.title.clone(),
		rating: update.rating,
		artists: update.artists.clone(),
		album_artist: update.album_artist.clone(),
		release_date: update.release_date.clone(),
		tags: update.tags.clone(),
		genres: update.genres.clone(),
		tracks: None,
		credits: update.credits.clone(),
		label: update.label.clone(),
		artwork_blob: None,
		artwork_path: update.artwork_path.clone(),
		emulate_type: update.emulate_type.clone(),
	};
	final_update.tracks = Some(serde_json::to_string(&union).map_err(|e| e.to_string())?);

	update_album_by_uid(&keep_conn, keep_uid, &final_update).map_err(|e| e.to_string())?;

	if let Some(from) = image_from_uid.filter(|u| *u != keep_uid) {
		copy_image_columns(
			profile_uid,
			"albums",
			&["artwork_blob", "artwork_path", "artwork_thumb"],
			from,
			&keep_conn,
			keep_uid,
		)?;
	}
	library_manager::sync_album(profile_uid, &keep_lib.uid, keep_uid)?;

	if new_title.trim().to_lowercase() != keep_old_title.trim().to_lowercase() {
		let artist = update.album_artist.clone().unwrap_or(keep_old_artist);
		library_manager::rename_album_everywhere(
			profile_uid,
			&keep_old_title,
			&new_title,
			&artist,
			Some(keep_uid),
		)?;
	}

	let settings = open_settings_conn(profile_uid);
	for uid in drop_uids {
		crate::db::settings_manager::add_uid_remap(&settings, uid, keep_uid, "album")
			.map_err(|e| e.to_string())?;
	}

	finish_delete(delete_entities(profile_uid, Kind::Albums, drop_uids, false)?)
}

#[tauri::command]
pub async fn merge_artists_cmd(
	state: State<'_, AppState>,
	keep_uid: String,
	drop_uids: Vec<String>,
	update: ArtistUpdate,
	image_from_uid: Option<String>,
) -> Result<(), String> {
	let profile_uid = state.get_uid();
	library_manager::run_blocking(move || {
		merge_artists_blocking(
			&profile_uid,
			&keep_uid,
			&drop_uids,
			&update,
			image_from_uid.as_deref(),
		)
	})
	.await
}

#[tauri::command]
pub async fn merge_albums_cmd(
	state: State<'_, AppState>,
	keep_uid: String,
	drop_uids: Vec<String>,
	update: AlbumUpdate,
	image_from_uid: Option<String>,
) -> Result<(), String> {
	let profile_uid = state.get_uid();
	library_manager::run_blocking(move || {
		merge_albums_blocking(
			&profile_uid,
			&keep_uid,
			&drop_uids,
			&update,
			image_from_uid.as_deref(),
		)
	})
	.await
}

fn text_missing(v: &Option<String>) -> bool {
	v.as_deref()
		.map(|s| s.trim().is_empty() || s.trim() == "[]")
		.unwrap_or(true)
}

fn union_strings(a: &Option<String>, b: &Option<String>) -> Option<String> {
	let mut out: Vec<String> = a
		.as_deref()
		.and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
		.unwrap_or_default();
	if let Some(extra) = b
		.as_deref()
		.and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
	{
		for item in extra {
			if !out.iter().any(|e| e.to_lowercase() == item.to_lowercase()) {
				out.push(item);
			}
		}
	}
	if out.is_empty() {
		None
	} else {
		serde_json::to_string(&out).ok()
	}
}

fn union_album_entries(a: &Option<String>, b: &Option<String>) -> Option<String> {
	let mut out: Vec<serde_json::Value> = a
		.as_deref()
		.and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
		.unwrap_or_default();
	if let Some(extra) = b
		.as_deref()
		.and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
	{
		for item in extra {
			let uid = entry_uid(&item).to_string();
			let name = item
				.get("name")
				.and_then(|n| n.as_str())
				.unwrap_or("")
				.to_lowercase();
			let exists = out.iter().any(|e| {
				let eu = entry_uid(e);
				if !uid.is_empty() && !eu.is_empty() {
					eu == uid
				} else {
					e.get("name")
						.and_then(|n| n.as_str())
						.map(|n| n.to_lowercase() == name)
						.unwrap_or(false)
				}
			});
			if !exists {
				out.push(item);
			}
		}
	}
	if out.is_empty() {
		None
	} else {
		serde_json::to_string(&out).ok()
	}
}

fn repoint_track_in_playlists(
	profile_uid: &str,
	from_uid: &str,
	to_uid: &str,
	to_name: &str,
) -> Result<(), String> {
	let path = crate::profiles::get_playlists_db_path(profile_uid);
	if !path.exists() {
		return Ok(());
	}
	let conn = library_manager::open_tuned(&path)?;
	let pattern = format!("%{}%", from_uid);

	let rows: Vec<(String, String)> = {
		let mut stmt = conn
			.prepare("SELECT uid, tracks FROM playlists WHERE tracks LIKE ?1")
			.map_err(|e| e.to_string())?;
		let collected: Vec<(String, String)> = stmt
			.query_map(params![pattern], |r| Ok((r.get(0)?, r.get(1)?)))
			.map_err(|e| e.to_string())?
			.filter_map(|r| r.ok())
			.collect();
		collected
	};

	let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
	for (playlist_uid, json) in rows {
		let Some(list) = parse_list(&json) else { continue };
		if !list.iter().any(|e| entry_uid(e) == from_uid) {
			continue;
		}

		let mut seen_target = false;
		let mut rebuilt: Vec<serde_json::Value> = Vec::new();
		for mut entry in list {
			let mut uid = entry_uid(&entry).to_string();
			if uid == from_uid {
				if let Some(obj) = entry.as_object_mut() {
					obj.insert("uid".to_string(), serde_json::Value::String(to_uid.to_string()));
					obj.insert("name".to_string(), serde_json::Value::String(to_name.to_string()));
				}
				uid = to_uid.to_string();
			}
			if uid == to_uid {
				if seen_target {
					continue;
				}
				seen_target = true;
			}
			rebuilt.push(entry);
		}
		for (i, entry) in rebuilt.iter_mut().enumerate() {
			if let Some(obj) = entry.as_object_mut() {
				obj.insert("order".to_string(), serde_json::json!(i + 1));
			}
		}

		let out = serde_json::to_string(&rebuilt).map_err(|e| e.to_string())?;
		conn.execute(
			"UPDATE playlists SET tracks = ?1 WHERE uid = ?2",
			params![out, playlist_uid],
		)
		.map_err(|e| e.to_string())?;
	}
	tx.commit().map_err(|e| e.to_string())?;
	Ok(())
}

fn merge_tracks_blocking(
	profile_uid: &str,
	keep_uid: &str,
	drop_uids: &[String],
) -> Result<(), String> {
	if drop_uids.is_empty() || drop_uids.iter().any(|u| u == keep_uid) {
		return Err("Choose one track to keep and at least one other track to merge".to_string());
	}

	let merged = open_merged_conn(profile_uid);
	let keep = crate::db::get_track_by_uid(&merged, keep_uid)
		.map_err(|e| e.to_string())?
		.ok_or_else(|| "The track to keep was not found".to_string())?;
	let mut drops = Vec::new();
	for uid in drop_uids {
		if let Some(t) = crate::db::get_track_by_uid(&merged, uid).map_err(|e| e.to_string())? {
			drops.push(t);
		}
	}
	drop(merged);

	let (keep_conn, keep_lib) =
		library_manager::open_source_conn_for_entity(profile_uid, keep_uid, "tracks")
			.map_err(|e| e.to_string())?;
	ensure_writable(&keep_lib, "The track to keep")?;
	for uid in drop_uids {
		let (_, lib) = library_manager::open_source_conn_for_entity(profile_uid, uid, "tracks")
			.map_err(|e| e.to_string())?;
		ensure_writable(&lib, "A merged track")?;
	}

	let keep_had_location = !keep.path.trim().is_empty()
		|| keep
			.remote_path
			.as_deref()
			.map(|r| !r.trim().is_empty())
			.unwrap_or(false);
	let mut gained_location = false;

	let fill_text = |col: &str, value: &Option<String>| -> Result<bool, String> {
		let Some(v) = value.as_deref().filter(|v| !v.trim().is_empty() && v.trim() != "[]") else {
			return Ok(false);
		};
		let changed = keep_conn
			.execute(
				&format!(
					"UPDATE tracks SET {c} = ?1 WHERE uid = ?2 AND ({c} IS NULL OR TRIM({c}) = '' OR TRIM({c}) = '[]')",
					c = col
				),
				params![v, keep_uid],
			)
			.map_err(|e| e.to_string())?;
		Ok(changed > 0)
	};
	let fill_number = |col: &str, value: Option<f64>| -> Result<(), String> {
		let Some(v) = value.filter(|v| *v != 0.0) else {
			return Ok(());
		};
		keep_conn
			.execute(
				&format!(
					"UPDATE tracks SET {c} = ?1 WHERE uid = ?2 AND ({c} IS NULL OR {c} = 0)",
					c = col
				),
				params![v, keep_uid],
			)
			.map_err(|e| e.to_string())?;
		Ok(())
	};

	let mut tags = keep.tags.clone();
	let mut genres = keep.genres.clone();
	let mut albums = keep.albums.clone();
	let (mut keep_locals, mut keep_remotes) = crate::db::track_paths::all_entries(&keep);

	for d in &drops {
		let (drop_locals, drop_remotes) = crate::db::track_paths::all_entries(d);
		keep_locals.extend(drop_locals);
		keep_remotes.extend(drop_remotes);
		for (col, value) in [
			("remote_data", &d.remote_data),
			("format", &d.format),
			("year", &d.year),
			("key", &d.key),
			("credits", &d.credits),
			("label", &d.label),
			("album_artist", &d.album_artist),
			("artists", &d.artists),
			("user_options", &d.user_options),
		] {
			fill_text(col, value)?;
		}
		fill_number("bitrate", d.bitrate.map(|v| v as f64))?;
		fill_number("duration_ms", d.duration_ms.map(|v| v as f64))?;
		fill_number("bpm", d.bpm.map(|v| v as f64))?;
		fill_number("rating", d.rating.map(|v| v as f64))?;

		tags = union_strings(&tags, &d.tags);
		genres = union_strings(&genres, &d.genres);
		albums = union_album_entries(&albums, &d.albums);
	}

	gained_location = !keep_had_location && (!keep_locals.is_empty() || !keep_remotes.is_empty());
	crate::db::track_paths::write_entries(&keep_conn, keep_uid, keep_locals, keep_remotes)
		.map_err(|e| e.to_string())?;

	keep_conn
		.execute(
			"UPDATE tracks SET tags = COALESCE(?1, tags), genres = COALESCE(?2, genres), albums = COALESCE(?3, albums) WHERE uid = ?4",
			params![tags, genres, albums, keep_uid],
		)
		.map_err(|e| e.to_string())?;

	let keep_has_art = keep.artwork_thumb.as_deref().map(|t| !t.is_empty()).unwrap_or(false)
		|| keep.artwork_path.as_deref().map(|p| !p.is_empty()).unwrap_or(false);
	if !keep_has_art {
		if let Some(donor) = drops.iter().find(|d| {
			d.artwork_thumb.as_deref().map(|t| !t.is_empty()).unwrap_or(false)
				|| d.artwork_path.as_deref().map(|p| !p.is_empty()).unwrap_or(false)
		}) {
			copy_image_columns(
				profile_uid,
				"tracks",
				&["artwork_blob", "artwork_path", "artwork_thumb"],
				&donor.uid,
				&keep_conn,
				keep_uid,
			)?;
		}
	}

	if gained_location && !keep_had_location {
		let current: Option<String> = keep_conn
			.query_row(
				"SELECT track_data FROM tracks WHERE uid = ?1",
				params![keep_uid],
				|r| r.get(0),
			)
			.ok()
			.flatten();
		if let Some(mut value) = current
			.as_deref()
			.and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
		{
			if let Some(obj) = value.as_object_mut() {
				if obj.contains_key("is_ghost") {
					obj.insert("is_ghost".to_string(), serde_json::Value::Bool(false));
					if let Ok(out) = serde_json::to_string(&value) {
						keep_conn
							.execute(
								"UPDATE tracks SET track_data = ?1 WHERE uid = ?2",
								params![out, keep_uid],
							)
							.map_err(|e| e.to_string())?;
					}
				}
			}
		}
	}

	library_manager::sync_track(profile_uid, &keep_lib.uid, keep_uid)?;

	let keep_title = keep.title.clone().unwrap_or_default();
	for uid in drop_uids {
		library_manager::repoint_track_in_albums(profile_uid, uid, keep_uid, &keep_title)?;
		repoint_track_in_playlists(profile_uid, uid, keep_uid, &keep_title)?;
	}

	{
		let analytics = crate::open_analytics_conn(profile_uid).map_err(|e| e.to_string())?;
		for uid in drop_uids {
			analytics
				.execute(
					"UPDATE scrobbles SET track_uid = ?1 WHERE track_uid = ?2",
					params![keep_uid, uid],
				)
				.map_err(|e| e.to_string())?;
		}
	}

	let settings = open_settings_conn(profile_uid);
	for uid in drop_uids {
		crate::db::settings_manager::add_uid_remap(&settings, uid, keep_uid, "track")
			.map_err(|e| e.to_string())?;
	}

	finish_delete(delete_entities(profile_uid, Kind::Tracks, drop_uids, false)?)
}

#[tauri::command]
pub async fn merge_tracks_cmd(
	state: State<'_, AppState>,
	keep_uid: String,
	drop_uids: Vec<String>,
) -> Result<(), String> {
	let profile_uid = state.get_uid();
	library_manager::run_blocking(move || merge_tracks_blocking(&profile_uid, &keep_uid, &drop_uids))
		.await
}
