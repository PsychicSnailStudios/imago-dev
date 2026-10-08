use crate::db::Setting;
use crate::open_settings_conn;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Vec<Setting>, String> {
    let uid = state.get_uid();
    let conn = open_settings_conn(&uid);
    crate::db::get_all_settings(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_setting(state: State<AppState>, key: String, value: String) -> Result<(), String> {
    let uid = state.get_uid();
    let conn = open_settings_conn(&uid);
    crate::db::set_setting(&conn, &key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn preview_filename_pattern_cmd(
    filename: String,
    pattern: String,
) -> crate::scanner::FilenamePreview {
    crate::scanner::preview_filename_pattern(&filename, pattern.trim())
}
