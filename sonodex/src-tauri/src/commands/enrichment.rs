use crate::db::{self, AlbumUpdate, ArtistUpdate, MetadataUpdate};
use crate::enrichment::{self, EnrichSettings};
use crate::library_manager;
use crate::state::AppState;
use crate::{open_lib_conn, open_merged_conn, open_settings_conn};
use tauri::{AppHandle, Emitter, State};

fn load_enrich_settings(settings_conn: &rusqlite::Connection) -> EnrichSettings {
    let g = |key: &str, default: &str| -> String {
        crate::db::get_setting(settings_conn, key)
            .ok()
            .flatten()
            .unwrap_or_else(|| default.to_string())
    };
    EnrichSettings {
        primary_api: g("enrich_primary_api", "musicbrainz"),
        lastfm_key: g("api_lastfm_key", ""),
        discogs_key: g("api_discogs_key", ""),
        audiodb_key: g("api_audiodb_key", ""),
        priority_title: g("enrich_priority_title", "local"),
        priority_artists: g("enrich_priority_artists", "local"),
        priority_album_artist: g("enrich_priority_album_artist", "local"),
        priority_album: g("enrich_priority_album", "local"),
        priority_year: g("enrich_priority_year", "local"),
        priority_genres: g("enrich_priority_genres", "local"),
        priority_bpm: g("enrich_priority_bpm", "local"),
        priority_key: g("enrich_priority_key", "local"),
        priority_artwork: g("enrich_priority_artwork", "local"),
    }
}

fn ensure_genre_tags_str(conn: &rusqlite::Connection, genres_json: &Option<String>) {
    if let Some(ref gj) = genres_json {
        if let Ok(names) = serde_json::from_str::<Vec<String>>(gj) {
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
pub async fn enrich_track(
    app: AppHandle,
    state: State<'_, AppState>,
    uid: String,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let settings_conn = open_settings_conn(&profile_uid);
    let settings = load_enrich_settings(&settings_conn);

    let (track_input, numeric_id) = {
        let conn = open_merged_conn(&profile_uid);
        let track = db::get_track_by_uid(&conn, &uid)
            .map_err(|e| e.to_string())?
            .ok_or("Track not found")?;
        let id = track.id.ok_or("Track has no id")?;
        let artwork = db::get_track_artwork(&conn, id).ok().flatten();
        let input = enrichment::TrackInput {
            id,
            title: track.title,
            artists: track.artists.clone(),
            album_artist: track.album_artist.clone(),
            albums: track.albums.clone(),
            year: track.year,
            genres: track.genres.clone(),
            bpm: track.bpm,
            key: track.key,
            existing_artwork: artwork,
        };
        (input, id)
    };

    let client = enrichment::make_client()?;
    let result =
        enrichment::enrich_track_async(&client, &track_input, &settings, Some(&profile_uid))
            .await
            .map_err(|e| e.to_string())?;

    let update = MetadataUpdate {
        title: result.title,
        artists: result.artists,
        album_artist: result.album_artist,
        albums: result.albums,
        year: result.year,
        genres: result.genres,
        bpm: result.bpm,
        key: result.key,
        artwork_blob: result.artwork,
        ..Default::default()
    };

    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "tracks") {
        Ok((source_conn, lib)) => {
            ensure_genre_tags_str(&source_conn, &update.genres);
            db::update_track_metadata_by_uid(&source_conn, &uid, &update)
                .map_err(|e| e.to_string())?;
            library_manager::incremental_update(&profile_uid, &[lib.uid])
                .map_err(|e| e.to_string())?;
        }
        Err(_) => {
            let conn = open_lib_conn(&profile_uid);
            ensure_genre_tags_str(&conn, &update.genres);
            db::update_track_metadata(&conn, numeric_id, &update).map_err(|e| e.to_string())?;
        }
    }

    app.emit("library:updated", ()).ok();
    Ok(())
}

#[tauri::command]
pub async fn enrich_all(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let settings_conn = open_settings_conn(&profile_uid);
    let settings = load_enrich_settings(&settings_conn);

    let track_inputs: Vec<enrichment::TrackInput> = {
        let conn = open_merged_conn(&profile_uid);
        db::get_all_tracks(&conn)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|t| {
                let id = t.id?;
                let artwork = db::get_track_artwork(&conn, id).ok().flatten();
                Some(enrichment::TrackInput {
                    id,
                    title: t.title,
                    artists: t.artists.clone(),
                    album_artist: t.album_artist.clone(),
                    albums: t.albums.clone(),
                    year: t.year,
                    genres: t.genres.clone(),
                    bpm: t.bpm,
                    key: t.key,
                    existing_artwork: artwork,
                })
            })
            .collect()
    };

    let total = track_inputs.len();
    let mut done = 0usize;
    let mut errors = 0usize;

    app.emit(
        "enrich:progress",
        serde_json::json!({ "done": 0, "total": total, "errors": 0 }),
    )
    .ok();

    let client = enrichment::make_client()?;

    for track_input in &track_inputs {
        match enrichment::enrich_track_async(&client, track_input, &settings, Some(&profile_uid))
            .await
        {
            Ok(result) => {
                let update = MetadataUpdate {
                    title: result.title,
                    artists: result.artists,
                    album_artist: result.album_artist,
                    albums: result.albums,
                    year: result.year,
                    genres: result.genres,
                    bpm: result.bpm,
                    key: result.key,
                    artwork_blob: result.artwork,
                    ..Default::default()
                };
                let conn = open_lib_conn(&profile_uid);
                ensure_genre_tags_str(&conn, &update.genres);
                db::update_track_metadata(&conn, track_input.id, &update).ok();
            }
            Err(_) => errors += 1,
        }
        done += 1;
        if done % 5 == 0 || done == total {
            app.emit(
                "enrich:progress",
                serde_json::json!({ "done": done, "total": total, "errors": errors }),
            )
            .ok();
        }
    }

    let settings_conn2 = open_settings_conn(&profile_uid);
    if let Ok(Some(lib)) = crate::db::library_registry::get_default_library(&settings_conn2) {
        let _ = library_manager::incremental_update(&profile_uid, &[lib.uid]);
    }

    app.emit(
        "enrich:done",
        serde_json::json!({ "total": total, "errors": errors }),
    )
    .ok();
    app.emit("library:updated", ()).ok();
    Ok(())
}

#[tauri::command]
pub async fn enrich_album(
    app: AppHandle,
    state: State<'_, AppState>,
    uid: String,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let settings_conn = open_settings_conn(&profile_uid);
    let settings = load_enrich_settings(&settings_conn);

    let (album, track_uids) = {
        let conn = open_merged_conn(&profile_uid);
        let album = db::get_album_by_uid(&conn, &uid)
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
        (album, track_uids)
    };

    let artist = db::resolved_artist_name(&album.album_artist, &album.artists)
        .ok_or("Album has no artist")?;

    let client = enrichment::make_client()?;
    let result = enrichment::enrich_album_async(
        &client,
        &album.title,
        &artist,
        &settings,
        Some(&profile_uid),
    )
    .await;

    let update = AlbumUpdate {
        title: None,
        format: result.format,
        rating: None,
        artists: None,
        album_artist: None,
        release_date: result.release_date,
        tags: None,
        genres: result.genres,
        tracks: None,
        credits: result.description,
        label: result.label,
        artwork_blob: result.artwork.clone(),
        artwork_path: None,
        emulate_type: None,
    };

    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "albums") {
        Ok((source_conn, lib)) => {
            ensure_genre_tags_str(&source_conn, &update.genres);
            db::update_album_by_uid(&source_conn, &uid, &update).map_err(|e| e.to_string())?;

            if let Some(ref art) = update.artwork_blob {
                for track_uid in &track_uids {
                    let has_art = db::get_track_artwork_by_uid(&source_conn, track_uid)
                        .ok()
                        .flatten()
                        .map(|b| !b.is_empty())
                        .unwrap_or(false);
                    if !has_art {
                        let art_update = MetadataUpdate {
                            artwork_blob: Some(art.clone()),
                            ..Default::default()
                        };
                        db::update_track_metadata_by_uid(&source_conn, track_uid, &art_update).ok();
                    }
                }
            }
            library_manager::incremental_update(&profile_uid, &[lib.uid])
                .map_err(|e| e.to_string())?;
        }
        Err(_) => {
            let conn = open_lib_conn(&profile_uid);
            ensure_genre_tags_str(&conn, &update.genres);
            db::update_album_by_uid(&conn, &uid, &update).map_err(|e| e.to_string())?;
        }
    }

    app.emit("library:updated", ()).ok();
    Ok(())
}

#[tauri::command]
pub async fn enrich_all_albums(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let settings_conn = open_settings_conn(&profile_uid);
    let settings = load_enrich_settings(&settings_conn);

    let albums = {
        let conn = open_merged_conn(&profile_uid);
        db::get_all_albums(&conn).map_err(|e| e.to_string())?
    };

    let total = albums.len();
    let mut done = 0usize;
    let mut errors = 0usize;

    app.emit(
        "enrich:progress",
        serde_json::json!({ "done": 0, "total": total, "errors": 0 }),
    )
    .ok();

    let client = enrichment::make_client()?;

    for album in &albums {
        let artist = db::resolved_artist_name(&album.album_artist, &album.artists);

        if let Some(artist) = artist {
            let result = enrichment::enrich_album_async(
                &client,
                &album.title,
                &artist,
                &settings,
                Some(&profile_uid),
            )
            .await;
            let update = AlbumUpdate {
                title: None,
                format: result.format,
                rating: None,
                artists: None,
                album_artist: None,
                release_date: result.release_date,
                tags: None,
                genres: result.genres,
                tracks: None,
                credits: result.description,
                label: result.label,
                artwork_blob: result.artwork,
                artwork_path: None,
                emulate_type: None,
            };
            let conn = open_lib_conn(&profile_uid);
            ensure_genre_tags_str(&conn, &update.genres);
            if db::update_album_by_uid(&conn, &album.uid, &update).is_err() {
                errors += 1;
            }
        } else {
            errors += 1;
        }

        done += 1;
        if done % 5 == 0 || done == total {
            app.emit(
                "enrich:progress",
                serde_json::json!({ "done": done, "total": total, "errors": errors }),
            )
            .ok();
        }
    }

    let settings_conn2 = open_settings_conn(&profile_uid);
    if let Ok(Some(lib)) = crate::db::library_registry::get_default_library(&settings_conn2) {
        let _ = library_manager::incremental_update(&profile_uid, &[lib.uid]);
    }

    app.emit(
        "enrich:done",
        serde_json::json!({ "total": total, "errors": errors }),
    )
    .ok();
    app.emit("library:updated", ()).ok();
    Ok(())
}

#[tauri::command]
pub async fn enrich_artist(
    app: AppHandle,
    state: State<'_, AppState>,
    uid: String,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let settings_conn = open_settings_conn(&profile_uid);
    let settings = load_enrich_settings(&settings_conn);

    let artist = {
        let conn = open_merged_conn(&profile_uid);
        db::get_artist_by_uid(&conn, &uid)
            .map_err(|e| e.to_string())?
            .ok_or("Artist not found")?
    };

    let client = enrichment::make_client()?;
    let result =
        enrichment::enrich_artist_async(&client, &artist.name, &settings, Some(&profile_uid)).await;

    let update = ArtistUpdate {
        name: None,
        aka: None,
        about: result.about,
        tags: None,
        genres: result.genres,
        websites: result.websites,
        members: None,
        profile_art_blob: result.profile_art,
        profile_art_path: None,
        banner_art_blob: result.banner_art,
        banner_art_path: None,
    };

    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "artists") {
        Ok((source_conn, lib)) => {
            ensure_genre_tags_str(&source_conn, &update.genres);
            db::update_artist_by_uid(&source_conn, &uid, &update).map_err(|e| e.to_string())?;
            library_manager::incremental_update(&profile_uid, &[lib.uid])
                .map_err(|e| e.to_string())?;
        }
        Err(_) => {
            let conn = open_lib_conn(&profile_uid);
            ensure_genre_tags_str(&conn, &update.genres);
            db::update_artist_by_uid(&conn, &uid, &update).map_err(|e| e.to_string())?;
        }
    }

    app.emit("library:updated", ()).ok();
    Ok(())
}

#[tauri::command]
pub async fn enrich_all_artists(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let settings_conn = open_settings_conn(&profile_uid);
    let settings = load_enrich_settings(&settings_conn);

    let artists = {
        let conn = open_merged_conn(&profile_uid);
        db::get_all_artists(&conn).map_err(|e| e.to_string())?
    };

    let total = artists.len();
    let mut done = 0usize;
    let mut errors = 0usize;

    app.emit(
        "enrich:progress",
        serde_json::json!({ "done": 0, "total": total, "errors": 0 }),
    )
    .ok();

    let client = enrichment::make_client()?;

    for artist in &artists {
        let result =
            enrichment::enrich_artist_async(&client, &artist.name, &settings, Some(&profile_uid))
                .await;
        let update = ArtistUpdate {
            name: None,
            aka: None,
            about: result.about,
            tags: None,
            genres: result.genres,
            websites: result.websites,
            members: None,
            profile_art_blob: result.profile_art,
            profile_art_path: None,
            banner_art_blob: result.banner_art,
            banner_art_path: None,
        };
        let conn = open_lib_conn(&profile_uid);
        ensure_genre_tags_str(&conn, &update.genres);
        if db::update_artist_by_uid(&conn, &artist.uid, &update).is_err() {
            errors += 1;
        }

        done += 1;
        if done % 5 == 0 || done == total {
            app.emit(
                "enrich:progress",
                serde_json::json!({ "done": done, "total": total, "errors": errors }),
            )
            .ok();
        }
    }

    let settings_conn2 = open_settings_conn(&profile_uid);
    if let Ok(Some(lib)) = crate::db::library_registry::get_default_library(&settings_conn2) {
        let _ = library_manager::incremental_update(&profile_uid, &[lib.uid]);
    }

    app.emit(
        "enrich:done",
        serde_json::json!({ "total": total, "errors": errors }),
    )
    .ok();
    app.emit("library:updated", ()).ok();
    Ok(())
}

// ─── Spotify enrichment ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn spotify_enrich_track_cmd(
    uid: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let conn = open_merged_conn(&profile_uid);
    let track = db::track_manager::get_track_by_uid(&conn, &uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Track not found: {}", uid))?;

    let title = track.title.as_deref().unwrap_or("");
    let artist = db::first_artist_name(&track.artists).unwrap_or_default();

    let client = enrichment::make_client()?;
    let Some(meta) = enrichment::spotify::enrich_track(&client, &profile_uid, title, &artist).await
    else {
        return Ok(());
    };

    let update = MetadataUpdate {
        title: meta.title,
        artists: meta
            .artists
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default()),
        year: meta.year,
        genres: meta.genres,
        ..Default::default()
    };

    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "tracks") {
        Ok((source_conn, lib)) => {
            ensure_genre_tags_str(&source_conn, &update.genres);
            db::track_manager::update_track_metadata_by_uid(&source_conn, &uid, &update)
                .map_err(|e| e.to_string())?;
            library_manager::incremental_update(&profile_uid, &[lib.uid])
                .map_err(|e| e.to_string())?;
        }
        Err(_) => {
            let fallback = open_lib_conn(&profile_uid);
            ensure_genre_tags_str(&fallback, &update.genres);
            db::track_manager::update_track_metadata_by_uid(&fallback, &uid, &update)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn spotify_enrich_album_cmd(
    uid: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let conn = open_merged_conn(&profile_uid);
    let album = db::album_manager::get_album_by_uid(&conn, &uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Album not found: {}", uid))?;

    let artist = db::resolved_artist_name(&album.album_artist, &album.artists).unwrap_or_default();

    let client = enrichment::make_client()?;
    let Some(meta) =
        enrichment::spotify::enrich_album(&client, &profile_uid, &album.title, &artist).await
    else {
        return Ok(());
    };

    let update = AlbumUpdate {
        title: None,
        format: None,
        rating: None,
        artists: None,
        album_artist: None,
        release_date: meta.release_date,
        tags: None,
        genres: meta
            .genres
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default()),
        tracks: None,
        credits: None,
        label: meta.label,
        artwork_blob: None,
        artwork_path: None,
        emulate_type: None,
    };

    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "albums") {
        Ok((source_conn, lib)) => {
            ensure_genre_tags_str(&source_conn, &update.genres);
            db::album_manager::update_album_by_uid(&source_conn, &uid, &update)
                .map_err(|e| e.to_string())?;
            library_manager::incremental_update(&profile_uid, &[lib.uid])
                .map_err(|e| e.to_string())?;
        }
        Err(_) => {
            let fallback = open_lib_conn(&profile_uid);
            ensure_genre_tags_str(&fallback, &update.genres);
            db::album_manager::update_album_by_uid(&fallback, &uid, &update)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn spotify_enrich_artist_cmd(
    uid: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let conn = open_merged_conn(&profile_uid);
    let artist = db::artist_manager::get_artist_by_uid(&conn, &uid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Artist not found: {}", uid))?;

    let client = enrichment::make_client()?;
    let Some(meta) = enrichment::spotify::enrich_artist(&client, &profile_uid, &artist.name).await
    else {
        return Ok(());
    };

    let update = ArtistUpdate {
        name: None,
        aka: None,
        about: None,
        tags: None,
        genres: meta
            .genres
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default()),
        websites: None,
        members: None,
        profile_art_blob: None,
        profile_art_path: None,
        banner_art_blob: None,
        banner_art_path: None,
    };

    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "artists") {
        Ok((source_conn, lib)) => {
            ensure_genre_tags_str(&source_conn, &update.genres);
            db::artist_manager::update_artist_by_uid(&source_conn, &uid, &update)
                .map_err(|e| e.to_string())?;
            library_manager::incremental_update(&profile_uid, &[lib.uid])
                .map_err(|e| e.to_string())?;
        }
        Err(_) => {
            let fallback = open_lib_conn(&profile_uid);
            ensure_genre_tags_str(&fallback, &update.genres);
            db::artist_manager::update_artist_by_uid(&fallback, &uid, &update)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
