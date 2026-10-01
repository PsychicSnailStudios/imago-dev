use crate::db::{add_library_path, get_library_paths, init_lib_db, init_settings_db};
use crate::library_manager;
use crate::profiles::{
    create_profile as new_profile, get_lib_db_path, read_registry, write_registry, Profile,
};
use crate::state::AppState;
use crate::{open_analytics_conn, open_settings_conn};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rusqlite::Connection;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn needs_profile_setup() -> bool {
    read_registry().profiles.is_empty()
}

#[tauri::command]
pub fn get_profiles() -> Result<Vec<Profile>, String> {
    let mut profiles = read_registry().profiles;
    for p in &mut profiles {
        p.avatar_blob = None;
    }
    Ok(profiles)
}

#[tauri::command]
pub fn get_active_profile(state: State<AppState>) -> Result<Profile, String> {
    let uid = state.get_uid();
    read_registry()
        .profiles
        .into_iter()
        .find(|p| p.uid == uid)
        .map(|mut p| {
            p.avatar_blob = None;
            p
        })
        .ok_or("Active profile not found".to_string())
}

#[tauri::command]
pub fn get_profile_avatar(uid: String) -> Result<Option<Vec<u8>>, String> {
    Ok(read_registry()
        .profiles
        .into_iter()
        .find(|p| p.uid == uid)
        .and_then(|p| p.avatar_blob))
}

#[tauri::command]
pub fn create_profile_cmd(
    app: AppHandle,
    state: State<AppState>,
    name: String,
    avatar_blob: Option<Vec<u8>>,
    copy_paths_from: Option<String>,
) -> Result<Profile, String> {
    let profile = new_profile(&name, avatar_blob);

    let settings_conn = open_settings_conn(&profile.uid);
    init_settings_db(&settings_conn).map_err(|e| e.to_string())?;

    let lib_conn = Connection::open(get_lib_db_path(&profile.uid)).map_err(|e| e.to_string())?;
    init_lib_db(&lib_conn).map_err(|e| e.to_string())?;

    library_manager::ensure_default_library(&profile.uid)?;

    if let Some(source_uid) = copy_paths_from {
        let source_conn = open_settings_conn(&source_uid);
        let paths = get_library_paths(&source_conn).unwrap_or_default();
        for p in paths {
            add_library_path(&settings_conn, &p.path).ok();
        }
    }

    let mut registry = read_registry();
    let is_first = registry.profiles.is_empty();
    registry.profiles.push(profile.clone());
    if is_first {
        registry.active = profile.uid.clone();
        state.set_uid(profile.uid.clone());
    }
    write_registry(&registry);

    if is_first {
        app.emit("profile:ready", ()).ok();
    }

    Ok(profile)
}

#[tauri::command]
pub fn update_profile_cmd(
    uid: String,
    name: Option<String>,
    avatar_blob: Option<Vec<u8>>,
) -> Result<(), String> {
    let mut registry = read_registry();
    if let Some(profile) = registry.profiles.iter_mut().find(|p| p.uid == uid) {
        if let Some(n) = name {
            profile.name = n;
        }
        if avatar_blob.is_some() {
            profile.avatar_blob = avatar_blob;
        }
    }
    write_registry(&registry);
    Ok(())
}

#[tauri::command]
pub fn delete_profile_cmd(uid: String, state: State<AppState>) -> Result<(), String> {
    if state.get_uid() == uid {
        return Err("Cannot delete the active profile".to_string());
    }
    let mut registry = read_registry();
    registry.profiles.retain(|p| p.uid != uid);
    write_registry(&registry);
    let profile_dir = crate::profiles::get_profile_dir(&uid);
    std::fs::remove_dir_all(profile_dir).ok();
    Ok(())
}

#[tauri::command]
pub fn switch_profile(uid: String, state: State<AppState>) -> Result<(), String> {
    let registry = read_registry();
    if !registry.profiles.iter().any(|p| p.uid == uid) {
        return Err("Profile not found".to_string());
    }
    let mut registry = registry;
    registry.active = uid.clone();
    write_registry(&registry);
    state.set_uid(uid);
    Ok(())
}

#[tauri::command]
pub fn set_profile_password(uid: String, password: String) -> Result<String, String> {
    let argon2 = Argon2::default();
    let pw_salt = SaltString::generate(&mut OsRng);
    let pw_hash = argon2
        .hash_password(password.as_bytes(), &pw_salt)
        .map_err(|e| e.to_string())?
        .to_string();

    let recovery_key: String = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let segments: Vec<String> = (0..6)
            .map(|_| {
                (0..4)
                    .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
                    .collect::<String>()
                    .to_uppercase()
            })
            .collect();
        segments.join("-")
    };

    let rk_salt = SaltString::generate(&mut OsRng);
    let rk_hash = argon2
        .hash_password(recovery_key.as_bytes(), &rk_salt)
        .map_err(|e| e.to_string())?
        .to_string();

    let mut registry = read_registry();
    if let Some(profile) = registry.profiles.iter_mut().find(|p| p.uid == uid) {
        profile.password_hash = Some(pw_hash);
        profile.recovery_key_hash = Some(rk_hash);
    } else {
        return Err("Profile not found".to_string());
    }
    write_registry(&registry);
    Ok(recovery_key)
}

#[tauri::command]
pub fn remove_profile_password(uid: String) -> Result<(), String> {
    let mut registry = read_registry();
    if let Some(profile) = registry.profiles.iter_mut().find(|p| p.uid == uid) {
        profile.password_hash = None;
        profile.recovery_key_hash = None;
    } else {
        return Err("Profile not found".to_string());
    }
    write_registry(&registry);
    Ok(())
}

#[tauri::command]
pub fn verify_profile_password(uid: String, password: String) -> Result<bool, String> {
    let registry = read_registry();
    let profile = registry
        .profiles
        .iter()
        .find(|p| p.uid == uid)
        .ok_or("Profile not found")?;
    let hash_str = match &profile.password_hash {
        Some(h) => h,
        None => return Ok(true),
    };
    let parsed = PasswordHash::new(hash_str).map_err(|e| e.to_string())?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[tauri::command]
pub fn verify_recovery_key(uid: String, key: String) -> Result<bool, String> {
    let registry = read_registry();
    let profile = registry
        .profiles
        .iter()
        .find(|p| p.uid == uid)
        .ok_or("Profile not found")?;
    let hash_str = match &profile.recovery_key_hash {
        Some(h) => h,
        None => return Ok(false),
    };
    let parsed = PasswordHash::new(hash_str).map_err(|e| e.to_string())?;
    Ok(Argon2::default()
        .verify_password(key.as_bytes(), &parsed)
        .is_ok())
}

#[tauri::command]
pub fn profile_has_password(uid: String) -> Result<bool, String> {
    let registry = read_registry();
    let profile = registry
        .profiles
        .iter()
        .find(|p| p.uid == uid)
        .ok_or("Profile not found")?;
    Ok(profile.password_hash.is_some())
}
