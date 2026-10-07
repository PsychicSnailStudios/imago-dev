// src-tauri/src/library_manager.rs
//
// Federated library merge engine.
//
// Responsibilities:
//   - On load: detect which library dbs have changed via data_version pragma
//   - If nothing changed: reuse existing merged.db
//   - If libraries changed: incremental update of only those libraries
//   - Full rebuild: triggered after import, remote pull, or manual request
//   - Write routing: resolve source_lib_uid from merged.db, write to correct source,
//     attempt push to remote if applicable, then update merged cache
//   - Sync: pull latest from remote if sidecar reports newer last_modified
//   - Push: upload local db file to sync_url using write_token

use crate::db::blocklist_manager::get_blocked_uid_set;
use crate::db::library_registry::{
    detect_changed_libraries, get_all_libraries, get_default_library, get_library_by_uid,
    read_data_version, store_data_version, Library, LibraryUpdate,
};
use crate::db::{init_library_db, init_merged_db};
use crate::profiles::{
    get_libraries_dir, get_library_db_path, get_local_library_db_path, get_merged_db_path,
    get_settings_db_path,
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashSet;

// ─── Public result type ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub rebuilt: bool,
    pub libraries_processed: usize,
}

// ─── Sidecar format ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RemoteSidecar {
    last_modified: i64,
    #[allow(dead_code)]
    write_requires_token: bool,
}

// ─── Connection helpers ───────────────────────────────────────────────────────

pub fn tune_conn(conn: &Connection) {
	let _ = conn.execute_batch(
		"PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA busy_timeout = 8000;",
	);
}

pub fn open_tuned<P: AsRef<std::path::Path>>(path: P) -> Result<Connection, String> {
	let conn = Connection::open(path.as_ref()).map_err(|e| e.to_string())?;
	tune_conn(&conn);
	Ok(conn)
}

pub fn remove_wal_files(db_path: &str) {
	for ext in ["-wal", "-shm"] {
		let p = format!("{}{}", db_path, ext);
		if std::path::Path::new(&p).exists() {
			let _ = std::fs::remove_file(&p);
		}
	}
}

pub fn checkpoint_db(db_path: &str) {
	if let Ok(conn) = Connection::open(db_path) {
		let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
	}
}

pub async fn run_blocking<T, F>(f: F) -> Result<T, String>
where
	F: FnOnce() -> Result<T, String> + Send + 'static,
	T: Send + 'static,
{
	match tauri::async_runtime::spawn_blocking(f).await {
		Ok(r) => r,
		Err(e) => Err(e.to_string()),
	}
}

pub fn open_library_conn(profile_uid: &str, lib_uid: &str) -> Result<Connection, String> {
    let path = get_library_db_path(profile_uid, lib_uid);
    let conn = open_tuned(&path).map_err(|e| e.to_string())?;
    init_library_db(&conn).map_err(|e| e.to_string())?;
    Ok(conn)
}

pub fn open_local_library_conn(profile_uid: &str) -> Result<Connection, String> {
    let path = get_local_library_db_path(profile_uid);
    let conn = open_tuned(&path).map_err(|e| e.to_string())?;
    init_library_db(&conn).map_err(|e| e.to_string())?;
    Ok(conn)
}

pub fn open_merged_conn(profile_uid: &str) -> Result<Connection, String> {
    let path = get_merged_db_path(profile_uid);
    let conn = open_tuned(&path).map_err(|e| e.to_string())?;
    Ok(conn)
}

// ─── Default library bootstrap ───────────────────────────────────────────────

// Called when a profile is first set up or when the libraries table is empty.
// Creates the local.db file and registers it as the default library.
pub fn ensure_default_library(profile_uid: &str) -> Result<String, String> {
    let settings_conn =
        open_tuned(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

    if let Some(existing) = get_default_library(&settings_conn).map_err(|e| e.to_string())? {
        return Ok(existing.uid);
    }

    let lib_uid = crate::db::generate_uid("lib");
    let file_path = get_local_library_db_path(profile_uid);

    let lib_conn = open_tuned(&file_path).map_err(|e| e.to_string())?;
    init_library_db(&lib_conn).map_err(|e| e.to_string())?;

    let library = Library {
        uid: lib_uid.clone(),
        name: "Local".to_string(),
        is_default: true,
        file_path: file_path.to_string_lossy().to_string(),
        sync_url: None,
        sync_meta_url: None,
        write_token: None,
        has_write_permission: true,
        last_synced: 0,
        last_data_version: 0,
        sort_order: 999,
    };

    crate::db::library_registry::create_library(&settings_conn, &library)
        .map_err(|e| e.to_string())?;

    Ok(lib_uid)
}

// ─── On-load entry point ──────────────────────────────────────────────────────

// Called on app load. Checks data_version on all libraries and decides
// whether to skip, incrementally update, or fully rebuild merged.db.
pub fn on_load_sync(profile_uid: &str) -> Result<MergeResult, String> {
    let settings_conn =
        open_tuned(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

    let changed = detect_changed_libraries(&settings_conn).map_err(|e| e.to_string())?;

    if changed.is_empty() {
        let merged_path = get_merged_db_path(profile_uid);
        if merged_path.exists() {
            return Ok(MergeResult {
                rebuilt: false,
                libraries_processed: 0,
            });
        }
        // merged.db is missing even though nothing changed — full rebuild
        return full_rebuild(profile_uid);
    }

    let count = changed.len();
    incremental_update(profile_uid, &changed)?;

    Ok(MergeResult {
        rebuilt: true,
        libraries_processed: count,
    })
}

// ─── Full rebuild ─────────────────────────────────────────────────────────────

fn reset_merged_file(merged_path: &std::path::Path) -> Result<(), String> {
	let base = merged_path.to_string_lossy().to_string();
	let mut main_removed = true;
	if merged_path.exists() {
		main_removed = std::fs::remove_file(merged_path).is_ok();
	}
	if main_removed {
		remove_wal_files(&base);
	}
	Ok(())
}

fn clear_merged_tables(merged: &Connection) -> Result<(), String> {
	for table in ["lyrics", "tracks", "albums", "artists"] {
		merged
			.execute(&format!("DELETE FROM {}", table), [])
			.map_err(|e| e.to_string())?;
	}
	Ok(())
}

pub fn full_rebuild(profile_uid: &str) -> Result<MergeResult, String> {
	let merged_path = get_merged_db_path(profile_uid);

	reset_merged_file(&merged_path)?;

	let merged_conn = open_tuned(&merged_path)?;
	init_merged_db(&merged_conn).map_err(|e| e.to_string())?;
	clear_merged_tables(&merged_conn)?;

	let settings_conn = open_tuned(get_settings_db_path(profile_uid))?;

	let blocklist = get_blocked_uid_set(&settings_conn).map_err(|e| e.to_string())?;
	let libraries = get_all_libraries(&settings_conn).map_err(|e| e.to_string())?;
	let default_lib_uid = get_default_library(&settings_conn)
		.map_err(|e| e.to_string())?
		.map(|l| l.uid)
		.unwrap_or_default();

	let count = libraries.len();

	for lib in &libraries {
		if !std::path::Path::new(&lib.file_path).exists() {
			eprintln!(
				"[library_manager] Skipping missing library file: {}",
				lib.file_path
			);
			continue;
		}

		let source_conn = open_tuned(&lib.file_path)?;
		{
			let tx = merged_conn.unchecked_transaction().map_err(|e| e.to_string())?;
			copy_library_into_merged(&source_conn, &merged_conn, &lib.uid, &blocklist)?;
			tx.commit().map_err(|e| e.to_string())?;
		}

		let current_version = read_data_version(&lib.file_path);
		store_data_version(&settings_conn, &lib.uid, current_version).map_err(|e| e.to_string())?;
	}

	refresh_local_override_flags(&merged_conn, &default_lib_uid)?;

	Ok(MergeResult {
		rebuilt: true,
		libraries_processed: count,
	})
}

// ─── Incremental update (whole library) ───────────────────────────────────────

pub fn incremental_update(profile_uid: &str, changed_lib_uids: &[String]) -> Result<(), String> {
	let merged_path = get_merged_db_path(profile_uid);
	if !merged_path.exists() {
		full_rebuild(profile_uid)?;
		return Ok(());
	}

	let merged_conn = open_tuned(&merged_path)?;
	let settings_conn = open_tuned(get_settings_db_path(profile_uid))?;

	let blocklist = get_blocked_uid_set(&settings_conn).map_err(|e| e.to_string())?;
	let all_libraries = get_all_libraries(&settings_conn).map_err(|e| e.to_string())?;
	let default_lib_uid = get_default_library(&settings_conn)
		.map_err(|e| e.to_string())?
		.map(|l| l.uid)
		.unwrap_or_default();

	for lib_uid in changed_lib_uids {
		let lib = match all_libraries.iter().find(|l| &l.uid == lib_uid) {
			Some(l) => l,
			None => continue,
		};

		if !std::path::Path::new(&lib.file_path).exists() {
			remove_library_from_merged(&merged_conn, lib_uid)?;
			continue;
		}

		let source_conn = open_tuned(&lib.file_path)?;
		{
			let tx = merged_conn.unchecked_transaction().map_err(|e| e.to_string())?;
			copy_library_into_merged(&source_conn, &merged_conn, lib_uid, &blocklist)?;
			cleanup_deleted_records(&source_conn, &merged_conn, lib_uid)?;
			tx.commit().map_err(|e| e.to_string())?;
		}

		let current_version = read_data_version(&lib.file_path);
		store_data_version(&settings_conn, lib_uid, current_version).map_err(|e| e.to_string())?;
	}

	refresh_local_override_flags(&merged_conn, &default_lib_uid)?;

	Ok(())
}

// ─── Targeted sync (specific records only) ────────────────────────────────────

pub fn sync_entities(
	profile_uid: &str,
	lib_uid: &str,
	track_uids: &[String],
	album_uids: &[String],
	artist_uids: &[String],
) -> Result<(), String> {
	if track_uids.is_empty() && album_uids.is_empty() && artist_uids.is_empty() {
		return Ok(());
	}

	let merged_path = get_merged_db_path(profile_uid);
	if !merged_path.exists() {
		full_rebuild(profile_uid)?;
		return Ok(());
	}

	let merged_conn = open_tuned(&merged_path)?;
	let settings_conn = open_tuned(get_settings_db_path(profile_uid))?;

	let blocklist = get_blocked_uid_set(&settings_conn).map_err(|e| e.to_string())?;
	let lib = get_library_by_uid(&settings_conn, lib_uid)
		.map_err(|e| e.to_string())?
		.ok_or_else(|| format!("Library not found: {}", lib_uid))?;
	let default_lib_uid = get_default_library(&settings_conn)
		.map_err(|e| e.to_string())?
		.map(|l| l.uid)
		.unwrap_or_default();

	if !std::path::Path::new(&lib.file_path).exists() {
		remove_library_from_merged(&merged_conn, lib_uid)?;
		return Ok(());
	}

	let source_conn = open_tuned(&lib.file_path)?;

	{
		let tx = merged_conn.unchecked_transaction().map_err(|e| e.to_string())?;

		if !track_uids.is_empty() {
			delete_merged_lyrics_for(&merged_conn, track_uids)?;
			copy_tracks(&source_conn, &merged_conn, lib_uid, &blocklist, Some(track_uids))
				.map_err(|e| e.to_string())?;
			copy_lyrics(&source_conn, &merged_conn, lib_uid, Some(track_uids))
				.map_err(|e| e.to_string())?;
			remove_missing_records(&source_conn, &merged_conn, lib_uid, "tracks", track_uids)?;
			refresh_flags_for(&merged_conn, &default_lib_uid, "tracks", track_uids)?;
		}
		if !album_uids.is_empty() {
			copy_albums(&source_conn, &merged_conn, lib_uid, &blocklist, Some(album_uids))
				.map_err(|e| e.to_string())?;
			remove_missing_records(&source_conn, &merged_conn, lib_uid, "albums", album_uids)?;
			refresh_flags_for(&merged_conn, &default_lib_uid, "albums", album_uids)?;
		}
		if !artist_uids.is_empty() {
			copy_artists(&source_conn, &merged_conn, lib_uid, &blocklist, Some(artist_uids))
				.map_err(|e| e.to_string())?;
			remove_missing_records(&source_conn, &merged_conn, lib_uid, "artists", artist_uids)?;
			refresh_flags_for(&merged_conn, &default_lib_uid, "artists", artist_uids)?;
		}

		tx.commit().map_err(|e| e.to_string())?;
	}

	let current_version = read_data_version(&lib.file_path);
	store_data_version(&settings_conn, lib_uid, current_version).map_err(|e| e.to_string())?;

	Ok(())
}

pub fn sync_track(profile_uid: &str, lib_uid: &str, uid: &str) -> Result<(), String> {
	sync_entities(profile_uid, lib_uid, &[uid.to_string()], &[], &[])
}

pub fn sync_album(profile_uid: &str, lib_uid: &str, uid: &str) -> Result<(), String> {
	sync_entities(profile_uid, lib_uid, &[], &[uid.to_string()], &[])
}

pub fn sync_artist(profile_uid: &str, lib_uid: &str, uid: &str) -> Result<(), String> {
	sync_entities(profile_uid, lib_uid, &[], &[], &[uid.to_string()])
}

fn delete_merged_lyrics_for(merged: &Connection, track_uids: &[String]) -> Result<(), String> {
	let mut stmt = merged
		.prepare_cached("DELETE FROM lyrics WHERE track_id IN (SELECT id FROM tracks WHERE uid = ?1)")
		.map_err(|e| e.to_string())?;
	for uid in track_uids {
		stmt.execute(params![uid]).map_err(|e| e.to_string())?;
	}
	Ok(())
}

fn remove_missing_records(
	source: &Connection,
	merged: &Connection,
	lib_uid: &str,
	table: &str,
	uids: &[String],
) -> Result<(), String> {
	let mut exists = source
		.prepare_cached(&format!("SELECT 1 FROM {} WHERE uid = ?1", table))
		.map_err(|e| e.to_string())?;
	let mut remove = merged
		.prepare_cached(&format!(
			"DELETE FROM {} WHERE uid = ?1 AND source_lib_uid = ?2",
			table
		))
		.map_err(|e| e.to_string())?;
	for uid in uids {
		let present = exists
			.exists(params![uid])
			.map_err(|e| e.to_string())?;
		if !present {
			if table == "tracks" {
				delete_merged_lyrics_for(merged, std::slice::from_ref(uid))?;
			}
			remove
				.execute(params![uid, lib_uid])
				.map_err(|e| e.to_string())?;
		}
	}
	Ok(())
}

fn refresh_flags_for(
	merged: &Connection,
	default_lib_uid: &str,
	table: &str,
	uids: &[String],
) -> Result<(), String> {
	let mut stmt = merged
		.prepare_cached(&format!(
			"UPDATE {} SET is_local_override = CASE WHEN source_lib_uid = ?1 THEN 1 ELSE 0 END WHERE uid = ?2",
			table
		))
		.map_err(|e| e.to_string())?;
	for uid in uids {
		stmt.execute(params![default_lib_uid, uid])
			.map_err(|e| e.to_string())?;
	}
	Ok(())
}

// ─── Copy one library into merged ─────────────────────────────────────────────

fn copy_library_into_merged(
	source: &Connection,
	merged: &Connection,
	lib_uid: &str,
	blocklist: &HashSet<String>,
) -> Result<(), String> {
	copy_tracks(source, merged, lib_uid, blocklist, None).map_err(|e| e.to_string())?;
	copy_albums(source, merged, lib_uid, blocklist, None).map_err(|e| e.to_string())?;
	copy_artists(source, merged, lib_uid, blocklist, None).map_err(|e| e.to_string())?;
	copy_lyrics(source, merged, lib_uid, None).map_err(|e| e.to_string())?;
	Ok(())
}

fn for_each_chunk<F>(filter: Option<&[String]>, column: &str, mut f: F) -> rusqlite::Result<()>
where
	F: FnMut(&str, &[String]) -> rusqlite::Result<()>,
{
	match filter {
		None => f("", &[]),
		Some(uids) => {
			for chunk in uids.chunks(400) {
				let placeholders = vec!["?"; chunk.len()].join(",");
				let clause = format!(" WHERE {} IN ({})", column, placeholders);
				f(&clause, chunk)?;
			}
			Ok(())
		}
	}
}

fn read_row_values(
	row: &rusqlite::Row,
	count: usize,
) -> rusqlite::Result<Vec<rusqlite::types::Value>> {
	let mut vals = Vec::with_capacity(count + 1);
	for i in 0..count {
		vals.push(row.get::<_, rusqlite::types::Value>(i)?);
	}
	Ok(vals)
}

fn copy_tracks(
	source: &Connection,
	merged: &Connection,
	lib_uid: &str,
	blocklist: &HashSet<String>,
	filter: Option<&[String]>,
) -> rusqlite::Result<()> {
	for_each_chunk(filter, "uid", |clause, args| {
		let sql = format!(
			"SELECT uid, path, last_modified, title, artists, album_artist, albums, genres, year,
			rating, tags, duration_ms, bpm, key, credits, label, artwork_blob, artwork_path,
			artwork_thumb, user_options, format, bitrate, remote_path, remote_data, track_data, date_added
			FROM tracks{}",
			clause
		);
		let mut stmt = source.prepare(&sql)?;
		let mut rows = stmt.query(rusqlite::params_from_iter(args.iter()))?;
		let mut insert = merged.prepare_cached(
			"INSERT INTO tracks (
				uid, path, last_modified, title, artists, album_artist, albums, genres, year,
				rating, tags, duration_ms, bpm, key, credits, label, artwork_blob, artwork_path,
				artwork_thumb, user_options, format, bitrate, remote_path, remote_data, track_data,
				date_added, source_lib_uid
			) VALUES (
				?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,COALESCE(?26, 0),?27
			)
			ON CONFLICT(uid) DO UPDATE SET
				path           = excluded.path,
				last_modified  = excluded.last_modified,
				title          = excluded.title,
				artists        = excluded.artists,
				album_artist   = excluded.album_artist,
				albums         = excluded.albums,
				genres         = excluded.genres,
				year           = excluded.year,
				rating         = excluded.rating,
				tags           = excluded.tags,
				duration_ms    = excluded.duration_ms,
				bpm            = excluded.bpm,
				key            = excluded.key,
				credits        = excluded.credits,
				label          = excluded.label,
				artwork_blob   = excluded.artwork_blob,
				artwork_path   = excluded.artwork_path,
				artwork_thumb  = excluded.artwork_thumb,
				user_options   = excluded.user_options,
				format         = excluded.format,
				bitrate        = excluded.bitrate,
				remote_path    = excluded.remote_path,
				remote_data    = excluded.remote_data,
				track_data     = excluded.track_data,
				date_added     = excluded.date_added,
				source_lib_uid = excluded.source_lib_uid",
		)?;
		while let Some(row) = rows.next()? {
			let mut vals = read_row_values(row, 26)?;
			if let rusqlite::types::Value::Text(ref uid) = vals[0] {
				if blocklist.contains(uid) {
					continue;
				}
			}
			vals.push(rusqlite::types::Value::Text(lib_uid.to_string()));
			insert.execute(rusqlite::params_from_iter(vals.iter()))?;
		}
		Ok(())
	})
}

fn copy_albums(
	source: &Connection,
	merged: &Connection,
	lib_uid: &str,
	blocklist: &HashSet<String>,
	filter: Option<&[String]>,
) -> rusqlite::Result<()> {
	for_each_chunk(filter, "uid", |clause, args| {
		let sql = format!(
			"SELECT uid, format, title, rating, artists, album_artist, release_date,
			tags, genres, tracks, credits, label, artwork_blob, artwork_path, artwork_thumb, emulate_type
			FROM albums{}",
			clause
		);
		let mut stmt = source.prepare(&sql)?;
		let mut rows = stmt.query(rusqlite::params_from_iter(args.iter()))?;
		let mut insert = merged.prepare_cached(
			"INSERT INTO albums (
				uid, format, title, rating, artists, album_artist, release_date,
				tags, genres, tracks, credits, label, artwork_blob, artwork_path,
				artwork_thumb, emulate_type, source_lib_uid
			) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
			ON CONFLICT(uid) DO UPDATE SET
				format         = excluded.format,
				title          = excluded.title,
				rating         = excluded.rating,
				artists        = excluded.artists,
				album_artist   = excluded.album_artist,
				release_date   = excluded.release_date,
				tags           = excluded.tags,
				genres         = excluded.genres,
				tracks         = excluded.tracks,
				credits        = excluded.credits,
				label          = excluded.label,
				artwork_blob   = excluded.artwork_blob,
				artwork_path   = excluded.artwork_path,
				artwork_thumb  = excluded.artwork_thumb,
				emulate_type   = excluded.emulate_type,
				source_lib_uid = excluded.source_lib_uid",
		)?;
		while let Some(row) = rows.next()? {
			let mut vals = read_row_values(row, 16)?;
			if let rusqlite::types::Value::Text(ref uid) = vals[0] {
				if blocklist.contains(uid) {
					continue;
				}
			}
			vals.push(rusqlite::types::Value::Text(lib_uid.to_string()));
			insert.execute(rusqlite::params_from_iter(vals.iter()))?;
		}
		Ok(())
	})
}

fn copy_artists(
	source: &Connection,
	merged: &Connection,
	lib_uid: &str,
	blocklist: &HashSet<String>,
	filter: Option<&[String]>,
) -> rusqlite::Result<()> {
	for_each_chunk(filter, "uid", |clause, args| {
		let sql = format!(
			"SELECT uid, name, aka, about, tags, genres, websites, members,
			profile_art_blob, profile_art_path, profile_art_thumb, banner_art_blob, banner_art_path
			FROM artists{}",
			clause
		);
		let mut stmt = source.prepare(&sql)?;
		let mut rows = stmt.query(rusqlite::params_from_iter(args.iter()))?;
		let mut insert = merged.prepare_cached(
			"INSERT INTO artists (
				uid, name, aka, about, tags, genres, websites, members,
				profile_art_blob, profile_art_path, profile_art_thumb,
				banner_art_blob, banner_art_path, source_lib_uid
			) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)
			ON CONFLICT(uid) DO UPDATE SET
				name              = excluded.name,
				aka               = excluded.aka,
				about             = excluded.about,
				tags              = excluded.tags,
				genres            = excluded.genres,
				websites          = excluded.websites,
				members           = excluded.members,
				profile_art_blob  = excluded.profile_art_blob,
				profile_art_path  = excluded.profile_art_path,
				profile_art_thumb = excluded.profile_art_thumb,
				banner_art_blob   = excluded.banner_art_blob,
				banner_art_path   = excluded.banner_art_path,
				source_lib_uid    = excluded.source_lib_uid",
		)?;
		while let Some(row) = rows.next()? {
			let mut vals = read_row_values(row, 13)?;
			if let rusqlite::types::Value::Text(ref uid) = vals[0] {
				if blocklist.contains(uid) {
					continue;
				}
			}
			vals.push(rusqlite::types::Value::Text(lib_uid.to_string()));
			insert.execute(rusqlite::params_from_iter(vals.iter()))?;
		}
		Ok(())
	})
}

fn copy_lyrics(
	source: &Connection,
	merged: &Connection,
	lib_uid: &str,
	filter: Option<&[String]>,
) -> rusqlite::Result<()> {
	for_each_chunk(filter, "t.uid", |clause, args| {
		let sql = format!(
			"SELECT t.uid, l.source, l.plain, l.synced, l.instrumental
			FROM lyrics l
			JOIN tracks t ON t.id = l.track_id{}",
			clause
		);
		let mut stmt = source.prepare(&sql)?;
		let mut rows = stmt.query(rusqlite::params_from_iter(args.iter()))?;
		let mut find_id = merged.prepare_cached("SELECT id FROM tracks WHERE uid = ?1")?;
		let mut insert = merged.prepare_cached(
			"INSERT INTO lyrics (track_id, source, plain, synced, instrumental, source_lib_uid)
			VALUES (?1,?2,?3,?4,?5,?6)
			ON CONFLICT(track_id) DO UPDATE SET
				source         = excluded.source,
				plain          = excluded.plain,
				synced         = excluded.synced,
				instrumental   = excluded.instrumental,
				source_lib_uid = excluded.source_lib_uid",
		)?;
		while let Some(row) = rows.next()? {
			let track_uid: String = row.get(0)?;
			let merged_track_id: Option<i64> = find_id
				.query_row(params![track_uid], |r| r.get(0))
				.ok();
			if let Some(track_id) = merged_track_id {
				let source_name: rusqlite::types::Value = row.get(1)?;
				let plain: rusqlite::types::Value = row.get(2)?;
				let synced: rusqlite::types::Value = row.get(3)?;
				let instrumental: rusqlite::types::Value = row.get(4)?;
				insert.execute(params![
					track_id,
					source_name,
					plain,
					synced,
					instrumental,
					lib_uid
				])?;
			}
		}
		Ok(())
	})
}

// ─── Cleanup deleted records ──────────────────────────────────────────────────

fn cleanup_deleted_records(
	source: &Connection,
	merged: &Connection,
	lib_uid: &str,
) -> Result<(), String> {
	for table in &["tracks", "albums", "artists"] {
		let source_uids: HashSet<String> = {
			let mut stmt = source
				.prepare(&format!("SELECT uid FROM {}", table))
				.map_err(|e| e.to_string())?;
			let result: HashSet<String> = stmt
				.query_map([], |row| row.get::<_, String>(0))
				.map_err(|e| e.to_string())?
				.filter_map(|r| r.ok())
				.collect();
			result
		};

		let merged_uids: Vec<String> = {
			let mut stmt = merged
				.prepare(&format!(
					"SELECT uid FROM {} WHERE source_lib_uid = ?1",
					table
				))
				.map_err(|e| e.to_string())?;
			let result: Vec<String> = stmt
				.query_map(params![lib_uid], |row| row.get::<_, String>(0))
				.map_err(|e| e.to_string())?
				.filter_map(|r| r.ok())
				.collect();
			result
		};

		let missing: Vec<String> = merged_uids
			.into_iter()
			.filter(|u| !source_uids.contains(u))
			.collect();

		if table == &"tracks" && !missing.is_empty() {
			delete_merged_lyrics_for(merged, &missing)?;
		}

		let mut remove = merged
			.prepare_cached(&format!(
				"DELETE FROM {} WHERE uid = ?1 AND source_lib_uid = ?2",
				table
			))
			.map_err(|e| e.to_string())?;
		for uid in missing {
			remove
				.execute(params![uid, lib_uid])
				.map_err(|e| e.to_string())?;
		}
	}
	Ok(())
}

// ─── Remove all records from a library ───────────────────────────────────────

fn remove_library_from_merged(merged: &Connection, lib_uid: &str) -> Result<(), String> {
	merged
		.execute(
			"DELETE FROM lyrics WHERE track_id IN (SELECT id FROM tracks WHERE source_lib_uid = ?1)",
			params![lib_uid],
		)
		.map_err(|e| e.to_string())?;
	for table in &["tracks", "albums", "artists"] {
		merged
			.execute(
				&format!("DELETE FROM {} WHERE source_lib_uid = ?1", table),
				params![lib_uid],
			)
			.map_err(|e| e.to_string())?;
	}
	Ok(())
}

// ─── is_local_override refresh ────────────────────────────────────────────────

fn refresh_local_override_flags(merged: &Connection, default_lib_uid: &str) -> Result<(), String> {
	for table in &["tracks", "albums", "artists"] {
		merged
			.execute(
				&format!(
					"UPDATE {} SET is_local_override = CASE WHEN source_lib_uid = ?1 THEN 1 ELSE 0 END",
					table
				),
				params![default_lib_uid],
			)
			.map_err(|e| e.to_string())?;
	}
	Ok(())
}

// ─── Batch rename across all writable libraries ───────────────────────────────

pub fn writable_libraries(profile_uid: &str) -> Result<Vec<Library>, String> {
	let settings_conn = open_tuned(get_settings_db_path(profile_uid))?;
	let libs = get_all_libraries(&settings_conn).map_err(|e| e.to_string())?;
	Ok(libs
		.into_iter()
		.filter(|l| (l.is_default || l.has_write_permission) && std::path::Path::new(&l.file_path).exists())
		.collect())
}

fn rename_artist_in_table(
	conn: &Connection,
	table: &str,
	old_lower: &str,
	new_name: &str,
) -> Result<Vec<String>, String> {
	let rows: Vec<(String, Option<String>, Option<String>)> = {
		let mut stmt = conn
			.prepare(&format!(
				"SELECT uid, artists, album_artist FROM {} WHERE artists IS NOT NULL OR album_artist IS NOT NULL",
				table
			))
			.map_err(|e| e.to_string())?;
		let collected: Vec<(String, Option<String>, Option<String>)> = stmt
			.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
			.map_err(|e| e.to_string())?
			.filter_map(|r| r.ok())
			.collect();
		collected
	};

	let mut update = conn
		.prepare_cached(&format!(
			"UPDATE {} SET artists = ?1, album_artist = ?2 WHERE uid = ?3",
			table
		))
		.map_err(|e| e.to_string())?;

	let mut changed = Vec::new();

	for (uid, artists, album_artist) in rows {
		let mut dirty = false;
		let mut new_artists = artists.clone();
		let mut new_album_artist = album_artist.clone();

		if let Some(ref json) = artists {
			if let Ok(mut list) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
				for item in list.iter_mut() {
					match item {
						serde_json::Value::String(s) => {
							if s.to_lowercase() == old_lower {
								*s = new_name.to_string();
								dirty = true;
							}
						}
						serde_json::Value::Object(map) => {
							let matches = map
								.get("name")
								.and_then(|n| n.as_str())
								.map(|n| n.to_lowercase() == old_lower)
								.unwrap_or(false);
							if matches {
								map.insert(
									"name".to_string(),
									serde_json::Value::String(new_name.to_string()),
								);
								dirty = true;
							}
						}
						_ => {}
					}
				}
				if dirty {
					new_artists = Some(serde_json::to_string(&list).map_err(|e| e.to_string())?);
				}
			}
		}

		if let Some(ref aa) = album_artist {
			if aa.to_lowercase() == old_lower {
				new_album_artist = Some(new_name.to_string());
				dirty = true;
			}
		}

		if dirty {
			update
				.execute(params![new_artists, new_album_artist, uid])
				.map_err(|e| e.to_string())?;
			changed.push(uid);
		}
	}

	Ok(changed)
}

pub fn rename_artist_everywhere(
	profile_uid: &str,
	old_name: &str,
	new_name: &str,
) -> Result<usize, String> {
	let old_lower = old_name.trim().to_lowercase();
	let new_name = new_name.trim();
	if old_lower.is_empty() || new_name.is_empty() || old_lower == new_name.to_lowercase() {
		return Ok(0);
	}

	let mut total = 0usize;
	for lib in writable_libraries(profile_uid)? {
		let conn = open_tuned(&lib.file_path)?;
		let (tracks, albums) = {
			let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
			let tracks = rename_artist_in_table(&conn, "tracks", &old_lower, new_name)?;
			let albums = rename_artist_in_table(&conn, "albums", &old_lower, new_name)?;
			tx.commit().map_err(|e| e.to_string())?;
			(tracks, albums)
		};
		total += tracks.len() + albums.len();
		sync_entities(profile_uid, &lib.uid, &tracks, &albums, &[])?;
	}
	Ok(total)
}

pub fn rename_album_everywhere(
	profile_uid: &str,
	old_name: &str,
	new_name: &str,
	album_artist: &str,
	album_uid: Option<&str>,
) -> Result<usize, String> {
	let old_lower = old_name.trim().to_lowercase();
	let new_name = new_name.trim();
	let artist_lower = album_artist.trim().to_lowercase();
	if old_lower.is_empty() || new_name.is_empty() || old_lower == new_name.to_lowercase() {
		return Ok(0);
	}
	let target_uid = album_uid.filter(|u| !u.is_empty());

	let mut total = 0usize;
	for lib in writable_libraries(profile_uid)? {
		let conn = open_tuned(&lib.file_path)?;

		let rows: Vec<(String, String, Option<String>)> = {
			let mut stmt = conn
				.prepare("SELECT uid, albums, album_artist FROM tracks WHERE albums IS NOT NULL AND albums != '[]'")
				.map_err(|e| e.to_string())?;
			let collected: Vec<(String, String, Option<String>)> = stmt
				.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
				.map_err(|e| e.to_string())?
				.filter_map(|r| r.ok())
				.collect();
			collected
		};

		let mut changed: Vec<String> = Vec::new();
		{
			let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
			let mut update = conn
				.prepare_cached("UPDATE tracks SET albums = ?1 WHERE uid = ?2")
				.map_err(|e| e.to_string())?;

			for (uid, albums_json, track_album_artist) in rows {
				let mut list = match serde_json::from_str::<Vec<serde_json::Value>>(&albums_json) {
					Ok(l) => l,
					Err(_) => continue,
				};
				let artist_matches = track_album_artist
					.as_deref()
					.map(|a| a.trim().to_lowercase() == artist_lower)
					.unwrap_or(artist_lower.is_empty());
				let mut dirty = false;

				for entry in list.iter_mut() {
					let entry_uid = entry
						.get("uid")
						.and_then(|u| u.as_str())
						.unwrap_or("")
						.to_string();
					let entry_name = entry
						.get("name")
						.and_then(|n| n.as_str())
						.unwrap_or("")
						.to_lowercase();

					let is_target = match target_uid {
						Some(t) if !entry_uid.is_empty() => entry_uid == t,
						_ => entry_name == old_lower && artist_matches,
					};

					if is_target {
						if let Some(obj) = entry.as_object_mut() {
							obj.insert(
								"name".to_string(),
								serde_json::Value::String(new_name.to_string()),
							);
							dirty = true;
						}
					}
				}

				if dirty {
					let out = serde_json::to_string(&list).map_err(|e| e.to_string())?;
					update.execute(params![out, uid]).map_err(|e| e.to_string())?;
					changed.push(uid);
				}
			}
			drop(update);
			tx.commit().map_err(|e| e.to_string())?;
		}

		total += changed.len();
		sync_entities(profile_uid, &lib.uid, &changed, &[], &[])?;
	}
	Ok(total)
}

// ─── Write routing ────────────────────────────────────────────────────────────

// Resolves which library a merged record came from.
// Returns (source_lib_uid, is_default).
pub fn resolve_source_library(
    profile_uid: &str,
    entity_uid: &str,
    entity_table: &str,
) -> Result<(String, bool), String> {
    let merged_conn = open_merged_conn(profile_uid)?;
    let source_lib_uid: String = merged_conn
        .query_row(
            &format!("SELECT source_lib_uid FROM {} WHERE uid = ?1", entity_table),
            params![entity_uid],
            |row| row.get(0),
        )
        .map_err(|e| format!("Record not found in merged db: {}", e))?;

    let settings_conn =
        open_tuned(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

    let lib = get_library_by_uid(&settings_conn, &source_lib_uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Library not found: {}", source_lib_uid))?;

    Ok((source_lib_uid, lib.is_default))
}

// Opens a connection to the source library for a given merged entity.
// Returns (conn, library) so the caller can check write permission.
pub fn open_source_conn_for_entity(
    profile_uid: &str,
    entity_uid: &str,
    entity_table: &str,
) -> Result<(Connection, Library), String> {
    let (source_lib_uid, _) = resolve_source_library(profile_uid, entity_uid, entity_table)?;

    let settings_conn =
        open_tuned(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

    let lib = get_library_by_uid(&settings_conn, &source_lib_uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Library not found: {}", source_lib_uid))?;

    let conn = open_tuned(&lib.file_path).map_err(|e| e.to_string())?;
    Ok((conn, lib))
}

// ─── Remote push ─────────────────────────────────────────────────────────────

// Attempts to push a local library file to its sync_url.
// Returns Ok(true) on success, Ok(false) if no sync_url is configured.
// On failure the error is logged but does not propagate — the local write
// already succeeded; the library is simply flagged as having unpushed changes.
pub async fn try_push_library(lib: &Library) -> bool {
    let sync_url = match &lib.sync_url {
        Some(u) => u.clone(),
        None => return false,
    };

    fn is_remote_http(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    checkpoint_db(&lib.file_path);

    if !is_remote_http(&sync_url) {
        match std::fs::copy(&lib.file_path, &sync_url) {
            Ok(_) => {
                let sidecar_path = format!("{}.json", sync_url);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                let sidecar = format!(
                    "{{\n\t\"last_modified\": {},\n\t\"write_requires_token\": false\n}}\n",
                    now
                );
                std::fs::write(&sidecar_path, sidecar).ok();
                true
            }
            Err(e) => {
                eprintln!("[library_manager] push: failed to copy to share: {}", e);
                false
            }
        }
    } else {
        let write_token = match &lib.write_token {
            Some(t) => t.clone(),
            None => return false,
        };

        let file_bytes = match std::fs::read(&lib.file_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[library_manager] push: failed to read file: {}", e);
                return false;
            }
        };

        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[library_manager] push: failed to build client: {}", e);
                return false;
            }
        };

        match client
            .put(&sync_url)
            .header("Authorization", format!("Bearer {}", write_token))
            .header("Content-Type", "application/octet-stream")
            .body(file_bytes)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => true,
            Ok(resp) => {
                eprintln!(
                    "[library_manager] push: server returned {} for {}",
                    resp.status(),
                    lib.uid
                );
                false
            }
            Err(e) => {
                eprintln!("[library_manager] push: request failed: {}", e);
                false
            }
        }
    }
}

// ─── Remote pull ─────────────────────────────────────────────────────────────

// Fetches the sidecar JSON, compares last_modified to last_synced,
// and downloads the db file if newer. Returns Ok(true) if a pull occurred.
pub async fn try_pull_library(profile_uid: &str, lib_uid: &str) -> Result<bool, String> {
    let settings_conn =
        open_tuned(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

    let lib = get_library_by_uid(&settings_conn, lib_uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Library not found: {}", lib_uid))?;

    let sync_url = match &lib.sync_url {
        Some(u) => u.clone(),
        None => return Ok(false),
    };

    fn is_unc_or_local(url: &str) -> bool {
        url.starts_with("\\\\")
            || url.starts_with("//")
            || (!url.starts_with("http://") && !url.starts_with("https://"))
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    if is_unc_or_local(&sync_url) {
        let sidecar_path = format!("{}.json", sync_url);
        if let Ok(content) = std::fs::read_to_string(&sidecar_path) {
            if let Ok(s) = serde_json::from_str::<RemoteSidecar>(&content) {
                if s.last_modified <= lib.last_synced {
                    return Ok(false);
                }
            }
        }

        remove_wal_files(&lib.file_path);
        std::fs::copy(&sync_url, &lib.file_path)
            .map_err(|e| format!("Could not copy database file: {}", e))?;
    } else {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;

        let sidecar_url = format!("{}.json", sync_url);
        if let Ok(resp) = client.get(&sidecar_url).send().await {
            if let Ok(sidecar) = resp.json::<RemoteSidecar>().await {
                if sidecar.last_modified <= lib.last_synced {
                    return Ok(false);
                }
            }
        }

        let bytes = client
            .get(&sync_url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .bytes()
            .await
            .map_err(|e| e.to_string())?;

        remove_wal_files(&lib.file_path);
        std::fs::write(&lib.file_path, &bytes).map_err(|e| e.to_string())?;
    }

    crate::db::library_registry::update_library(
        &settings_conn,
        lib_uid,
        &LibraryUpdate {
            name: None,
            sync_url: None,
            sync_meta_url: None,
            write_token: None,
            has_write_permission: None,
            last_synced: Some(now),
            last_data_version: None,
            sort_order: None,
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(true)
}

// ─── Write permission check ───────────────────────────────────────────────────

pub async fn check_write_permission(profile_uid: &str, lib_uid: &str) -> Result<bool, String> {
    let settings_conn =
        open_tuned(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

    let lib = get_library_by_uid(&settings_conn, lib_uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Library not found: {}", lib_uid))?;

    fn is_remote_http(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    let has_permission = match &lib.sync_url {
        None => true,
        Some(url) if !is_remote_http(url) => {
            std::fs::OpenOptions::new().write(true).open(url).is_ok()
        }
        Some(_) => {
            let sync_url = lib.sync_url.as_deref().unwrap_or("");
            let token = lib.write_token.as_deref().unwrap_or("");
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| e.to_string())?;
            match client
                .head(sync_url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
            {
                Ok(resp) => resp.status().is_success() || resp.status().as_u16() == 405,
                Err(_) => false,
            }
        }
    };

    let update = LibraryUpdate {
        has_write_permission: Some(has_permission),
        name: None,
        sync_url: None,
        sync_meta_url: None,
        write_token: None,
        last_synced: None,
        last_data_version: None,
        sort_order: None,
    };
    crate::db::library_registry::update_library(&settings_conn, lib_uid, &update)
        .map_err(|e| e.to_string())?;

    Ok(has_permission)
}

// ─── Import a remote library ──────────────────────────────────────────────────

pub async fn import_library(
    profile_uid: &str,
    name: &str,
    sync_url: &str,
    write_token: Option<&str>,
) -> Result<Library, String> {
    fn is_unc_or_local(url: &str) -> bool {
        url.starts_with("\\\\")
            || url.starts_with("//")
            || (!url.starts_with("http://") && !url.starts_with("https://"))
    }

    let lib_uid = crate::db::generate_uid("lib");
    let file_path = get_library_db_path(profile_uid, &lib_uid);

    let mut last_synced = 0i64;

    if is_unc_or_local(sync_url) {
        std::fs::copy(sync_url, &file_path)
            .map_err(|e| format!("Could not copy database file: {}", e))?;

        let sidecar_path = format!("{}.json", sync_url);
        if let Ok(content) = std::fs::read_to_string(&sidecar_path) {
            if let Ok(s) = serde_json::from_str::<RemoteSidecar>(&content) {
                if s.last_modified > 0 {
                    last_synced = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                }
            }
        }
    } else {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;

        let sidecar_url = format!("{}.json", sync_url);
        if let Ok(resp) = client.get(&sidecar_url).send().await {
            if let Ok(s) = resp.json::<RemoteSidecar>().await {
                if s.last_modified > 0 {
                    last_synced = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                }
            }
        }

        let bytes = client
            .get(sync_url)
            .send()
            .await
            .map_err(|e| format!("Could not download remote database: {}", e))?
            .bytes()
            .await
            .map_err(|e| e.to_string())?;

        std::fs::write(&file_path, &bytes).map_err(|e| e.to_string())?;
    }

    let settings_conn =
        open_tuned(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

    let existing_libs = get_all_libraries(&settings_conn).map_err(|e| e.to_string())?;
    let sort_order = existing_libs.len() as i64;

    let sidecar_url = format!("{}.json", sync_url);

    let library = Library {
        uid: lib_uid.clone(),
        name: name.to_string(),
        is_default: false,
        file_path: file_path.to_string_lossy().to_string(),
        sync_url: Some(sync_url.to_string()),
        sync_meta_url: Some(sidecar_url),
        write_token: write_token.map(|t| t.to_string()),
        has_write_permission: false,
        last_synced,
        last_data_version: 0,
        sort_order,
    };

    crate::db::library_registry::create_library(&settings_conn, &library)
        .map_err(|e| e.to_string())?;

    let puid = profile_uid.to_string();
    let luid = lib_uid.clone();
    tauri::async_runtime::spawn(async move {
        let _ = check_write_permission(&puid, &luid).await;
    });

    full_rebuild(profile_uid)?;

    Ok(library)
}

// ─── Export a library ─────────────────────────────────────────────────────────

pub fn export_library(profile_uid: &str, lib_uid: &str, dest_path: &str) -> Result<(), String> {
    let settings_conn =
        open_tuned(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

    let lib = get_library_by_uid(&settings_conn, lib_uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Library not found: {}", lib_uid))?;

    checkpoint_db(&lib.file_path);
    std::fs::copy(&lib.file_path, dest_path).map_err(|e| e.to_string())?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let sidecar = format!(
        "{{\n\t\"last_modified\": {},\n\t\"write_requires_token\": false\n}}\n",
        now
    );
    let sidecar_path = format!("{}.json", dest_path);
    std::fs::write(&sidecar_path, sidecar).map_err(|e| e.to_string())?;

    Ok(())
}

// ─── Create a new local library ───────────────────────────────────────────────

pub fn create_local_library(profile_uid: &str, name: &str) -> Result<Library, String> {
    let lib_uid = crate::db::generate_uid("lib");
    let file_path = get_library_db_path(profile_uid, &lib_uid);

    let lib_conn = open_tuned(&file_path).map_err(|e| e.to_string())?;
    init_library_db(&lib_conn).map_err(|e| e.to_string())?;

    let settings_conn =
        open_tuned(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

    let existing = get_all_libraries(&settings_conn).map_err(|e| e.to_string())?;
    let sort_order = existing.len() as i64;

    let library = Library {
        uid: lib_uid.clone(),
        name: name.to_string(),
        is_default: false,
        file_path: file_path.to_string_lossy().to_string(),
        sync_url: None,
        sync_meta_url: None,
        write_token: None,
        has_write_permission: true,
        last_synced: 0,
        last_data_version: 0,
        sort_order,
    };

    crate::db::library_registry::create_library(&settings_conn, &library)
        .map_err(|e| e.to_string())?;

    Ok(library)
}

// ─── Delete a library ─────────────────────────────────────────────────────────

pub fn delete_local_library(
    profile_uid: &str,
    lib_uid: &str,
    delete_file: bool,
) -> Result<(), String> {
    let settings_conn =
        open_tuned(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

    let lib = get_library_by_uid(&settings_conn, lib_uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Library not found: {}", lib_uid))?;

    if lib.is_default {
        return Err("Cannot delete the default library".to_string());
    }

    crate::db::library_registry::delete_library(&settings_conn, lib_uid)
        .map_err(|e| e.to_string())?;

    if delete_file && std::path::Path::new(&lib.file_path).exists() {
        std::fs::remove_file(&lib.file_path).map_err(|e| e.to_string())?;
        remove_wal_files(&lib.file_path);
    }

    // Remove this library's records from merged
    let merged_path = get_merged_db_path(profile_uid);
    if merged_path.exists() {
        let merged_conn = open_tuned(&merged_path).map_err(|e| e.to_string())?;
        remove_library_from_merged(&merged_conn, lib_uid)?;
    }

    Ok(())
}

// ─── Get the libraries dir for a profile ─────────────────────────────────────

pub fn get_profile_libraries_dir(profile_uid: &str) -> std::path::PathBuf {
    get_libraries_dir(profile_uid)
}
