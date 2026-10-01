use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
    pub uid: String,
    pub name: String,
    pub avatar_blob: Option<Vec<u8>>,
    #[serde(default)]
    pub password_hash: Option<String>,
    #[serde(default)]
    pub recovery_key_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProfileRegistry {
    pub active: String,
    pub profiles: Vec<Profile>,
}

pub fn get_profiles_dir() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("imago");
    fs::create_dir_all(&path).ok();
    path
}

pub fn get_profiles_json_path() -> PathBuf {
    get_profiles_dir().join("profiles.json")
}

pub fn get_profile_dir(uid: &str) -> PathBuf {
    let mut path = get_profiles_dir();
    path.push(uid);
    fs::create_dir_all(&path).ok();
    path
}

pub fn get_settings_db_path(uid: &str) -> PathBuf {
    get_profile_dir(uid).join("settings.db")
}

// Legacy — still used by existing commands that haven't been migrated yet.
// Points to lib.db in the profile root. Will be removed once migration is complete.
pub fn get_lib_db_path(uid: &str) -> PathBuf {
    get_profile_dir(uid).join("lib.db")
}

pub fn get_analytics_db_path(uid: &str) -> PathBuf {
    get_profile_dir(uid).join("analytics.db")
}

pub fn get_playlists_db_path(uid: &str) -> PathBuf {
    get_profile_dir(uid).join("playlists.db")
}

pub fn get_merged_db_path(uid: &str) -> PathBuf {
    get_profile_dir(uid).join("merged.db")
}

// Returns the libraries/ subdirectory for a profile, creating it if needed.
pub fn get_libraries_dir(uid: &str) -> PathBuf {
    let mut path = get_profile_dir(uid);
    path.push("libraries");
    fs::create_dir_all(&path).ok();
    path
}

// Returns the path to the default local library db.
pub fn get_local_library_db_path(uid: &str) -> PathBuf {
    get_libraries_dir(uid).join("local.db")
}

// Returns the path to a specific library db by its lib_uid.
pub fn get_library_db_path(profile_uid: &str, lib_uid: &str) -> PathBuf {
    get_libraries_dir(profile_uid).join(format!("{}.db", lib_uid))
}

pub fn read_registry() -> ProfileRegistry {
    let path = get_profiles_json_path();
    if !path.exists() {
        return ProfileRegistry {
            active: String::new(),
            profiles: vec![],
        };
    }
    let raw = fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&raw).unwrap_or(ProfileRegistry {
        active: String::new(),
        profiles: vec![],
    })
}

pub fn write_registry(registry: &ProfileRegistry) {
    let path = get_profiles_json_path();
    let raw = serde_json::to_string_pretty(registry).unwrap_or_default();
    fs::write(path, raw).ok();
}

pub fn create_profile(name: &str, avatar_blob: Option<Vec<u8>>) -> Profile {
    let uid = uuid::Uuid::new_v4().to_string();
    get_profile_dir(&uid);
    get_libraries_dir(&uid);
    Profile {
        uid,
        name: name.to_string(),
        avatar_blob,
        password_hash: None,
        recovery_key_hash: None,
    }
}

pub fn get_active_profile_uid() -> Option<String> {
    let registry = read_registry();
    if registry.active.is_empty() {
        None
    } else {
        Some(registry.active)
    }
}
