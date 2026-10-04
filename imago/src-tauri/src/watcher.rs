use crate::profiles::{get_library_db_path, get_settings_db_path};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "ogg", "wav", "aac", "m4a", "opus", "aiff"];

fn is_audio_file(path: &PathBuf) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn is_remote_path(path: &str) -> bool {
    path.starts_with("http://")
        || path.starts_with("https://")
        || path.starts_with("\\\\")
        || path.starts_with("//")
}

// profile_uid — the active profile
// lib_uid — which library db to write scanned tracks into
// paths — the folders to watch
pub fn start_watcher(app: AppHandle, profile_uid: String, lib_uid: String, paths: Vec<String>) {
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
        let mut watcher = RecommendedWatcher::new(
            tx,
            Config::default().with_poll_interval(Duration::from_secs(2)),
        )
        .expect("Failed to create watcher");

        for path in &paths {
            if let Err(e) = watcher.watch(PathBuf::from(path).as_path(), RecursiveMode::Recursive) {
                eprintln!("[watcher] Failed to watch {path}: {e}");
            } else {
                eprintln!("[watcher] Watching: {path}");
            }
        }

        let mut debounce: HashMap<PathBuf, (Instant, EventKind)> = HashMap::new();
        let debounce_duration = Duration::from_millis(3000);

        loop {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(Ok(event)) => {
                    let now = Instant::now();

                    for path in &event.paths {
                        if !is_audio_file(path) {
                            continue;
                        }

                        let prev = debounce.get(path);
                        let too_soon = prev
                            .map(|(last, _)| now.duration_since(*last) < debounce_duration)
                            .unwrap_or(false);

                        if too_soon {
                            let kind = match prev.map(|(_, k)| k) {
                                Some(EventKind::Create(_)) => event.kind.clone(),
                                _ => event.kind.clone(),
                            };
                            debounce.insert(path.clone(), (now, kind));
                            continue;
                        }

                        debounce.insert(path.clone(), (now, event.kind.clone()));
                    }
                }
                Ok(Err(e)) => eprintln!("[watcher] notify error: {e}"),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let now = Instant::now();
                    let ready: Vec<(PathBuf, EventKind)> = debounce
                        .iter()
                        .filter(|(_, (last, _))| now.duration_since(*last) >= debounce_duration)
                        .map(|(p, (_, k))| (p.clone(), k.clone()))
                        .collect();

                    for (path, kind) in ready {
                        debounce.remove(&path);
                        process_event(&app, &profile_uid, &lib_uid, &path, kind);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    eprintln!("[watcher] channel disconnected, exiting");
                    break;
                }
            }
        }
    });
}

fn process_event(
    app: &AppHandle,
    profile_uid: &str,
    lib_uid: &str,
    path: &PathBuf,
    kind: EventKind,
) {
    // Open the specific library db this watcher is assigned to
    let lib_path = get_library_db_path(profile_uid, lib_uid);
    let conn = match Connection::open(&lib_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[watcher] DB open error: {e}");
            return;
        }
    };

    // Attach settings.db so scanner setting reads work the same way
    let settings_path = get_settings_db_path(profile_uid);
    if let Err(e) = conn.execute_batch(&format!(
        "ATTACH DATABASE '{}' AS settings;",
        settings_path.to_string_lossy().replace('\'', "''")
    )) {
        eprintln!("[watcher] Failed to attach settings.db: {e}");
    }

    match kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            let path_str = path.to_string_lossy().to_string();

            // Wait for file size to stabilise before reading
            if !is_remote_path(&path_str) {
                let mut last_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                for _ in 0..6 {
                    std::thread::sleep(Duration::from_millis(500));
                    let current_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                    if current_size == last_size && current_size > 0 {
                        break;
                    }
                    last_size = current_size;
                }
                if !path.exists() || std::fs::metadata(path).map(|m| m.len()).unwrap_or(0) == 0 {
                    eprintln!("[watcher] Path gone or empty after wait: {:?}", path);
                    return;
                }
            }

            let g = |key: &str, default: &str| -> String {
                crate::db::get_setting(&conn, key)
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| default.to_string())
            };

            let priority_title = g("filename_priority_title", "tag");
            let priority_artist = g("filename_priority_artist", "tag");
            let priority_album = g("filename_priority_album", "tag");
            let priority_year = g("filename_priority_year", "tag");
            let custom_pattern = g("filename_custom_pattern", "");
            let try_ampersand = g("scan_try_parse_ampersand", "true") == "true";

            let tag_delim_raw = g("artist_tag_delimiters", " / |; |, |,");
            let filename_delim_raw = g(
                "artist_filename_delimiters",
                " / |; | feat. | ft. | featuring ",
            );
            let genre_delim_raw = g("genre_delimiters", " / |; |, ");

            let tag_delims: Vec<String> = tag_delim_raw
                .split('|')
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let filename_delims: Vec<String> = filename_delim_raw
                .split('|')
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let genre_delims: Vec<String> = genre_delim_raw
                .split('|')
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let tag_delims_ref: Vec<&str> = tag_delims.iter().map(|s| s.as_str()).collect();
            let filename_delims_ref: Vec<&str> =
                filename_delims.iter().map(|s| s.as_str()).collect();
            let genre_delims_ref: Vec<&str> = genre_delims.iter().map(|s| s.as_str()).collect();

            let track_opt = crate::scanner::read_track_with_settings(
                path,
                &priority_title,
                &priority_artist,
                &priority_album,
                &priority_year,
                &custom_pattern,
                &tag_delims_ref,
                &filename_delims_ref,
                &genre_delims_ref,
                try_ampersand,
            );

            match track_opt {
                Some(mut track) => {
                    if is_remote_path(&track.path) {
                        track.remote_path = Some(track.path.clone());
                        track.path = String::new();
                    }

                    if let Some(existing_uid) =
                        crate::scanner::find_remote_local_counterpart(&conn, &track)
                    {
                        crate::scanner::merge_paths_into_existing(&conn, &existing_uid, &track);
                        trigger_merge_update(app, profile_uid, lib_uid);
                        return;
                    }

                    match crate::db::upsert_track(&conn, &track) {
                        Ok(_) => {
                            crate::scanner::process_track(&conn, &track);

                            let tag_conn_owned = crate::scanner::default_library_conn_if_other(&conn);
                            let tag_conn: &Connection = tag_conn_owned.as_ref().unwrap_or(&conn);

                            if let Some(names) = track
                                .tags
                                .as_deref()
                                .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                            {
                                for name in &names {
                                    crate::db::tag_manager::ensure_tag(
                                        tag_conn,
                                        name,
                                        crate::db::tag_manager::TagKind::Tag,
                                    );
                                }
                            }
                            if let Some(names) = track
                                .genres
                                .as_deref()
                                .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                            {
                                for name in &names {
                                    crate::db::tag_manager::ensure_tag(
                                        tag_conn,
                                        name,
                                        crate::db::tag_manager::TagKind::Genre,
                                    );
                                }
                            }

                            eprintln!("[watcher] Upserted + processed: {:?}", path);
                            trigger_merge_update(app, profile_uid, lib_uid);
                        }
                        Err(e) => eprintln!("[watcher] upsert_track error: {e}"),
                    }
                }
                None => eprintln!("[watcher] read_track returned None for: {:?}", path),
            }
        }
        EventKind::Remove(_) => {
            let path_str = path.to_string_lossy().to_string();
            if is_remote_path(&path_str) {
                conn.execute(
                    "DELETE FROM tracks WHERE remote_path = ?1",
                    rusqlite::params![path_str],
                )
                .ok();
            } else {
                crate::db::delete_track(&conn, &path_str).ok();
            }
            trigger_merge_update(app, profile_uid, lib_uid);
            eprintln!("[watcher] Deleted: {:?}", path);
        }
        _ => {}
    }
}

// After a watcher write, trigger an incremental merge update for just this library
// so merged.db stays current without a full rebuild.
fn trigger_merge_update(app: &AppHandle, profile_uid: &str, lib_uid: &str) {
    let profile_uid = profile_uid.to_string();
    let lib_uid = lib_uid.to_string();
    let app = app.clone();

    tauri::async_runtime::spawn(async move {
        match crate::library_manager::incremental_update(&profile_uid, &[lib_uid]) {
            Ok(_) => {
                app.emit("library:updated", ()).ok();
            }
            Err(e) => {
                eprintln!("[watcher] merge update failed: {e}");
                // Still emit so the frontend knows something changed
                app.emit("library:updated", ()).ok();
            }
        }
    });
}