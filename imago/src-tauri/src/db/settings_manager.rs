use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryPath {
    pub id: Option<i64>,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

pub fn add_library_path(conn: &Connection, path: &str) -> Result<()> {
    let normalized = path.replace('\\', "/");
    conn.execute(
        "INSERT OR IGNORE INTO library_paths (path) VALUES (?1)",
        params![normalized],
    )?;
    Ok(())
}

pub fn remove_library_path(conn: &Connection, path: &str) -> Result<()> {
    conn.execute("DELETE FROM library_paths WHERE path = ?1", params![path])?;
    Ok(())
}

pub fn get_library_paths(conn: &Connection) -> Result<Vec<LibraryPath>> {
    let mut stmt = conn.prepare("SELECT id, path FROM library_paths")?;
    let paths = stmt
        .query_map([], |row| {
            Ok(LibraryPath {
                id: row.get(0)?,
                path: row.get::<_, String>(1)?.replace('\\', "/"),
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(paths)
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
		"INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
		params![key, value],
	)?;
    Ok(())
}

pub fn get_all_settings(conn: &Connection) -> Result<Vec<Setting>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let settings = stmt
        .query_map([], |row| {
            Ok(Setting {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(settings)
}

pub fn add_uid_remap(
    conn: &Connection,
    old_uid: &str,
    new_uid: &str,
    entity_type: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO uuid_remaps (shared_uid, local_uid, entity_type) VALUES (?1, ?2, ?3)
		ON CONFLICT(shared_uid, entity_type) DO UPDATE SET local_uid = excluded.local_uid",
        params![old_uid, new_uid, entity_type],
    )?;
    Ok(())
}

pub fn resolve_uid(conn: &Connection, uid: &str) -> Result<String> {
    let mut stmt = conn.prepare("SELECT local_uid FROM uuid_remaps WHERE shared_uid = ?1")?;
    let mut rows = stmt.query(params![uid])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(uid.to_string())
    }
}
