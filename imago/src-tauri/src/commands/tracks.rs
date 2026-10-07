use crate::db::{self, get_all_tracks, update_track_metadata_by_uid, MetadataUpdate, Track};
use crate::library_manager;
use crate::profiles::get_settings_db_path;
use crate::state::AppState;
use crate::{open_lib_conn, open_local_library_conn, open_merged_conn, open_settings_conn};
use tauri::{AppHandle, Emitter, State};

fn blocking_err<T>(r: Result<T, rusqlite::Error>) -> Result<T, String> {
    r.map_err(|e| e.to_string())
}

fn ensure_tag(conn: &rusqlite::Connection, tags: &Option<String>, genres: &Option<String>) {
    if let Some(ref s) = tags {
        if let Ok(names) = serde_json::from_str::<Vec<String>>(s) {
            for name in names {
                crate::db::tag_manager::ensure_tag(
                    conn,
                    &name,
                    crate::db::tag_manager::TagKind::Tag,
                );
            }
        }
    }
    if let Some(ref s) = genres {
        if let Ok(names) = serde_json::from_str::<Vec<String>>(s) {
            for name in names {
                crate::db::tag_manager::ensure_tag(
                    conn,
                    &name,
                    crate::db::tag_manager::TagKind::Genre,
                );
            }
        }
    }
}

#[tauri::command]
pub async fn get_tracks(state: State<'_, AppState>) -> Result<Vec<Track>, String> {
    let uid = state.get_uid();
    library_manager::run_blocking(move || {
        let conn = open_merged_conn(&uid);
        blocking_err(get_all_tracks(&conn))
    })
    .await
}

#[tauri::command]
pub fn get_track(state: State<AppState>, uid: String) -> Result<Option<Track>, String> {
    let profile_uid = state.get_uid();
    let conn = open_merged_conn(&profile_uid);
    db::get_track_by_uid(&conn, &uid).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_track_artwork(state: State<AppState>, uid: String) -> Result<Option<Vec<u8>>, String> {
    let profile_uid = state.get_uid();
    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "tracks") {
        Ok((conn, _)) => db::get_track_artwork_by_uid(&conn, &uid).map_err(|e| e.to_string()),
        Err(_) => {
            let conn = open_merged_conn(&profile_uid);
            db::get_track_artwork_by_uid(&conn, &uid).map_err(|e| e.to_string())
        }
    }
}

#[tauri::command]
pub async fn get_duplicates(state: State<'_, AppState>) -> Result<Vec<db::DuplicateGroup>, String> {
    let uid = state.get_uid();
    library_manager::run_blocking(move || {
        let conn = open_merged_conn(&uid);
        blocking_err(db::find_duplicates(&conn))
    })
    .await
}

#[tauri::command]
pub async fn update_track_metadata(
    state: State<'_, AppState>,
    uid: String,
    update: MetadataUpdate,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || {
        match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "tracks") {
            Ok((source_conn, lib)) => {
                update_track_metadata_by_uid(&source_conn, &uid, &update).map_err(|e| e.to_string())?;
                ensure_tag(&source_conn, &update.tags, &update.genres);
                library_manager::sync_track(&profile_uid, &lib.uid, &uid)?;
            }
            Err(_) => {
                let conn = open_lib_conn(&profile_uid);
                update_track_metadata_by_uid(&conn, &uid, &update).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn write_track_tags(
    app: AppHandle,
    state: State<'_, AppState>,
    uid: String,
    path: String,
    update: MetadataUpdate,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || write_track_tags_blocking(app, profile_uid, uid, path, update)).await
}

fn write_track_tags_blocking(
    app: AppHandle,
    profile_uid: String,
    uid: String,
    path: String,
    update: MetadataUpdate,
) -> Result<(), String> {
    use lofty::prelude::*;
    use lofty::probe::Probe;

    let mut tagged_file = Probe::open(&path)
        .map_err(|e| e.to_string())?
        .guess_file_type()
        .map_err(|e| e.to_string())?
        .read()
        .map_err(|e| e.to_string())?;

    let has_primary = tagged_file.primary_tag().is_some();
    let tag = if has_primary {
        tagged_file.primary_tag_mut()
    } else {
        tagged_file.first_tag_mut()
    }
    .ok_or("No tag found in file")?;

    if let Some(title) = &update.title {
        tag.set_title(title.clone());
    }
    if let Some(ref artists_json) = &update.artists {
        if let Ok(names) = serde_json::from_str::<Vec<String>>(artists_json) {
            tag.set_artist(names.join(", "));
        }
    }
    if let Some(ref album_artist) = &update.album_artist {
        tag.insert(lofty::tag::TagItem::new(
            lofty::tag::ItemKey::AlbumArtist,
            lofty::tag::ItemValue::Text(album_artist.clone()),
        ));
    }
    if let Some(ref albums_json) = &update.albums {
        if let Ok(albums) = serde_json::from_str::<Vec<serde_json::Value>>(albums_json) {
            if let Some(name) = albums
                .first()
                .and_then(|a| a.get("name"))
                .and_then(|n| n.as_str())
            {
                tag.set_album(name.to_string());
            }
        }
    }
    if let Some(year) = &update.year {
        if let Ok(y) = year.parse::<u32>() {
            tag.set_year(y);
        }
    }
    if let Some(ref genres_json) = &update.genres {
        if let Ok(genres) = serde_json::from_str::<Vec<String>>(genres_json) {
            tag.set_genre(genres.join("/"));
        }
    }
    if let Some(bpm) = update.bpm {
        tag.insert(lofty::tag::TagItem::new(
            lofty::tag::ItemKey::Bpm,
            lofty::tag::ItemValue::Text(bpm.to_string()),
        ));
    }

    tagged_file
        .save_to_path(&path, lofty::config::WriteOptions::default())
        .map_err(|e| e.to_string())?;

    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "tracks") {
        Ok((source_conn, lib)) => {
            update_track_metadata_by_uid(&source_conn, &uid, &update).map_err(|e| e.to_string())?;
            ensure_tag(&source_conn, &update.tags, &update.genres);
            library_manager::sync_track(&profile_uid, &lib.uid, &uid)?;
        }
        Err(_) => {
            let conn = open_lib_conn(&profile_uid);
            update_track_metadata_by_uid(&conn, &uid, &update).map_err(|e| e.to_string())?;
        }
    }

    app.emit("library:updated", ()).ok();
    Ok(())
}

#[tauri::command]
pub async fn remove_track_from_library(state: State<'_, AppState>, uid: String) -> Result<(), String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || {
        match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "tracks") {
            Ok((source_conn, lib)) => {
                db::delete_track_by_uid(&source_conn, &uid).map_err(|e| e.to_string())?;
                library_manager::sync_track(&profile_uid, &lib.uid, &uid)?;
            }
            Err(_) => {
                let conn = open_lib_conn(&profile_uid);
                db::delete_track_by_uid(&conn, &uid).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn delete_track_file(
    app: AppHandle,
    state: State<'_, AppState>,
    uid: String,
    path: String,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "tracks") {
            Ok((source_conn, lib)) => {
                db::delete_track_by_uid(&source_conn, &uid).map_err(|e| e.to_string())?;
                library_manager::sync_track(&profile_uid, &lib.uid, &uid)?;
            }
            Err(_) => {
                let conn = open_lib_conn(&profile_uid);
                db::delete_track_by_uid(&conn, &uid).map_err(|e| e.to_string())?;
            }
        }
        app.emit("library:updated", ()).ok();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn merge_remote_local_tracks(
    app: AppHandle,
    state: State<'_, AppState>,
    keep_uid: String,
    drop_uid: String,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || {
        match library_manager::open_source_conn_for_entity(&profile_uid, &keep_uid, "tracks") {
            Ok((source_conn, lib)) => {
                let drop_track = db::get_track_by_uid(&source_conn, &drop_uid)
                    .map_err(|e| e.to_string())?
                    .ok_or("Drop track not found")?;
                crate::scanner::merge_paths_into_existing(&source_conn, &keep_uid, &drop_track);
                db::delete_track_by_uid(&source_conn, &drop_uid).map_err(|e| e.to_string())?;
                library_manager::sync_entities(
                    &profile_uid,
                    &lib.uid,
                    &[keep_uid.clone(), drop_uid.clone()],
                    &[],
                    &[],
                )?;
            }
            Err(_) => {
                let conn = open_lib_conn(&profile_uid);
                let drop_track = db::get_track_by_uid(&conn, &drop_uid)
                    .map_err(|e| e.to_string())?
                    .ok_or("Drop track not found")?;
                crate::scanner::merge_paths_into_existing(&conn, &keep_uid, &drop_track);
                db::delete_track_by_uid(&conn, &drop_uid).map_err(|e| e.to_string())?;
            }
        }
        app.emit("library:updated", ()).ok();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn replace_track_path(
    app: AppHandle,
    state: State<'_, AppState>,
    uid: String,
    new_path: String,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let new_path = new_path.replace('\\', "/");
    let settings_conn = open_settings_conn(&profile_uid);

    let g = |key: &str, default: &str| -> String {
        crate::db::settings_manager::get_setting(&settings_conn, key)
            .ok()
            .flatten()
            .unwrap_or_else(|| default.to_string())
    };

    let priority_title = g("filename_priority_title", "tag");
    let priority_artist = g("filename_priority_artist", "tag");
    let priority_album = g("filename_priority_album", "tag");
    let priority_year = g("filename_priority_year", "tag");
    let custom_pattern = g("filename_custom_pattern", "");
    let tag_delim_raw = g("artist_tag_delimiters", " / |; ");
    let filename_delim_raw = g(
        "artist_filename_delimiters",
        " / |; | feat. | ft. | featuring ",
    );
    let genre_delim_raw = g("genre_delimiters", " / |; |, ");
    let tag_delims: Vec<String> = tag_delim_raw.split('|').map(|s| s.to_string()).collect();
    let filename_delims: Vec<String> = filename_delim_raw
        .split('|')
        .map(|s| s.to_string())
        .collect();
    let genre_delims: Vec<String> = genre_delim_raw.split('|').map(|s| s.to_string()).collect();
    let tag_delims_ref: Vec<&str> = tag_delims.iter().map(|s| s.as_str()).collect();
    let filename_delims_ref: Vec<&str> = filename_delims.iter().map(|s| s.as_str()).collect();
    let genre_delims_ref: Vec<&str> = genre_delims.iter().map(|s| s.as_str()).collect();
    let try_ampersand = g("scan_try_parse_ampersand", "true") == "true";

    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "tracks") {
        Ok((source_conn, lib)) => {
            source_conn
                .execute(
                    "UPDATE tracks SET path = ?1 WHERE uid = ?2",
                    rusqlite::params![new_path, uid],
                )
                .map_err(|e| e.to_string())?;

            let path = std::path::Path::new(&new_path);
            if let Some(fresh_track) = crate::scanner::read_track_with_settings(
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
            ) {
                let meta = MetadataUpdate {
                    title: fresh_track.title,
                    artists: fresh_track.artists,
                    album_artist: fresh_track.album_artist,
                    albums: fresh_track.albums,
                    year: fresh_track.year,
                    genres: fresh_track.genres,
                    bpm: fresh_track.bpm,
                    rating: fresh_track.rating,
                    key: fresh_track.key,
                    credits: fresh_track.credits,
                    label: fresh_track.label,
                    artwork_blob: fresh_track.artwork_blob,
                    format: fresh_track.format,
                    bitrate: fresh_track.bitrate,
                    ..Default::default()
                };
                update_track_metadata_by_uid(&source_conn, &uid, &meta)
                    .map_err(|e| e.to_string())?;
            }
            library_manager::sync_track(&profile_uid, &lib.uid, &uid)?;
        }
        Err(_) => {
            let lib_conn = open_lib_conn(&profile_uid);
            lib_conn
                .execute(
                    "UPDATE tracks SET path = ?1 WHERE uid = ?2",
                    rusqlite::params![new_path, uid],
                )
                .map_err(|e| e.to_string())?;
        }
    }

    app.emit("library:updated", ()).ok();
    Ok(())
}

#[tauri::command]
pub fn add_uid_remap(
    state: State<AppState>,
    old_uid: String,
    new_uid: String,
    entity_type: String,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let conn = open_settings_conn(&profile_uid);
    crate::db::settings_manager::add_uid_remap(&conn, &old_uid, &new_uid, &entity_type)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resolve_uid(state: State<AppState>, uid: String) -> Result<String, String> {
    let profile_uid = state.get_uid();
    let conn = open_settings_conn(&profile_uid);
    crate::db::settings_manager::resolve_uid(&conn, &uid).map_err(|e| e.to_string())
}
#[tauri::command]
pub fn get_tracks_by_uids(state: State<AppState>, uids: Vec<String>) -> Result<Vec<Track>, String> {
    let profile_uid = state.get_uid();
    let conn = open_merged_conn(&profile_uid);
    crate::db::search_manager::get_tracks_by_uids(&conn, &uids).map_err(|e| e.to_string())
}
