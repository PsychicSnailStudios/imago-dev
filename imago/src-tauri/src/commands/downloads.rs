use crate::db;
use crate::library_manager;
use crate::state::AppState;
use crate::{open_merged_conn, open_settings_conn};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_shell::ShellExt;

#[tauri::command]
pub async fn download_track_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    uid: String,
) -> Result<String, String> {
    let profile_uid = state.get_uid();
    let merged_conn = open_merged_conn(&profile_uid);
    let settings_conn = open_settings_conn(&profile_uid);

    let track = db::get_track_by_uid(&merged_conn, &uid)
        .map_err(|e| e.to_string())?
        .ok_or("Track not found")?;

    let remote_path = track
        .remote_path
        .clone()
        .ok_or("Track has no remote path")?;

    let download_base = db::settings_manager::get_setting(&settings_conn, "download_path")
        .ok()
        .flatten()
        .unwrap_or_default();
    if download_base.is_empty() {
        return Err("Download path not configured".to_string());
    }

    let path_style = db::settings_manager::get_setting(&settings_conn, "download_path_style")
        .ok()
        .flatten()
        .unwrap_or_else(|| "{artist}/{album}".to_string());

    let filename_style =
        db::settings_manager::get_setting(&settings_conn, "download_filename_style")
            .ok()
            .flatten()
            .unwrap_or_else(|| "{track_number} - {title}".to_string());

    let convert_mp3 = db::settings_manager::get_setting(&settings_conn, "download_convert_mp3")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false);

    let artist = db::resolved_artist_name(&track.album_artist, &track.artists)
        .unwrap_or_else(|| "Unknown Artist".to_string());

    let album = db::first_album_name(&track.albums).unwrap_or_else(|| "Unknown Album".to_string());

    let track_number = db::first_track_number(&track.albums)
        .map(|n| format!("{:02}", n))
        .unwrap_or_else(|| "00".to_string());

    let year = track
        .year
        .clone()
        .unwrap_or_else(|| "Unknown Year".to_string());
    let title = track
        .title
        .clone()
        .unwrap_or_else(|| "Unknown Title".to_string());

    let sanitize = |s: &str| -> String {
        s.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                c => c,
            })
            .collect()
    };

    let folder = path_style
        .replace("{artist}", &sanitize(&artist))
        .replace("{album}", &sanitize(&album))
        .replace("{year}", &sanitize(&year));

    let ext = if convert_mp3 {
        "mp3".to_string()
    } else {
        remote_path
            .rsplit('.')
            .next()
            .unwrap_or("mp3")
            .to_string()
            .to_lowercase()
    };

    let filename_base = filename_style
        .replace("{track_number}", &track_number)
        .replace("{title}", &sanitize(&title))
        .replace("{artist}", &sanitize(&artist))
        .replace("{album}", &sanitize(&album))
        .replace("{year}", &sanitize(&year));

    let dest_dir = PathBuf::from(&download_base).join(&folder);
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let dest_path = dest_dir.join(format!("{}.{}", filename_base, ext));
    let dest_str = dest_path.to_string_lossy().to_string();

    if convert_mp3 {
        let ffmpeg_output = app
            .shell()
            .sidecar("ffmpeg")
            .map_err(|e| e.to_string())?
            .args(["-y", "-i", &remote_path, "-b:a", "320k", &dest_str])
            .output()
            .await
            .map_err(|e| e.to_string())?;

        if !ffmpeg_output.status.success() {
            let stderr = String::from_utf8_lossy(&ffmpeg_output.stderr).to_string();
            return Err(format!("FFmpeg failed: {}", stderr));
        }
    } else if remote_path.starts_with("http://") || remote_path.starts_with("https://") {
        let client = crate::enrichment::make_client()?;
        let bytes = client
            .get(&remote_path)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .bytes()
            .await
            .map_err(|e| e.to_string())?;
        std::fs::write(&dest_path, &bytes).map_err(|e| e.to_string())?;
    } else {
        std::fs::copy(&remote_path, &dest_path).map_err(|e| e.to_string())?;
    }

    let format = ext.to_uppercase();
    let bitrate: Option<i64> = if let Ok(metadata) = std::fs::metadata(&dest_path) {
        let duration_secs = track.duration_ms.unwrap_or(0) as f64 / 1000.0;
        if duration_secs > 0.0 {
            Some(((metadata.len() as f64 * 8.0) / duration_secs / 1000.0) as i64)
        } else {
            None
        }
    } else {
        None
    };

    let track_data_json = serde_json::to_string(&serde_json::json!({
        "format": format.clone(),
        "bitrate": bitrate,
        "is_ghost": false,
    }))
    .ok();

    let meta_update = db::MetadataUpdate {
        format: Some(format),
        bitrate,
        track_data: track_data_json,
        ..Default::default()
    };

    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "tracks") {
        Ok((source_conn, lib)) => {
            db::update_track_metadata_by_uid(&source_conn, &uid, &meta_update)
                .map_err(|e| e.to_string())?;
            source_conn
                .execute(
                    "UPDATE tracks SET path = ?1 WHERE uid = ?2",
                    rusqlite::params![dest_str, uid],
                )
                .map_err(|e| e.to_string())?;
            library_manager::incremental_update(&profile_uid, &[lib.uid])
                .map_err(|e| e.to_string())?;
        }
        Err(_) => {
            let lib_conn = crate::open_lib_conn(&profile_uid);
            db::update_track_metadata_by_uid(&lib_conn, &uid, &meta_update)
                .map_err(|e| e.to_string())?;
            lib_conn
                .execute(
                    "UPDATE tracks SET path = ?1 WHERE uid = ?2",
                    rusqlite::params![dest_str, uid],
                )
                .map_err(|e| e.to_string())?;
        }
    }

    app.emit("library:updated", ()).ok();
    Ok(dest_str)
}

#[tauri::command]
pub async fn download_album_tracks_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    uid: String,
    subscribe: bool,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let merged_conn = open_merged_conn(&profile_uid);

    let album = db::get_album_by_uid(&merged_conn, &uid)
        .map_err(|e| e.to_string())?
        .ok_or("Album not found")?;

    let track_uids: Vec<String> = album
        .tracks
        .as_deref()
        .and_then(|t| serde_json::from_str::<Vec<serde_json::Value>>(t).ok())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| e["uid"].as_str().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect();

    for track_uid in track_uids {
        let track = match db::get_track_by_uid(&merged_conn, &track_uid) {
            Ok(Some(t)) => t,
            _ => continue,
        };
        if track.remote_path.is_none() {
            continue;
        }
        let local_path_ok = !track.path.is_empty()
            && !track.path.starts_with("t-")
            && std::path::Path::new(&track.path).exists();
        if local_path_ok {
            continue;
        }
        let _ = download_track_cmd(app.clone(), state.clone(), track_uid).await;
    }

    if subscribe {
        let settings_conn = open_settings_conn(&profile_uid);
        let mut subs: Vec<String> =
            db::settings_manager::get_setting(&settings_conn, "offline_subscriptions")
                .ok()
                .flatten()
                .and_then(|v| serde_json::from_str(&v).ok())
                .unwrap_or_default();
        if !subs.contains(&uid) {
            subs.push(uid);
            db::settings_manager::set_setting(
                &settings_conn,
                "offline_subscriptions",
                &serde_json::to_string(&subs).unwrap_or_else(|_| "[]".to_string()),
            )
            .ok();
        }
    }

    app.emit("library:updated", ()).ok();
    Ok(())
}

#[tauri::command]
pub async fn download_playlist_tracks_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    uid: String,
    subscribe: bool,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let merged_conn = open_merged_conn(&profile_uid);

    let playlist = db::get_playlist_by_uid(&merged_conn, &uid)
        .map_err(|e| e.to_string())?
        .ok_or("Playlist not found")?;

    let track_uids: Vec<String> = playlist
        .tracks
        .as_deref()
        .and_then(|t| serde_json::from_str::<Vec<serde_json::Value>>(t).ok())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| e["uid"].as_str().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect();

    for track_uid in track_uids {
        let track = match db::get_track_by_uid(&merged_conn, &track_uid) {
            Ok(Some(t)) => t,
            _ => continue,
        };
        if track.remote_path.is_none() {
            continue;
        }
        let local_path_ok = !track.path.is_empty()
            && !track.path.starts_with("t-")
            && std::path::Path::new(&track.path).exists();
        if local_path_ok {
            continue;
        }
        let _ = download_track_cmd(app.clone(), state.clone(), track_uid).await;
    }

    if subscribe {
        let settings_conn = open_settings_conn(&profile_uid);
        let mut subs: Vec<String> =
            db::settings_manager::get_setting(&settings_conn, "offline_subscriptions")
                .ok()
                .flatten()
                .and_then(|v| serde_json::from_str(&v).ok())
                .unwrap_or_default();
        if !subs.contains(&uid) {
            subs.push(uid);
            db::settings_manager::set_setting(
                &settings_conn,
                "offline_subscriptions",
                &serde_json::to_string(&subs).unwrap_or_else(|_| "[]".to_string()),
            )
            .ok();
        }
    }

    app.emit("library:updated", ()).ok();
    Ok(())
}

#[tauri::command]
pub async fn sync_offline_subscriptions_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let settings_conn = open_settings_conn(&profile_uid);

    let subs: Vec<String> =
        db::settings_manager::get_setting(&settings_conn, "offline_subscriptions")
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_str(&v).ok())
            .unwrap_or_default();

    for sub_uid in subs {
        match sub_uid.chars().take(2).collect::<String>().as_str() {
            "a-" => {
                let _ = download_album_tracks_cmd(app.clone(), state.clone(), sub_uid, false).await;
            }
            "p-" => {
                let _ =
                    download_playlist_tracks_cmd(app.clone(), state.clone(), sub_uid, false).await;
            }
            _ => {}
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn unsubscribe_offline_cmd(
    state: State<'_, AppState>,
    uid: String,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let settings_conn = open_settings_conn(&profile_uid);

    let mut subs: Vec<String> =
        db::settings_manager::get_setting(&settings_conn, "offline_subscriptions")
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_str(&v).ok())
            .unwrap_or_default();

    subs.retain(|s| s != &uid);

    db::settings_manager::set_setting(
        &settings_conn,
        "offline_subscriptions",
        &serde_json::to_string(&subs).unwrap_or_else(|_| "[]".to_string()),
    )
    .map_err(|e| e.to_string())
}
