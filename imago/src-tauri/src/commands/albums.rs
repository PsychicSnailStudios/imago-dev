use crate::db::{
    self, create_album, delete_album_by_uid, get_album_by_uid, get_all_albums, update_album_by_uid,
    Album, AlbumUpdate,
};
use crate::library_manager;
use crate::state::AppState;
use crate::{open_lib_conn, open_local_library_conn, open_merged_conn, open_settings_conn};
use tauri::State;

#[tauri::command]
pub fn get_albums(state: State<AppState>) -> Result<Vec<Album>, String> {
    let uid = state.get_uid();
    let conn = open_merged_conn(&uid);
    get_all_albums(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_album(state: State<AppState>, uid: String) -> Result<Option<Album>, String> {
    let profile_uid = state.get_uid();
    let conn = open_merged_conn(&profile_uid);
    get_album_by_uid(&conn, &uid).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_album_artwork(state: State<AppState>, uid: String) -> Result<Option<Vec<u8>>, String> {
    let profile_uid = state.get_uid();
    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "albums") {
        Ok((conn, _)) => db::get_album_artwork_by_uid(&conn, &uid).map_err(|e| e.to_string()),
        Err(_) => {
            let conn = open_merged_conn(&profile_uid);
            db::get_album_artwork_by_uid(&conn, &uid).map_err(|e| e.to_string())
        }
    }
}

#[tauri::command]
pub fn create_album_entry(state: State<AppState>, album: Album) -> Result<(), String> {
    let uid = state.get_uid();
    let conn = open_local_library_conn(&uid);
    create_album(&conn, &album).map_err(|e| e.to_string())?;
    let settings_conn = open_settings_conn(&uid);
    if let Some(lib) = crate::db::library_registry::get_default_library(&settings_conn)
        .map_err(|e| e.to_string())?
    {
        library_manager::incremental_update(&uid, &[lib.uid]).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn update_album_entry(
    state: State<AppState>,
    uid: String,
    update: AlbumUpdate,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "albums") {
        Ok((source_conn, lib)) => {
            update_album_by_uid(&source_conn, &uid, &update).map_err(|e| e.to_string())?;
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
            library_manager::incremental_update(&profile_uid, &[lib.uid])
                .map_err(|e| e.to_string())?;
        }
        Err(_) => {
            let conn = open_lib_conn(&profile_uid);
            update_album_by_uid(&conn, &uid, &update).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn delete_album_entry(state: State<AppState>, uid: String) -> Result<(), String> {
    let profile_uid = state.get_uid();
    match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "albums") {
        Ok((source_conn, lib)) => {
            delete_album_by_uid(&source_conn, &uid).map_err(|e| e.to_string())?;
            library_manager::incremental_update(&profile_uid, &[lib.uid])
                .map_err(|e| e.to_string())?;
        }
        Err(_) => {
            let conn = open_lib_conn(&profile_uid);
            delete_album_by_uid(&conn, &uid).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
