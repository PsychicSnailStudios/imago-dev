use crate::db::{
    self, create_artist, delete_artist_by_uid, get_all_artists, get_artist_by_uid,
    update_artist_by_uid, Artist, ArtistUpdate,
};
use crate::library_manager;
use crate::state::AppState;
use crate::{open_lib_conn, open_local_library_conn, open_merged_conn, open_settings_conn};
use tauri::State;

#[tauri::command]
pub async fn get_artists(state: State<'_, AppState>) -> Result<Vec<Artist>, String> {
    let uid = state.get_uid();
    library_manager::run_blocking(move || {
        let conn = open_merged_conn(&uid);
        get_all_artists(&conn).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub fn get_artist(state: State<AppState>, uid: String) -> Result<Option<Artist>, String> {
    let profile_uid = state.get_uid();
    let conn = open_merged_conn(&profile_uid);
    get_artist_by_uid(&conn, &uid).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_artist_uid_by_name(
    state: State<AppState>,
    name: String,
) -> Result<Option<String>, String> {
    let profile_uid = state.get_uid();
    let conn = open_merged_conn(&profile_uid);
    db::get_artist_uid_by_name(&conn, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_artist_profile_art(
    state: State<AppState>,
    uid: String,
) -> Result<Option<Vec<u8>>, String> {
    let profile_uid = state.get_uid();
    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "artists") {
        Ok((conn, _)) => db::get_artist_profile_art_by_uid(&conn, &uid).map_err(|e| e.to_string()),
        Err(_) => {
            let conn = open_merged_conn(&profile_uid);
            db::get_artist_profile_art_by_uid(&conn, &uid).map_err(|e| e.to_string())
        }
    }
}

#[tauri::command]
pub fn get_artist_banner_art(
    state: State<AppState>,
    uid: String,
) -> Result<Option<Vec<u8>>, String> {
    let profile_uid = state.get_uid();
    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "artists") {
        Ok((conn, _)) => db::get_artist_banner_art_by_uid(&conn, &uid).map_err(|e| e.to_string()),
        Err(_) => {
            let conn = open_merged_conn(&profile_uid);
            db::get_artist_banner_art_by_uid(&conn, &uid).map_err(|e| e.to_string())
        }
    }
}

#[tauri::command]
pub async fn create_artist_entry(state: State<'_, AppState>, artist: Artist) -> Result<(), String> {
    let uid = state.get_uid();
    library_manager::run_blocking(move || {
        let conn = open_local_library_conn(&uid);
        create_artist(&conn, &artist).map_err(|e| e.to_string())?;
        let settings_conn = open_settings_conn(&uid);
        if let Some(lib) = crate::db::library_registry::get_default_library(&settings_conn)
            .map_err(|e| e.to_string())?
        {
            library_manager::sync_artist(&uid, &lib.uid, &artist.uid)?;
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn update_artist_entry(
    state: State<'_, AppState>,
    uid: String,
    update: ArtistUpdate,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || {
        match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "artists") {
            Ok((source_conn, lib)) => {
                update_artist_by_uid(&source_conn, &uid, &update).map_err(|e| e.to_string())?;
                if let Some(ref tags_json) = update.tags {
                    if let Ok(names) = serde_json::from_str::<Vec<String>>(tags_json) {
                        for name in names {
                            crate::db::tag_manager::ensure_tag(
                                &source_conn,
                                &name,
                                crate::db::tag_manager::TagKind::Tag,
                            );
                        }
                    }
                }
                if let Some(ref genres_json) = update.genres {
                    if let Ok(names) = serde_json::from_str::<Vec<String>>(genres_json) {
                        for name in names {
                            crate::db::tag_manager::ensure_tag(
                                &source_conn,
                                &name,
                                crate::db::tag_manager::TagKind::Genre,
                            );
                        }
                    }
                }
                library_manager::sync_artist(&profile_uid, &lib.uid, &uid)?;
            }
            Err(_) => {
                let conn = open_lib_conn(&profile_uid);
                update_artist_by_uid(&conn, &uid, &update).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn delete_artist_entry(state: State<'_, AppState>, uid: String) -> Result<(), String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || {
        match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "artists") {
            Ok((source_conn, lib)) => {
                delete_artist_by_uid(&source_conn, &uid).map_err(|e| e.to_string())?;
                library_manager::sync_artist(&profile_uid, &lib.uid, &uid)?;
            }
            Err(_) => {
                let conn = open_lib_conn(&profile_uid);
                delete_artist_by_uid(&conn, &uid).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn rename_artist_in_library_cmd(
    state: State<'_, AppState>,
    old_name: String,
    new_name: String,
) -> Result<usize, String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || {
        library_manager::rename_artist_everywhere(&profile_uid, &old_name, &new_name)
    })
    .await
}

#[tauri::command]
pub fn get_artist_albums(
    state: State<AppState>,
    artist_uid: String,
    aka_uids: Vec<String>,
) -> Result<Vec<crate::db::album_manager::Album>, String> {
    let profile_uid = state.get_uid();
    let conn = open_merged_conn(&profile_uid);
    crate::db::search_manager::get_artist_albums(&conn, &artist_uid, &aka_uids)
        .map_err(|e| e.to_string())
}
