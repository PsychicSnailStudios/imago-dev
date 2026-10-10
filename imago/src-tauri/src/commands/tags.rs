use crate::db::tag_manager::{self, Tag, TagGroup, TagKind};
use crate::library_manager;
use crate::open_local_library_conn;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_all_tags_cmd(state: State<'_, AppState>) -> Result<Vec<Tag>, String> {
    let uid = state.get_uid();
    library_manager::run_blocking(move || {
        if let Err(e) = library_manager::sync_tag_dictionary(&uid) {
            eprintln!("[tags] dictionary sync failed: {e}");
        }
        let conn = open_local_library_conn(&uid);
        tag_manager::get_all_tags(&conn).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn add_tag_cmd(
    state: State<'_, AppState>,
    name: String,
    color: Option<String>,
) -> Result<String, String> {
    let uid = state.get_uid();
    let conn = open_local_library_conn(&uid);
    tag_manager::add_tag(&conn, &name, TagKind::Tag, color.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_genre_cmd(
    state: State<'_, AppState>,
    name: String,
    color: Option<String>,
) -> Result<String, String> {
    let uid = state.get_uid();
    let conn = open_local_library_conn(&uid);
    tag_manager::add_tag(&conn, &name, TagKind::Genre, color.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_tag_cmd(
    state: State<'_, AppState>,
    uid: String,
    new_name: String,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || {
        let conn = open_local_library_conn(&profile_uid);
        let tag = tag_manager::get_tag_by_uid(&conn, &uid).map_err(|e| e.to_string())?;
        tag_manager::rename_tag(&conn, &uid, &new_name).map_err(|e| e.to_string())?;
        if let Some(tag) = tag {
            library_manager::propagate_tag_change(
                &profile_uid,
                tag.kind == TagKind::Genre,
                &tag.name,
                Some(&new_name),
            )?;
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn delete_tag_cmd(state: State<'_, AppState>, uid: String) -> Result<(), String> {
    let profile_uid = state.get_uid();
    library_manager::run_blocking(move || {
        let conn = open_local_library_conn(&profile_uid);
        let tag = tag_manager::get_tag_by_uid(&conn, &uid).map_err(|e| e.to_string())?;
        tag_manager::delete_tag(&conn, &uid).map_err(|e| e.to_string())?;
        if let Some(tag) = tag {
            library_manager::propagate_tag_change(
                &profile_uid,
                tag.kind == TagKind::Genre,
                &tag.name,
                None,
            )?;
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn update_tag_color_cmd(
    state: State<'_, AppState>,
    uid: String,
    color: Option<String>,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let conn = open_local_library_conn(&profile_uid);
    tag_manager::update_tag_color(&conn, &uid, color.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_tag_groups_cmd(state: State<'_, AppState>) -> Result<Vec<TagGroup>, String> {
    let uid = state.get_uid();
    let conn = open_local_library_conn(&uid);
    tag_manager::get_all_tag_groups(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_tag_group_cmd(
    state: State<'_, AppState>,
    name: String,
    kind: String,
    color: Option<String>,
    member_uids: Vec<String>,
) -> Result<TagGroup, String> {
    let profile_uid = state.get_uid();
    let conn = open_local_library_conn(&profile_uid);
    tag_manager::create_tag_group(
        &conn,
        &name,
        TagKind::from_str(&kind),
        color.as_deref(),
        &member_uids,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_tag_group_cmd(
    state: State<'_, AppState>,
    uid: String,
    name: Option<String>,
    color: Option<Option<String>>,
    member_uids: Option<Vec<String>>,
) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let conn = open_local_library_conn(&profile_uid);
    tag_manager::update_tag_group(
        &conn,
        &uid,
        name.as_deref(),
        color.as_ref().map(|c| c.as_deref()),
        member_uids.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_tag_group_cmd(state: State<'_, AppState>, uid: String) -> Result<(), String> {
    let profile_uid = state.get_uid();
    let conn = open_local_library_conn(&profile_uid);
    tag_manager::delete_tag_group(&conn, &uid).map_err(|e| e.to_string())
}