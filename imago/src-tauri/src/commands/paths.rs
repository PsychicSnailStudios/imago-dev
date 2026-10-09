use crate::db::settings_manager::LibraryPath;
use crate::library_manager;
use crate::profiles::{get_lib_db_path, get_library_db_path, get_settings_db_path};
use crate::state::AppState;
use crate::{open_local_library_conn, open_settings_conn};
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

fn get_paths_from_lib_db(conn: &Connection) -> rusqlite::Result<Vec<LibraryPath>> {
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

fn spawn_post_scan_tasks(app: AppHandle, profile_uid: String) {
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
		let state = app.state::<AppState>();
		if auto_tracks {
			let _ = crate::commands::enrichment::enrich_all(app.clone(), state.clone()).await;
		}
		if auto_albums {
			let _ =
				crate::commands::enrichment::enrich_all_albums(app.clone(), state.clone()).await;
		}
		if auto_artists {
			let _ =
				crate::commands::enrichment::enrich_all_artists(app.clone(), state.clone()).await;
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
    let path = normalize_path(&path);
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

    {
        let lib_conn = Connection::open(&lib_file_path).map_err(|e| e.to_string())?;
        crate::db::init_library_db(&lib_conn).map_err(|e| e.to_string())?;
        add_path_to_lib_db(&lib_conn, &path).map_err(|e| e.to_string())?;
    }

    let app_clone = app.clone();
    let path_clone = path.clone();
    let uid_clone = uid.clone();
    let lib_uid_clone = target_lib_uid.clone();
    let lib_file_path_clone = lib_file_path.clone();

    std::thread::spawn(move || {
        let lib_conn = Connection::open(&lib_file_path_clone).expect("Failed to open library db");
        let settings_path = get_settings_db_path(&uid_clone);
        lib_conn
            .execute_batch(&format!(
                "ATTACH DATABASE '{}' AS settings;",
                settings_path.to_string_lossy().replace('\'', "''")
            ))
            .ok();
        crate::scanner::scan_directory_with_progress(&lib_conn, &path_clone, &app_clone);

        let _ = library_manager::incremental_update(&uid_clone, &[lib_uid_clone.clone()]);

		spawn_post_scan_tasks(app_clone.clone(), uid_clone.clone());

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

fn delete_tracks_under_path(conn: &Connection, key: &str) -> rusqlite::Result<usize> {
    let cond = format!(
        "({n} = ?1 OR substr({n}, 1, length(?1) + 1) = ?1 || '/')",
        n = NORMALISED_TRACK_PATH
    );
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

fn delete_orphans(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM albums WHERE uid NOT IN (
            SELECT DISTINCT json_extract(json_each.value, '$.uid')
            FROM tracks, json_each(tracks.albums)
            WHERE json_extract(json_each.value, '$.uid') IS NOT NULL
            AND json_extract(json_each.value, '$.uid') != ''
        )",
        [],
    )?;
    conn.execute(
        "DELETE FROM artists WHERE name NOT IN (
            SELECT DISTINCT json_each.value
            FROM tracks, json_each(tracks.artists)
            WHERE tracks.artists IS NOT NULL AND tracks.artists != '[]'
        ) AND name NOT IN (
            SELECT DISTINCT album_artist FROM tracks WHERE album_artist IS NOT NULL
        )",
        [],
    )?;
    Ok(())
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
            let lib_conn = library_manager::open_tuned(&lib.file_path)?;
            {
                let tx = lib_conn.unchecked_transaction().map_err(|e| e.to_string())?;
                delete_library_path_row(&lib_conn, &key).map_err(|e| e.to_string())?;
                delete_tracks_under_path(&lib_conn, &key).map_err(|e| e.to_string())?;
                delete_orphans(&lib_conn).map_err(|e| e.to_string())?;
                tx.commit().map_err(|e| e.to_string())?;
            }

            let remaining: Vec<String> = get_paths_from_lib_db(&lib_conn)
                .unwrap_or_default()
                .into_iter()
                .map(|p| p.path)
                .collect();

            library_manager::incremental_update(uid, &[lib.uid.clone()])?;

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
    let scan_path = normalize_path(&path);

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

        if mode == "clear" {
            let cleared = lib_conn
                .unchecked_transaction()
                .and_then(|tx| {
                    delete_tracks_under_path(&lib_conn, &key)?;
                    delete_orphans(&lib_conn)?;
                    tx.commit()
                });
            if let Err(e) = cleared {
                eprintln!("[rescan_path] clear failed: {e}");
                app.emit("scan:done", ()).ok();
                return;
            }
            if let Err(e) = library_manager::incremental_update(&uid, &[lib.uid.clone()]) {
                eprintln!("[rescan_path] merge after clear failed: {e}");
            }
            app.emit("library:updated", ()).ok();
        }

        if mode == "new" {
            crate::scanner::scan_directory_new_only(&lib_conn, &scan_path, &app);
        } else {
            crate::scanner::scan_directory_with_progress(&lib_conn, &scan_path, &app);
        }

        if let Err(e) = library_manager::incremental_update(&uid, &[lib.uid.clone()]) {
            eprintln!("[rescan_path] merge failed: {e}");
        }
        app.emit("library:updated", ()).ok();
        spawn_post_scan_tasks(app.clone(), uid.clone());

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

	let mut handles = Vec::new();

    for lib in all_libs {
        let lib_file_path = lib.file_path.clone();
        let lib_uid = lib.uid.clone();
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
                crate::scanner::scan_directory_with_progress(&lib_conn, p, &app_clone);
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
		spawn_post_scan_tasks(app_done, uid_done);
	});

    Ok(())
}