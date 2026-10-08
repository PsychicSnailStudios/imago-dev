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
