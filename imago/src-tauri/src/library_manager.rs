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

pub fn open_library_conn(profile_uid: &str, lib_uid: &str) -> Result<Connection, String> {
    let path = get_library_db_path(profile_uid, lib_uid);
    let conn = Connection::open(&path).map_err(|e| e.to_string())?;
    init_library_db(&conn).map_err(|e| e.to_string())?;
    Ok(conn)
}

pub fn open_local_library_conn(profile_uid: &str) -> Result<Connection, String> {
    let path = get_local_library_db_path(profile_uid);
    let conn = Connection::open(&path).map_err(|e| e.to_string())?;
    init_library_db(&conn).map_err(|e| e.to_string())?;
    Ok(conn)
}

pub fn open_merged_conn(profile_uid: &str) -> Result<Connection, String> {
    let path = get_merged_db_path(profile_uid);
    let conn = Connection::open(&path).map_err(|e| e.to_string())?;
    Ok(conn)
}

// ─── Default library bootstrap ───────────────────────────────────────────────

// Called when a profile is first set up or when the libraries table is empty.
// Creates the local.db file and registers it as the default library.
pub fn ensure_default_library(profile_uid: &str) -> Result<String, String> {
    let settings_conn =
        Connection::open(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

    if let Some(existing) = get_default_library(&settings_conn).map_err(|e| e.to_string())? {
        return Ok(existing.uid);
    }

    let lib_uid = crate::db::generate_uid("lib");
    let file_path = get_local_library_db_path(profile_uid);

    let lib_conn = Connection::open(&file_path).map_err(|e| e.to_string())?;
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
        Connection::open(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

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

// Deletes and recreates merged.db from all source libraries.
// Always called after: import, remote pull replacing entire file, manual request.
pub fn full_rebuild(profile_uid: &str) -> Result<MergeResult, String> {
    let merged_path = get_merged_db_path(profile_uid);

    // Delete existing merged.db
    if merged_path.exists() {
        std::fs::remove_file(&merged_path).map_err(|e| e.to_string())?;
    }

    let merged_conn = Connection::open(&merged_path).map_err(|e| e.to_string())?;
    init_merged_db(&merged_conn).map_err(|e| e.to_string())?;

    let settings_conn =
        Connection::open(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

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

        let source_conn = Connection::open(&lib.file_path).map_err(|e| e.to_string())?;
        copy_library_into_merged(
            &source_conn,
            &merged_conn,
            &lib.uid,
            &default_lib_uid,
            &blocklist,
        )?;

        let current_version = read_data_version(&lib.file_path);
        store_data_version(&settings_conn, &lib.uid, current_version).map_err(|e| e.to_string())?;
    }

    Ok(MergeResult {
        rebuilt: true,
        libraries_processed: count,
    })
}

// ─── Incremental update ───────────────────────────────────────────────────────

// Reprocesses only the libraries whose data_version has changed.
// For each changed library: upsert all its records into merged.db,
// then remove any merged records that no longer exist in any source.
pub fn incremental_update(profile_uid: &str, changed_lib_uids: &[String]) -> Result<(), String> {
    let merged_path = get_merged_db_path(profile_uid);
    if !merged_path.exists() {
        full_rebuild(profile_uid)?;
        return Ok(());
    }

    let merged_conn = Connection::open(&merged_path).map_err(|e| e.to_string())?;
    let settings_conn =
        Connection::open(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

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
            // Library file gone — remove all its records from merged
            remove_library_from_merged(&merged_conn, lib_uid)?;
            continue;
        }

        let source_conn = Connection::open(&lib.file_path).map_err(|e| e.to_string())?;
        copy_library_into_merged(
            &source_conn,
            &merged_conn,
            lib_uid,
            &default_lib_uid,
            &blocklist,
        )?;

        // Remove merged records from this library that no longer exist in the source
        cleanup_deleted_records(&source_conn, &merged_conn, lib_uid)?;

        let current_version = read_data_version(&lib.file_path);
        store_data_version(&settings_conn, lib_uid, current_version).map_err(|e| e.to_string())?;
    }

    // Recompute is_local_override across the full merged set
    refresh_local_override_flags(&merged_conn, &default_lib_uid)?;

    Ok(())
}

// ─── Copy one library into merged ─────────────────────────────────────────────

fn copy_library_into_merged(
    source: &Connection,
    merged: &Connection,
    lib_uid: &str,
    default_lib_uid: &str,
    blocklist: &HashSet<String>,
) -> Result<(), String> {
    copy_tracks(source, merged, lib_uid, blocklist).map_err(|e| e.to_string())?;
    copy_albums(source, merged, lib_uid, blocklist).map_err(|e| e.to_string())?;
    copy_artists(source, merged, lib_uid, blocklist).map_err(|e| e.to_string())?;
    copy_lyrics(source, merged, lib_uid).map_err(|e| e.to_string())?;
    refresh_local_override_flags(merged, default_lib_uid)?;
    Ok(())
}

fn copy_tracks(
    source: &Connection,
    merged: &Connection,
    lib_uid: &str,
    blocklist: &HashSet<String>,
) -> rusqlite::Result<()> {
    let mut stmt = source.prepare(
        "SELECT uid, path, last_modified, title, artists, album_artist, albums, genres, year,
		rating, tags, duration_ms, bpm, key, credits, label, artwork_blob, artwork_path,
		artwork_thumb, user_options, format, bitrate, remote_path, remote_data, track_data, date_added
		FROM tracks",
    )?;

    let rows: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<f32>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<i64>>(11)?,
                row.get::<_, Option<f32>>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, Option<String>>(15)?,
                row.get::<_, Option<Vec<u8>>>(16)?,
                row.get::<_, Option<String>>(17)?,
                row.get::<_, Option<String>>(18)?,
                row.get::<_, Option<String>>(19)?,
                row.get::<_, Option<String>>(20)?,
                row.get::<_, Option<i64>>(21)?,
                row.get::<_, Option<String>>(22)?,
                row.get::<_, Option<String>>(23)?,
                row.get::<_, Option<String>>(24)?,
                row.get::<_, Option<i64>>(25)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for row in rows {
        if blocklist.contains(&row.0) {
            continue;
        }
        merged.execute(
			"INSERT INTO tracks (
				uid, path, last_modified, title, artists, album_artist, albums, genres, year,
				rating, tags, duration_ms, bpm, key, credits, label, artwork_blob, artwork_path,
				artwork_thumb, user_options, format, bitrate, remote_path, remote_data, track_data,
				source_lib_uid, date_added
			) VALUES (
				?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,COALESCE(?27, 0)
			)
			ON CONFLICT(uid) DO UPDATE SET
				path          = excluded.path,
				last_modified = excluded.last_modified,
				title         = excluded.title,
				artists       = excluded.artists,
				album_artist  = excluded.album_artist,
				albums        = excluded.albums,
				genres        = excluded.genres,
				year          = excluded.year,
				rating        = excluded.rating,
				tags          = excluded.tags,
				duration_ms   = excluded.duration_ms,
				bpm           = excluded.bpm,
				key           = excluded.key,
				credits       = excluded.credits,
				label         = excluded.label,
				artwork_blob  = excluded.artwork_blob,
				artwork_path  = excluded.artwork_path,
				artwork_thumb = excluded.artwork_thumb,
				user_options  = excluded.user_options,
				format        = excluded.format,
				bitrate       = excluded.bitrate,
				remote_path   = excluded.remote_path,
				remote_data   = excluded.remote_data,
				track_data    = excluded.track_data,
				source_lib_uid = excluded.source_lib_uid,
				date_added    = excluded.date_added",
			params![
				row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8,
				row.9, row.10, row.11, row.12, row.13, row.14, row.15, row.16, row.17,
				row.18, row.19, row.20, row.21, row.22, row.23, row.24, lib_uid, row.25,
			],
		)?;
    }
    Ok(())
}

fn copy_albums(
    source: &Connection,
    merged: &Connection,
    lib_uid: &str,
    blocklist: &HashSet<String>,
) -> rusqlite::Result<()> {
    let mut stmt = source.prepare(
        "SELECT uid, format, title, rating, artists, album_artist, release_date,
		tags, genres, tracks, credits, label, artwork_blob, artwork_path, artwork_thumb, emulate_type
		FROM albums",
    )?;

    let rows: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<f32>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<Vec<u8>>>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, Option<String>>(15)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for row in rows {
        if blocklist.contains(&row.0) {
            continue;
        }
        merged.execute(
            "INSERT INTO albums (
				uid, format, title, rating, artists, album_artist, release_date,
				tags, genres, tracks, credits, label, artwork_blob, artwork_path,
				artwork_thumb, emulate_type, source_lib_uid
			) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
			ON CONFLICT(uid) DO UPDATE SET
				format       = excluded.format,
				title        = excluded.title,
				rating       = excluded.rating,
				artists      = excluded.artists,
				album_artist = excluded.album_artist,
				release_date = excluded.release_date,
				tags         = excluded.tags,
				genres       = excluded.genres,
				tracks       = excluded.tracks,
				credits      = excluded.credits,
				label        = excluded.label,
				artwork_blob = excluded.artwork_blob,
				artwork_path = excluded.artwork_path,
				artwork_thumb = excluded.artwork_thumb,
				emulate_type = excluded.emulate_type,
				source_lib_uid = excluded.source_lib_uid",
            params![
                row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8, row.9, row.10,
                row.11, row.12, row.13, row.14, row.15, lib_uid,
            ],
        )?;
    }
    Ok(())
}

fn copy_artists(
    source: &Connection,
    merged: &Connection,
    lib_uid: &str,
    blocklist: &HashSet<String>,
) -> rusqlite::Result<()> {
    let mut stmt = source.prepare(
        "SELECT uid, name, aka, about, tags, genres, websites, members,
		profile_art_blob, profile_art_path, profile_art_thumb, banner_art_blob, banner_art_path
		FROM artists",
    )?;

    let rows: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<Vec<u8>>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<Vec<u8>>>(11)?,
                row.get::<_, Option<String>>(12)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for row in rows {
        if blocklist.contains(&row.0) {
            continue;
        }
        merged.execute(
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
            params![
                row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8, row.9, row.10,
                row.11, row.12, lib_uid,
            ],
        )?;
    }
    Ok(())
}

fn copy_lyrics(source: &Connection, merged: &Connection, lib_uid: &str) -> rusqlite::Result<()> {
    // Lyrics are joined to tracks via track_id (numeric). In merged.db the track_id
    // may differ from the source, so we look up the merged track's id by uid.
    let mut stmt = source.prepare(
        "SELECT t.uid, l.source, l.plain, l.synced, l.instrumental
		FROM lyrics l
		JOIN tracks t ON t.id = l.track_id",
    )?;

    let rows: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for (track_uid, source_name, plain, synced, instrumental) in rows {
        let merged_track_id: Option<i64> = merged
            .query_row(
                "SELECT id FROM tracks WHERE uid = ?1",
                params![track_uid],
                |row| row.get(0),
            )
            .ok();

        if let Some(track_id) = merged_track_id {
            merged.execute(
                "INSERT INTO lyrics (track_id, source, plain, synced, instrumental, source_lib_uid)
				VALUES (?1,?2,?3,?4,?5,?6)
				ON CONFLICT(track_id) DO UPDATE SET
					source         = excluded.source,
					plain          = excluded.plain,
					synced         = excluded.synced,
					instrumental   = excluded.instrumental,
					source_lib_uid = excluded.source_lib_uid",
                params![track_id, source_name, plain, synced, instrumental, lib_uid],
            )?;
        }
    }
    Ok(())
}

// ─── Cleanup deleted records ──────────────────────────────────────────────────

// After re-copying a changed library, remove any merged records attributed to
// that library which no longer exist in the source. Handles deletes correctly.
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

        for uid in merged_uids {
            if !source_uids.contains(&uid) {
                merged
                    .execute(
                        &format!(
                            "DELETE FROM {} WHERE uid = ?1 AND source_lib_uid = ?2",
                            table
                        ),
                        params![uid, lib_uid],
                    )
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

// ─── Remove all records from a library ───────────────────────────────────────

fn remove_library_from_merged(merged: &Connection, lib_uid: &str) -> Result<(), String> {
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

// After any merge pass, marks records where the default local library has
// its own version. This tells the UI which records have local overrides.
fn refresh_local_override_flags(merged: &Connection, default_lib_uid: &str) -> Result<(), String> {
    for table in &["tracks", "albums", "artists"] {
        merged
            .execute(
                &format!(
                    "UPDATE {table} SET is_local_override = CASE
						WHEN source_lib_uid = ?1 THEN 1
						WHEN uid IN (SELECT uid FROM {table} WHERE source_lib_uid = ?1) THEN 1
						ELSE 0
					END"
                ),
                params![default_lib_uid],
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(())
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
        Connection::open(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

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
        Connection::open(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

    let lib = get_library_by_uid(&settings_conn, &source_lib_uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Library not found: {}", source_lib_uid))?;

    let conn = Connection::open(&lib.file_path).map_err(|e| e.to_string())?;
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
        Connection::open(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

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
        Connection::open(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

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
        Connection::open(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

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
        Connection::open(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

    let lib = get_library_by_uid(&settings_conn, lib_uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Library not found: {}", lib_uid))?;

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

    let lib_conn = Connection::open(&file_path).map_err(|e| e.to_string())?;
    init_library_db(&lib_conn).map_err(|e| e.to_string())?;

    let settings_conn =
        Connection::open(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

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
        Connection::open(get_settings_db_path(profile_uid)).map_err(|e| e.to_string())?;

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
    }

    // Remove this library's records from merged
    let merged_path = get_merged_db_path(profile_uid);
    if merged_path.exists() {
        let merged_conn = Connection::open(&merged_path).map_err(|e| e.to_string())?;
        remove_library_from_merged(&merged_conn, lib_uid)?;
    }

    Ok(())
}

// ─── Get the libraries dir for a profile ─────────────────────────────────────

pub fn get_profile_libraries_dir(profile_uid: &str) -> std::path::PathBuf {
    get_libraries_dir(profile_uid)
}
