use crate::db::{
    self, create_album, delete_album_by_uid, get_album_by_uid, get_all_albums, update_album_by_uid,
    Album, AlbumUpdate,
};
use crate::library_manager;
use crate::state::AppState;
use crate::{open_lib_conn, open_local_library_conn, open_merged_conn, open_settings_conn};
use tauri::State;

#[tauri::command]
pub async fn get_albums(state: State<'_, AppState>) -> Result<Vec<Album>, String> {
    let uid = state.get_uid();
    library_manager::run_blocking(move || {
        let conn = open_merged_conn(&uid);
        get_all_albums(&conn).map_err(|e| e.to_string())
    })
    .await
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
pub async fn create_album_entry(state: State<'_, AppState>, album: Album) -> Result<(), String> {
    let uid = state.get_uid();
    library_manager::run_blocking(move || {
        let conn = open_local_library_conn(&uid);
        create_album(&conn, &album).map_err(|e| e.to_string())?;
        let settings_conn = open_settings_conn(&uid);
        if let Some(lib) = crate::db::library_registry::get_default_library(&settings_conn)
            .map_err(|e| e.to_string())?
        {
            library_manager::sync_album(&uid, &lib.uid, &album.uid)?;
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn update_album_entry(
    state: State<'_, AppState>,
    uid: String,
    update: AlbumUpdate,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || {
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
                library_manager::sync_album(&profile_uid, &lib.uid, &uid)?;
            }
            Err(_) => {
                let conn = open_lib_conn(&profile_uid);
                update_album_by_uid(&conn, &uid, &update).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn delete_album_entry(state: State<'_, AppState>, uid: String) -> Result<(), String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || {
        match library_manager::open_source_conn_for_entity(&profile_uid, &uid, "albums") {
            Ok((source_conn, lib)) => {
                delete_album_by_uid(&source_conn, &uid).map_err(|e| e.to_string())?;
                library_manager::sync_album(&profile_uid, &lib.uid, &uid)?;
            }
            Err(_) => {
                let conn = open_lib_conn(&profile_uid);
                delete_album_by_uid(&conn, &uid).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn rename_album_in_tracks_cmd(
    state: State<'_, AppState>,
    old_name: String,
    new_name: String,
    album_artist: String,
    album_uid: Option<String>,
) -> Result<usize, String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || {
        library_manager::rename_album_everywhere(
            &profile_uid,
            &old_name,
            &new_name,
            &album_artist,
            album_uid.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub async fn set_album_artist_for_album_cmd(
    state: State<'_, AppState>,
    album_uid: String,
    new_name: String,
) -> Result<usize, String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || {
        library_manager::set_album_artist_for_album(&profile_uid, &album_uid, &new_name)
    })
    .await
}

#[tauri::command]
pub async fn rename_album_artist_cmd(
    state: State<'_, AppState>,
    old_name: String,
    new_name: String,
) -> Result<usize, String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || {
        library_manager::rename_album_artist_everywhere(&profile_uid, &old_name, &new_name)
    })
    .await
}
