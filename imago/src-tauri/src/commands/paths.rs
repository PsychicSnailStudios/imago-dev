use crate::db::settings_manager::LibraryPath;
use crate::library_manager;
use crate::profiles::{get_lib_db_path, get_library_db_path, get_settings_db_path};
use crate::state::AppState;
use crate::{open_local_library_conn, open_merged_conn, open_settings_conn};
use rusqlite::Connection;
use tauri::Manager;
use tauri::{AppHandle, Emitter, State};

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn open_library_conn_for_uid(profile_uid: &str, lib_uid: &str) -> Result<Connection, String> {
    let path = get_library_db_path(profile_uid, lib_uid);
    Connection::open(&path).map_err(|e| e.to_string())
}

fn add_path_to_lib_db(conn: &Connection, path: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO library_paths (path) VALUES (?1)",
        rusqlite::params![path],
    )?;
    Ok(())
}

fn remove_path_from_lib_db(conn: &Connection, path: &str) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM library_paths WHERE path = ?1",
        rusqlite::params![path],
    )?;
    Ok(())
}

pub fn get_paths_from_lib_db(conn: &Connection) -> rusqlite::Result<Vec<LibraryPath>> {
    let mut stmt = conn.prepare("SELECT id, path FROM library_paths ORDER BY id ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(LibraryPath {
            id: Some(row.get(0)?),
            path: row.get::<_, String>(1)?.replace('\\', "/"),
        })
    })?;
    rows.collect()
}

fn get_lib_uid_for_path(settings_conn: &Connection, path: &str) -> Option<String> {
    settings_conn
        .query_row(
            "SELECT lib_uid FROM library_paths WHERE path = ?1",
            rusqlite::params![path],
            |row| row.get(0),
        )
        .ok()
        .flatten()
}

#[derive(Default, Clone)]
pub struct EntitySnapshot {
    pub tracks: std::collections::HashSet<String>,
    pub albums: std::collections::HashSet<String>,
    pub artists: std::collections::HashSet<String>,
}

fn snapshot_entity_uids(profile_uid: &str) -> EntitySnapshot {
    let conn = open_merged_conn(profile_uid);
    let collect = |table: &str| -> std::collections::HashSet<String> {
        conn.prepare(&format!("SELECT uid FROM {}", table))
            .ok()
            .map(|mut stmt| {
                let set: std::collections::HashSet<String> = stmt
                    .query_map([], |r| r.get::<_, String>(0))
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default();
                set
            })
            .unwrap_or_default()
    };
    EntitySnapshot {
        tracks: collect("tracks"),
        albums: collect("albums"),
        artists: collect("artists"),
    }
}

fn entities_added_since(profile_uid: &str, before: &EntitySnapshot) -> EntitySnapshot {
    let after = snapshot_entity_uids(profile_uid);
    EntitySnapshot {
        tracks: after.tracks.difference(&before.tracks).cloned().collect(),
        albums: after.albums.difference(&before.albums).cloned().collect(),
        artists: after.artists.difference(&before.artists).cloned().collect(),
    }
}

fn spawn_post_scan_tasks(app: AppHandle, profile_uid: String, added: EntitySnapshot) {
	{
		let relink_app = app.clone();
		let relink_uid = profile_uid.clone();
		std::thread::spawn(move || {
			match crate::commands::scrobbles::relink_unmatched_scrobbles(&relink_uid) {
				Ok(n) if n > 0 => {
					relink_app.emit("scrobbles:relinked", n).ok();
				}
				Ok(_) => {}
				Err(e) => eprintln!("[scrobbles] relink failed: {e}"),
			}
		});
	}

	let settings_conn = open_settings_conn(&profile_uid);
	let flag = |key: &str| -> bool {
		crate::db::get_setting(&settings_conn, key)
			.ok()
			.flatten()
			.map(|v| v == "true")
			.unwrap_or(false)
	};
	let auto_tracks = flag("auto_enrich_tracks");
	let auto_albums = flag("auto_enrich_albums");
	let auto_artists = flag("auto_enrich_artists");
	let auto_lyrics = flag("auto_fetch_lyrics");

	if !(auto_tracks || auto_albums || auto_artists || auto_lyrics) {
		return;
	}

	tauri::async_runtime::spawn(async move {
		if auto_tracks && !added.tracks.is_empty() {
			let _ = crate::commands::enrichment::enrich_all_for(
				app.clone(),
				profile_uid.clone(),
				Some(added.tracks.clone()),
			)
			.await;
		}
		if auto_albums && !added.albums.is_empty() {
			let _ = crate::commands::enrichment::enrich_all_albums_for(
				app.clone(),
				profile_uid.clone(),
				Some(added.albums.clone()),
			)
			.await;
		}
		if auto_artists && !added.artists.is_empty() {
			let _ = crate::commands::enrichment::enrich_all_artists_for(
				app.clone(),
				profile_uid.clone(),
				Some(added.artists.clone()),
			)
			.await;
		}
		if auto_lyrics {
			crate::commands::lyrics::fetch_missing_lyrics(app.clone(), profile_uid.clone()).await;
		}
	});
}

#[tauri::command]
pub fn add_path(
    app: AppHandle,
    state: State<AppState>,
    path: String,
    lib_uid: Option<String>,
) -> Result<(), String> {
    let is_web = crate::webdav::is_http_url(&path);
    let path = if is_web {
        crate::webdav::encode_url_spaces(&path)
    } else {
        normalize_path(&path)
    };
    if !is_web && !std::path::Path::new(&path).is_dir() {
        return Err(format!("Folder not found or not reachable: {}", path));
    }
    let uid = state.get_uid();
    let settings_conn = open_settings_conn(&uid);

    let target_lib_uid = match lib_uid {
        Some(ref l) => l.clone(),
        None => {
            library_manager::ensure_default_library(&uid)?;
            crate::db::library_registry::get_default_library(&settings_conn)
                .map_err(|e| e.to_string())?
                .ok_or("No default library found")?
                .uid
        }
    };

    let lib_file_path: String = {
        crate::db::library_registry::get_library_by_uid(&settings_conn, &target_lib_uid)
            .ok()
            .flatten()
            .map(|l| l.file_path)
            .unwrap_or_else(|| {
                get_library_db_path(&uid, &target_lib_uid)
                    .to_string_lossy()
                    .to_string()
            })
    };

    if let Ok(Some(lib)) = crate::db::library_registry::get_library_by_uid(&settings_conn, &target_lib_uid) {
        library_manager::ensure_library_file_usable(&lib)?;
    }

    {
        let lib_conn = library_manager::open_tuned(&lib_file_path)
            .map_err(|e| format!("Could not open the library database: {}", e))?;
        crate::db::init_library_db(&lib_conn).map_err(|e| e.to_string())?;
        add_path_to_lib_db(&lib_conn, &path).map_err(|e| e.to_string())?;
    }

    let creds: Option<String> =
        crate::db::library_registry::get_library_by_uid(&settings_conn, &target_lib_uid)
            .ok()
            .flatten()
            .and_then(|l| l.write_token);

    let app_clone = app.clone();
    let path_clone = path.clone();
    let uid_clone = uid.clone();
    let lib_uid_clone = target_lib_uid.clone();
    let lib_file_path_clone = lib_file_path.clone();

    std::thread::spawn(move || {
        let lib_conn = match library_manager::open_tuned(&lib_file_path_clone) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[add_path] could not open library db: {e}");
                app_clone.emit("scan:done", ()).ok();
                return;
            }
        };
        let settings_path = get_settings_db_path(&uid_clone);
        lib_conn
            .execute_batch(&format!(
                "ATTACH DATABASE '{}' AS settings;",
                settings_path.to_string_lossy().replace('\'', "''")
            ))
            .ok();
        let before_scan = snapshot_entity_uids(&uid_clone);
        crate::scanner::scan_directory_with_creds(
            &lib_conn,
            &path_clone,
            &app_clone,
            false,
            creds.as_deref(),
        );

        let _ = library_manager::incremental_update(&uid_clone, &[lib_uid_clone.clone()]);

        let added = entities_added_since(&uid_clone, &before_scan);
		spawn_post_scan_tasks(app_clone.clone(), uid_clone.clone(), added);

        let all_paths: Vec<String> = {
            if let Ok(lc) = Connection::open(&lib_file_path_clone) {
                get_paths_from_lib_db(&lc)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| p.path)
                    .collect()
            } else {
                vec![]
            }
        };
        crate::watcher::start_watcher(app_clone, uid_clone, lib_uid_clone, all_paths);
    });

    Ok(())
}

fn path_key(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

const NORMALISED_TRACK_PATH: &str = "LOWER(REPLACE(path, char(92), '/'))";

fn path_condition(key: &str) -> String {
    let column_expr = if key.starts_with("http://") || key.starts_with("https://") {
        "LOWER(COALESCE(remote_path, ''))"
    } else {
        NORMALISED_TRACK_PATH
    };
    format!(
        "({n} = ?1 OR substr({n}, 1, length(?1) + 1) = ?1 || '/')",
        n = column_expr
    )
}

fn delete_tracks_under_path(conn: &Connection, key: &str) -> rusqlite::Result<usize> {
    let cond = path_condition(key);
    conn.execute(
        &format!(
            "DELETE FROM lyrics WHERE track_id IN (SELECT id FROM tracks WHERE {})",
            cond
        ),
        rusqlite::params![key],
    )?;
    conn.execute(
        &format!("DELETE FROM tracks WHERE {}", cond),
        rusqlite::params![key],
    )
}

fn delete_library_path_row(conn: &Connection, key: &str) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM library_paths WHERE LOWER(RTRIM(REPLACE(path, char(92), '/'), '/')) = ?1",
        rusqlite::params![key],
    )
}

fn table_uids(conn: &Connection, table: &str) -> rusqlite::Result<std::collections::HashSet<String>> {
    let mut stmt = conn.prepare(&format!("SELECT uid FROM {}", table))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect()
}

fn delete_orphans(conn: &Connection) -> rusqlite::Result<(Vec<String>, Vec<String>)> {
    let albums_before = table_uids(conn, "albums")?;
    let artists_before = table_uids(conn, "artists")?;

    conn.execute(
        "DELETE FROM albums WHERE uid NOT IN (
            SELECT CASE WHEN j.type = 'object' THEN json_extract(j.value, '$.uid') END
            FROM tracks t, json_each(CASE WHEN json_valid(t.albums) THEN t.albums ELSE '[]' END) j
            WHERE CASE WHEN j.type = 'object' THEN json_extract(j.value, '$.uid') END IS NOT NULL
        )",
        [],
    )?;
    conn.execute(
        "DELETE FROM artists WHERE LOWER(name) NOT IN (
            SELECT LOWER(j.value)
            FROM tracks t, json_each(CASE WHEN json_valid(t.artists) THEN t.artists ELSE '[]' END) j
            WHERE j.type = 'text'
        ) AND LOWER(name) NOT IN (
            SELECT LOWER(album_artist) FROM tracks
            WHERE album_artist IS NOT NULL AND album_artist != ''
        )",
        [],
    )?;

    let albums_after = table_uids(conn, "albums")?;
    let artists_after = table_uids(conn, "artists")?;
    Ok((
        albums_before.difference(&albums_after).cloned().collect(),
        artists_before.difference(&artists_after).cloned().collect(),
    ))
}

struct Removed {
    tracks: Vec<String>,
    albums: Vec<String>,
    artists: Vec<String>,
}

fn remove_tracks_and_orphans(
    conn: &Connection,
    key: &str,
    remove_path_row: bool,
) -> Result<Removed, String> {
    conn.busy_timeout(std::time::Duration::from_secs(60)).ok();
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    if remove_path_row {
        delete_library_path_row(conn, key).map_err(|e| e.to_string())?;
    }
    let (mut tracks, modified) =
        crate::db::track_paths::detach_prefix(conn, key).map_err(|e| e.to_string())?;
    tracks.extend(modified);
    let (albums, artists) = delete_orphans(conn).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(Removed {
        tracks,
        albums,
        artists,
    })
}

fn merge_removed(profile_uid: &str, lib_uid: &str, removed: &Removed) -> Result<(), String> {
    match library_manager::sync_entities(
        profile_uid,
        lib_uid,
        &removed.tracks,
        &removed.albums,
        &removed.artists,
    ) {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("[paths] targeted merge failed, falling back to full update: {e}");
            library_manager::incremental_update(profile_uid, &[lib_uid.to_string()])
        }
    }
}

#[tauri::command]
pub async fn remove_path(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let uid = state.get_uid();
    library_manager::run_blocking(move || remove_path_blocking(&app, &uid, &path)).await
}

fn remove_path_blocking(app: &AppHandle, uid: &str, raw_path: &str) -> Result<(), String> {
    let key = path_key(raw_path);
    if key.is_empty() {
        return Err("Path is empty".to_string());
    }

    let settings_conn = open_settings_conn(uid);
    let all_libs =
        crate::db::library_registry::get_all_libraries(&settings_conn).map_err(|e| e.to_string())?;

    let mut target: Option<crate::db::library_registry::Library> = None;
    for lib in &all_libs {
        if let Ok(lc) = library_manager::open_tuned(&lib.file_path) {
            let has_path = get_paths_from_lib_db(&lc)
                .unwrap_or_default()
                .iter()
                .any(|p| path_key(&p.path) == key);
            if has_path {
                target = Some(lib.clone());
                break;
            }
        }
    }

    if target.is_none() {
        if let Some(luid) = get_lib_uid_for_path(&settings_conn, &normalize_path(raw_path)) {
            target = all_libs.iter().find(|l| l.uid == luid).cloned();
        }
    }

    match target {
        Some(lib) => {
            crate::scanner::cancel_scans_for_file(&lib.file_path);
            let lib_conn = library_manager::open_tuned(&lib.file_path)?;
            let removed = remove_tracks_and_orphans(&lib_conn, &key, true)?;

            let remaining: Vec<String> = get_paths_from_lib_db(&lib_conn)
                .unwrap_or_default()
                .into_iter()
                .map(|p| p.path)
                .collect();

            merge_removed(uid, &lib.uid, &removed)?;

            if remaining.is_empty() {
                crate::watcher::stop_watcher(uid, &lib.uid);
            } else {
                crate::watcher::start_watcher(app.clone(), uid.to_string(), lib.uid.clone(), remaining);
            }
        }
        None => {
            let lib_conn = open_lib_conn_legacy(uid)?;
            delete_tracks_under_path(&lib_conn, &key).map_err(|e| e.to_string())?;
        }
    }

    app.emit("library:updated", ()).ok();
    Ok(())
}

fn open_lib_conn_legacy(uid: &str) -> Result<Connection, String> {
    library_manager::open_tuned(get_lib_db_path(uid))
}

fn find_library_for_path(
    uid: &str,
    key: &str,
) -> Result<crate::db::library_registry::Library, String> {
    let settings_conn = open_settings_conn(uid);
    let all_libs =
        crate::db::library_registry::get_all_libraries(&settings_conn).map_err(|e| e.to_string())?;
    for lib in all_libs {
        if let Ok(lc) = library_manager::open_tuned(&lib.file_path) {
            let has_path = get_paths_from_lib_db(&lc)
                .unwrap_or_default()
                .iter()
                .any(|p| path_key(&p.path) == key);
            if has_path {
                return Ok(lib);
            }
        }
    }
    Err("This folder is not registered in any library".to_string())
}

#[tauri::command]
pub fn rescan_path_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    mode: String,
) -> Result<(), String> {
    let uid = state.get_uid();
    let key = path_key(&path);
    if key.is_empty() {
        return Err("Path is empty".to_string());
    }
    if mode != "new" && mode != "full" && mode != "clear" {
        return Err(format!("Unknown rescan mode: {}", mode));
    }

    let lib = find_library_for_path(&uid, &key)?;
    if mode == "clear" {
        crate::scanner::cancel_scans_for_file(&lib.file_path);
    }
    let scan_path = if crate::webdav::is_http_url(&path) {
        crate::webdav::encode_url_spaces(&path)
    } else {
        normalize_path(&path)
    };

    std::thread::spawn(move || {
        let lib_conn = match library_manager::open_tuned(&lib.file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[rescan_path] open library failed: {e}");
                app.emit("scan:done", ()).ok();
                return;
            }
        };
        let settings_path = get_settings_db_path(&uid);
        lib_conn
            .execute_batch(&format!(
                "ATTACH DATABASE '{}' AS settings;",
                settings_path.to_string_lossy().replace('\'', "''")
            ))
            .ok();

        let before_scan = snapshot_entity_uids(&uid);
        if mode == "clear" {
            match remove_tracks_and_orphans(&lib_conn, &key, false) {
                Ok(removed) => {
                    if let Err(e) = merge_removed(&uid, &lib.uid, &removed) {
                        eprintln!("[rescan_path] merge after clear failed: {e}");
                    }
                }
                Err(e) => {
                    eprintln!("[rescan_path] clear failed: {e}");
                    app.emit("scan:error", format!("Clear failed: {e}")).ok();
                    app.emit("scan:done", ()).ok();
                    return;
                }
            }
            app.emit("library:updated", ()).ok();
        }

        crate::scanner::scan_directory_with_creds(
            &lib_conn,
            &scan_path,
            &app,
            mode == "new",
            lib.write_token.as_deref(),
        );

        if let Err(e) = library_manager::incremental_update(&uid, &[lib.uid.clone()]) {
            eprintln!("[rescan_path] merge failed: {e}");
        }
        app.emit("library:updated", ()).ok();
        let added = entities_added_since(&uid, &before_scan);
        spawn_post_scan_tasks(app.clone(), uid.clone(), added);

        let remaining: Vec<String> = get_paths_from_lib_db(&lib_conn)
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.path)
            .collect();
        if !remaining.is_empty() {
            crate::watcher::start_watcher(app, uid, lib.uid.clone(), remaining);
        }
    });

    Ok(())
}

#[tauri::command]
pub fn get_paths(state: State<AppState>) -> Result<Vec<LibraryPath>, String> {
    let uid = state.get_uid();
    let settings_conn = open_settings_conn(&uid);
    let all_libs = crate::db::library_registry::get_all_libraries(&settings_conn)
        .map_err(|e| e.to_string())?;

    let mut all_paths = Vec::new();
    for lib in &all_libs {
        if let Ok(lc) = Connection::open(&lib.file_path) {
            if let Ok(paths) = get_paths_from_lib_db(&lc) {
                all_paths.extend(paths);
            }
        }
    }
    Ok(all_paths)
}

#[tauri::command]
pub fn get_paths_for_library(
    state: State<AppState>,
    lib_uid: String,
) -> Result<Vec<LibraryPath>, String> {
    let uid = state.get_uid();
    let settings_conn = open_settings_conn(&uid);
    let lib = crate::db::library_registry::get_library_by_uid(&settings_conn, &lib_uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Library not found: {}", lib_uid))?;

    let lib_conn = Connection::open(&lib.file_path).map_err(|e| e.to_string())?;
    get_paths_from_lib_db(&lib_conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rescan(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let uid = state.get_uid();
    let settings_conn = open_settings_conn(&uid);
    let all_libs = crate::db::library_registry::get_all_libraries(&settings_conn)
        .map_err(|e| e.to_string())?;

	let before_scan = snapshot_entity_uids(&uid);
	let mut handles = Vec::new();

    for lib in all_libs {
        let lib_file_path = lib.file_path.clone();
        let lib_uid = lib.uid.clone();
        let lib_token = lib.write_token.clone();
        let uid_clone = uid.clone();
        let app_clone = app.clone();

        let paths: Vec<String> = {
            if let Ok(lc) = Connection::open(&lib_file_path) {
                get_paths_from_lib_db(&lc)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| p.path)
                    .collect()
            } else {
                vec![]
            }
        };

        if paths.is_empty() {
            continue;
        }

		let paths_clone = paths.clone();
		let handle = std::thread::spawn(move || {
            let lib_conn = Connection::open(&lib_file_path).expect("Failed to open library db");
            let settings_path = get_settings_db_path(&uid_clone);
            lib_conn
                .execute_batch(&format!(
                    "ATTACH DATABASE '{}' AS settings;",
                    settings_path.to_string_lossy().replace('\'', "''")
                ))
                .ok();

            for p in &paths_clone {
                crate::scanner::scan_directory_with_creds(
                    &lib_conn,
                    p,
                    &app_clone,
                    false,
                    lib_token.as_deref(),
                );
            }

            let _ = library_manager::incremental_update(&uid_clone, &[lib_uid.clone()]);
            crate::watcher::start_watcher(app_clone, uid_clone, lib_uid, paths_clone);
        });
		handles.push(handle);
    }

	if handles.is_empty() {
		return Ok(());
	}

	let app_done = app.clone();
	let uid_done = uid.clone();
	std::thread::spawn(move || {
		for handle in handles {
			let _ = handle.join();
		}
		let added = entities_added_since(&uid_done, &before_scan);
		spawn_post_scan_tasks(app_done, uid_done, added);
	});

    Ok(())
}