use crate::db::blocklist_manager::{BlocklistEntry, LibraryDeletePreference};
use crate::db::library_registry::{Library, LibraryUpdate};
use crate::library_manager::{self, MergeResult};
use crate::open_settings_conn;
use crate::profiles::get_merged_db_path;
use crate::state::AppState;
use rusqlite::Connection;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn get_libraries(state: State<AppState>) -> Result<Vec<Library>, String> {
    let uid = state.get_uid();
    let conn = open_settings_conn(&uid);
    crate::db::library_registry::get_all_libraries(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_library_cmd(state: State<AppState>, name: String) -> Result<Library, String> {
    let uid = state.get_uid();
    library_manager::create_local_library(&uid, &name)
}

#[tauri::command]
pub async fn import_library_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    sync_url: String,
    write_token: Option<String>,
) -> Result<Library, String> {
    let uid = state.get_uid();
    let library =
        library_manager::import_library(&uid, &name, &sync_url, write_token.as_deref()).await?;
    crate::commands::scrobbles::schedule_scrobble_relink(app, uid);
    Ok(library)
}

#[tauri::command]
pub fn export_library_cmd(
    state: State<AppState>,
    lib_uid: String,
    dest_path: String,
) -> Result<(), String> {
    let uid = state.get_uid();
    library_manager::export_library(&uid, &lib_uid, &dest_path)
}

#[tauri::command]
pub fn update_library_cmd(
    state: State<AppState>,
    lib_uid: String,
    mut update: LibraryUpdate,
) -> Result<(), String> {
    if let Some(u) = update.sync_url.as_mut() {
        *u = library_manager::normalize_sync_location(u);
    }
    if let Some(u) = update.sync_meta_url.as_mut() {
        *u = library_manager::normalize_sync_location(u);
    }
    let uid = state.get_uid();
    let conn = open_settings_conn(&uid);
    crate::db::library_registry::update_library(&conn, &lib_uid, &update)
        .map_err(|e| e.to_string())?;
    crate::webdav::clear_credential_cache();
    Ok(())
}

#[tauri::command]
pub fn library_has_credentials_cmd(
    state: State<AppState>,
    lib_uid: String,
) -> Result<bool, String> {
    let uid = state.get_uid();
    let conn = open_settings_conn(&uid);
    let lib = crate::db::library_registry::get_library_by_uid(&conn, &lib_uid)
        .map_err(|e| e.to_string())?;
    Ok(lib
        .and_then(|l| l.write_token)
        .map(|t| !t.trim().is_empty())
        .unwrap_or(false))
}

#[tauri::command]
pub async fn delete_library_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    lib_uid: String,
    delete_file: bool,
) -> Result<(), String> {
    let uid = state.get_uid();
    library_manager::run_blocking(move || {
        library_manager::delete_local_library(&uid, &lib_uid, delete_file)?;
        app.emit("library:updated", ()).ok();
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn sync_library_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    lib_uid: String,
    force: Option<bool>,
) -> Result<bool, String> {
    let uid = state.get_uid();
    let pulled =
        library_manager::try_pull_library(&uid, &lib_uid, force.unwrap_or(false)).await?;
    if pulled {
        let rebuild_uid = uid.clone();
        library_manager::run_blocking(move || library_manager::full_rebuild(&rebuild_uid)).await?;
        app.emit("library:updated", ()).ok();
        crate::commands::scrobbles::schedule_scrobble_relink(app.clone(), uid.clone());
    }
    Ok(pulled)
}

#[tauri::command]
pub async fn push_library_cmd(state: State<'_, AppState>, lib_uid: String) -> Result<bool, String> {
    let uid = state.get_uid();
    let conn = open_settings_conn(&uid);
    let lib = crate::db::library_registry::get_library_by_uid(&conn, &lib_uid)
        .map_err(|e| e.to_string())?
        .ok_or("Library not found")?;
    library_manager::push_library_checked(&lib).await?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let update = crate::db::library_registry::LibraryUpdate {
        name: None,
        sync_url: None,
        sync_meta_url: None,
        write_token: None,
        has_write_permission: None,
        last_synced: Some(now),
        last_data_version: None,
        sort_order: None,
    };
    crate::db::library_registry::update_library(&conn, &lib_uid, &update).ok();
    Ok(true)
}

#[tauri::command]
pub async fn check_write_permission_cmd(
    state: State<'_, AppState>,
    lib_uid: String,
) -> Result<bool, String> {
    let uid = state.get_uid();
    library_manager::check_write_permission(&uid, &lib_uid).await
}

#[tauri::command]
pub async fn rebuild_merged_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<MergeResult, String> {
    let uid = state.get_uid();
    let relink_uid = uid.clone();
    let result = library_manager::run_blocking(move || library_manager::full_rebuild(&uid)).await?;
    crate::commands::scrobbles::schedule_scrobble_relink(app, relink_uid);
    Ok(result)
}

#[tauri::command]
pub fn get_blocklist(state: State<AppState>) -> Result<Vec<BlocklistEntry>, String> {
    let uid = state.get_uid();
    let conn = open_settings_conn(&uid);
    crate::db::blocklist_manager::get_all_blocklist_entries(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_to_blocklist_cmd(
    state: State<AppState>,
    uid: String,
    entity_type: String,
    cascade: bool,
    source_lib_uid: String,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let conn = open_settings_conn(&profile_uid);
    crate::db::blocklist_manager::add_to_blocklist(
        &conn,
        &uid,
        &entity_type,
        Some("hidden by user"),
        cascade,
        &source_lib_uid,
    )
    .map_err(|e| e.to_string())?;

    let merged_path = get_merged_db_path(&profile_uid);
    if merged_path.exists() {
        if let Ok(merged_conn) = Connection::open(&merged_path) {
            merged_conn
                .execute(
                    &format!("DELETE FROM {} WHERE uid = ?1", entity_type),
                    rusqlite::params![uid],
                )
                .ok();
        }
    }
    Ok(())
}

#[tauri::command]
pub fn remove_from_blocklist_cmd(state: State<AppState>, uid: String) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let conn = open_settings_conn(&profile_uid);
    crate::db::blocklist_manager::remove_from_blocklist(&conn, &uid).map_err(|e| e.to_string())?;
    library_manager::full_rebuild(&profile_uid)?;
    Ok(())
}

#[tauri::command]
pub fn get_delete_preference_cmd(
    state: State<AppState>,
    lib_uid: String,
) -> Result<Option<LibraryDeletePreference>, String> {
    let uid = state.get_uid();
    let conn = open_settings_conn(&uid);
    crate::db::blocklist_manager::get_delete_preference(&conn, &lib_uid).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_delete_preference_cmd(
    state: State<AppState>,
    lib_uid: String,
    cascade_delete: i64,
) -> Result<(), String> {
    let uid = state.get_uid();
    let conn = open_settings_conn(&uid);
    crate::db::blocklist_manager::set_delete_preference(&conn, &lib_uid, cascade_delete)
        .map_err(|e| e.to_string())
}
