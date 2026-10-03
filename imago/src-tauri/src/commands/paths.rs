use crate::db::settings_manager::LibraryPath;
use crate::library_manager;
use crate::profiles::{get_lib_db_path, get_library_db_path, get_settings_db_path};
use crate::state::AppState;
use crate::{open_local_library_conn, open_settings_conn};
use rusqlite::Connection;
use tauri::Manager;
use tauri::{AppHandle, State};

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

#[tauri::command]
pub fn remove_path(state: State<AppState>, path: String) -> Result<(), String> {
    let path = normalize_path(&path);
    let uid = state.get_uid();
    let settings_conn = open_settings_conn(&uid);

    let lib_uid = get_lib_uid_for_path(&settings_conn, &path).or_else(|| {
        let all_libs =
            crate::db::library_registry::get_all_libraries(&settings_conn).unwrap_or_default();
        for lib in &all_libs {
            if let Ok(lc) = Connection::open(&lib.file_path) {
                let found: bool = lc
                    .query_row(
                        "SELECT 1 FROM library_paths WHERE path = ?1",
                        rusqlite::params![path],
                        |_| Ok(true),
                    )
                    .unwrap_or(false);
                if found {
                    return Some(lib.uid.clone());
                }
            }
        }
        None
    });

    if let Some(ref luid) = lib_uid {
        let lib_path = crate::db::library_registry::get_library_by_uid(&settings_conn, luid)
			.ok()
			.flatten()
			.map(|l| std::path::PathBuf::from(l.file_path))
			.unwrap_or_else(|| get_library_db_path(&uid, luid));
        
        if let Ok(lib_conn) = Connection::open(&lib_path) {
            remove_path_from_lib_db(&lib_conn, &path).ok();

            lib_conn
                .execute(
                    "DELETE FROM tracks WHERE path LIKE ?1",
                    rusqlite::params![format!("{}%", path)],
                )
                .ok();
            lib_conn
                .execute(
                    "DELETE FROM albums WHERE uid NOT IN (
					SELECT DISTINCT json_extract(json_each.value, '$.uid')
					FROM tracks, json_each(tracks.albums)
					WHERE json_extract(json_each.value, '$.uid') IS NOT NULL
					AND json_extract(json_each.value, '$.uid') != ''
				)",
                    [],
                )
                .ok();
            lib_conn
                .execute(
                    "DELETE FROM artists WHERE name NOT IN (
					SELECT DISTINCT json_each.value
					FROM tracks, json_each(tracks.artists)
					WHERE tracks.artists IS NOT NULL AND tracks.artists != '[]'
				) AND name NOT IN (
					SELECT DISTINCT album_artist FROM tracks WHERE album_artist IS NOT NULL
				)",
                    [],
                )
                .ok();
            let _ = library_manager::incremental_update(&uid, &[luid.clone()]);
        }
    } else {
        if let Ok(lib_conn) = Connection::open(get_lib_db_path(&uid)) {
            lib_conn
                .execute(
                    "DELETE FROM tracks WHERE path LIKE ?1",
                    rusqlite::params![format!("{}%", path)],
                )
                .ok();
        }
    }

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