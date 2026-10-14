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
            Err(e) => return Err(e),
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
        Err(e) => return Err(e),
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
            Err(e) => return Err(e),
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
            Err(e) => return Err(e),
        }
        app.emit("library:updated", ()).ok();
        Ok(())
    })
    .await
}

fn repoint_tracks_everywhere(
    profile_uid: &str,
    remaps: &std::collections::HashMap<String, (String, String)>,
) {
    if remaps.is_empty() {
        return;
    }

    let playlists_path = crate::profiles::get_playlists_db_path(profile_uid);
    if playlists_path.exists() {
        if let Ok(conn) = library_manager::open_tuned(&playlists_path) {
            let rows: Vec<(String, String)> = match conn
                .prepare("SELECT uid, tracks FROM playlists WHERE tracks IS NOT NULL")
            {
                Ok(mut stmt) => stmt
                    .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default(),
                Err(_) => Vec::new(),
            };
            let tx = conn.unchecked_transaction().ok();
            for (playlist_uid, json) in rows {
                let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&json) else {
                    continue;
                };
                let uid_of = |e: &serde_json::Value| -> String {
                    e.get("uid").and_then(|v| v.as_str()).unwrap_or("").to_string()
                };
                if !list.iter().any(|e| remaps.contains_key(&uid_of(e))) {
                    continue;
                }
                let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
                let mut rebuilt: Vec<serde_json::Value> = Vec::new();
                for mut entry in list {
                    let mut uid = uid_of(&entry);
                    if let Some((keep_uid, keep_name)) = remaps.get(&uid) {
                        if let Some(obj) = entry.as_object_mut() {
                            obj.insert("uid".into(), serde_json::Value::String(keep_uid.clone()));
                            obj.insert("name".into(), serde_json::Value::String(keep_name.clone()));
                        }
                        uid = keep_uid.clone();
                    }
                    if !seen.insert(uid) {
                        continue;
                    }
                    rebuilt.push(entry);
                }
                for (i, entry) in rebuilt.iter_mut().enumerate() {
                    if let Some(obj) = entry.as_object_mut() {
                        obj.insert("order".into(), serde_json::json!(i + 1));
                    }
                }
                if let Ok(out) = serde_json::to_string(&rebuilt) {
                    let _ = conn.execute(
                        "UPDATE playlists SET tracks = ?1 WHERE uid = ?2",
                        rusqlite::params![out, playlist_uid],
                    );
                }
            }
            if let Some(tx) = tx {
                let _ = tx.commit();
            }
        }
    }

    if let Ok(analytics) = crate::open_analytics_conn(profile_uid) {
        let _ = analytics.execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS track_remap (old_uid TEXT PRIMARY KEY, new_uid TEXT NOT NULL);
             DELETE FROM track_remap;",
        );
        if let Ok(tx) = analytics.unchecked_transaction() {
            for (old_uid, (new_uid, _)) in remaps {
                let _ = analytics.execute(
                    "INSERT OR REPLACE INTO track_remap (old_uid, new_uid) VALUES (?1, ?2)",
                    rusqlite::params![old_uid, new_uid],
                );
            }
            let _ = analytics.execute(
                "UPDATE scrobbles SET track_uid = (SELECT new_uid FROM track_remap WHERE old_uid = track_uid)
                 WHERE track_uid IN (SELECT old_uid FROM track_remap)",
                [],
            );
            let _ = tx.commit();
        }
    }

    let settings_conn = open_settings_conn(profile_uid);
    for (old_uid, (new_uid, _)) in remaps {
        let _ = db::settings_manager::add_uid_remap(&settings_conn, old_uid, new_uid, "track");
    }
}

fn cluster_by_album(tracks: Vec<Track>) -> Vec<Vec<Track>> {
    let mut clusters: Vec<Vec<Track>> = Vec::new();
    for track in tracks {
        let position = clusters.iter().position(|cluster| {
            cluster
                .iter()
                .any(|member| db::track_paths::same_album(&member.albums, &track.albums))
        });
        match position {
            Some(i) => clusters[i].push(track),
            None => clusters.push(vec![track]),
        }
    }
    clusters
}

#[derive(serde::Serialize)]
pub struct AutoMergeResult {
    pub merged: usize,
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
            Err(e) => return Err(e),
        }
        app.emit("library:updated", ()).ok();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn auto_merge_duplicates_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AutoMergeResult, String> {
    let profile_uid = state.get_uid();
    let result = library_manager::run_blocking(move || {
        let mut merged = 0usize;
        let mut remaps: std::collections::HashMap<String, (String, String)> =
            std::collections::HashMap::new();

        for lib in library_manager::writable_libraries(&profile_uid)? {
            let conn = library_manager::open_tuned(&lib.file_path)?;
            conn.busy_timeout(std::time::Duration::from_secs(60)).ok();
            let groups = db::find_duplicates(&conn).map_err(|e| e.to_string())?;

            let mut touched_tracks: Vec<String> = Vec::new();
            let mut touched_albums: Vec<String> = Vec::new();

            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            for group in groups {
                for cluster in cluster_by_album(group.tracks) {
                    if cluster.len() < 2 {
                        continue;
                    }
                    let keeper_idx = (0..cluster.len())
                        .max_by(|&a, &b| {
                            db::track_paths::keeper_score(&cluster[a])
                                .cmp(&db::track_paths::keeper_score(&cluster[b]))
                                .then(cluster[b].id.cmp(&cluster[a].id))
                        })
                        .unwrap_or(0);
                    let keeper = cluster[keeper_idx].clone();
                    let keeper_name = keeper.title.clone().unwrap_or_default();

                    for (i, other) in cluster.iter().enumerate() {
                        if i == keeper_idx {
                            continue;
                        }
                        match db::track_paths::merge_track_rows(&conn, &keeper.uid, &other.uid) {
                            Ok(albums) => {
                                merged += 1;
                                touched_tracks.push(other.uid.clone());
                                touched_albums.extend(albums);
                                remaps.insert(
                                    other.uid.clone(),
                                    (keeper.uid.clone(), keeper_name.clone()),
                                );
                            }
                            Err(e) => eprintln!("[merge] failed to merge {}: {e}", other.uid),
                        }
                    }
                    touched_tracks.push(keeper.uid.clone());
                }
            }
            tx.commit().map_err(|e| e.to_string())?;

            touched_tracks.sort();
            touched_tracks.dedup();
            touched_albums.sort();
            touched_albums.dedup();

            if let Err(e) = library_manager::sync_entities(
                &profile_uid,
                &lib.uid,
                &touched_tracks,
                &touched_albums,
                &[],
            ) {
                eprintln!("[merge] targeted merge failed, falling back to full update: {e}");
                library_manager::incremental_update(&profile_uid, &[lib.uid.clone()])?;
            }
        }

        repoint_tracks_everywhere(&profile_uid, &remaps);
        Ok(AutoMergeResult { merged })
    })
    .await?;

    app.emit("library:updated", ()).ok();
    Ok(result)
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
        Err(e) => return Err(e),
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
