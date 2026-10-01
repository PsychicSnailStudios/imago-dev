use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub uid: String,
    pub name: String,
    pub is_default: bool,
    pub file_path: String,
    pub sync_url: Option<String>,
    pub sync_meta_url: Option<String>,
    // write_token is never serialised to the frontend
    #[serde(skip_serializing)]
    pub write_token: Option<String>,
    pub has_write_permission: bool,
    pub last_synced: i64,
    pub last_data_version: i64,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryUpdate {
    pub name: Option<String>,
    pub sync_url: Option<String>,
    pub sync_meta_url: Option<String>,
    pub write_token: Option<String>,
    pub has_write_permission: Option<bool>,
    pub last_synced: Option<i64>,
    pub last_data_version: Option<i64>,
    pub sort_order: Option<i64>,
}

fn row_to_library(row: &rusqlite::Row) -> rusqlite::Result<Library> {
    Ok(Library {
        uid: row.get(0)?,
        name: row.get(1)?,
        is_default: row.get::<_, i64>(2)? != 0,
        file_path: row.get(3)?,
        sync_url: row.get(4)?,
        sync_meta_url: row.get(5)?,
        write_token: row.get(6)?,
        has_write_permission: row.get::<_, i64>(7)? != 0,
        last_synced: row.get(8)?,
        last_data_version: row.get(9)?,
        sort_order: row.get(10)?,
    })
}

const SELECT_COLS: &str = "uid, name, is_default, file_path, sync_url, sync_meta_url, write_token,
	has_write_permission, last_synced, last_data_version, sort_order";

pub fn create_library(conn: &Connection, library: &Library) -> Result<()> {
    conn.execute(
        &format!(
            "INSERT INTO libraries ({}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            SELECT_COLS
        ),
        params![
            library.uid,
            library.name,
            library.is_default as i64,
            library.file_path,
            library.sync_url,
            library.sync_meta_url,
            library.write_token,
            library.has_write_permission as i64,
            library.last_synced,
            library.last_data_version,
            library.sort_order,
        ],
    )?;
    Ok(())
}

pub fn get_all_libraries(conn: &Connection) -> Result<Vec<Library>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM libraries ORDER BY sort_order ASC",
        SELECT_COLS
    ))?;
    let rows = stmt.query_map([], row_to_library)?;
    rows.collect()
}

pub fn get_library_by_uid(conn: &Connection, uid: &str) -> Result<Option<Library>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM libraries WHERE uid = ?1",
        SELECT_COLS
    ))?;
    let mut rows = stmt.query(params![uid])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row_to_library(row)?))
    } else {
        Ok(None)
    }
}

pub fn get_default_library(conn: &Connection) -> Result<Option<Library>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM libraries WHERE is_default = 1 LIMIT 1",
        SELECT_COLS
    ))?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row_to_library(row)?))
    } else {
        Ok(None)
    }
}

pub fn update_library(conn: &Connection, uid: &str, update: &LibraryUpdate) -> Result<()> {
    if let Some(ref name) = update.name {
        conn.execute(
            "UPDATE libraries SET name = ?1 WHERE uid = ?2",
            params![name, uid],
        )?;
    }
    if let Some(ref sync_url) = update.sync_url {
        conn.execute(
            "UPDATE libraries SET sync_url = ?1 WHERE uid = ?2",
            params![sync_url, uid],
        )?;
    }
    if let Some(ref sync_meta_url) = update.sync_meta_url {
        conn.execute(
            "UPDATE libraries SET sync_meta_url = ?1 WHERE uid = ?2",
            params![sync_meta_url, uid],
        )?;
    }
    if let Some(ref write_token) = update.write_token {
        conn.execute(
            "UPDATE libraries SET write_token = ?1 WHERE uid = ?2",
            params![write_token, uid],
        )?;
    }
    if let Some(has_write_permission) = update.has_write_permission {
        conn.execute(
            "UPDATE libraries SET has_write_permission = ?1 WHERE uid = ?2",
            params![has_write_permission as i64, uid],
        )?;
    }
    if let Some(last_synced) = update.last_synced {
        conn.execute(
            "UPDATE libraries SET last_synced = ?1 WHERE uid = ?2",
            params![last_synced, uid],
        )?;
    }
    if let Some(last_data_version) = update.last_data_version {
        conn.execute(
            "UPDATE libraries SET last_data_version = ?1 WHERE uid = ?2",
            params![last_data_version, uid],
        )?;
    }
    if let Some(sort_order) = update.sort_order {
        conn.execute(
            "UPDATE libraries SET sort_order = ?1 WHERE uid = ?2",
            params![sort_order, uid],
        )?;
    }
    Ok(())
}

pub fn delete_library(conn: &Connection, uid: &str) -> Result<()> {
    conn.execute("DELETE FROM libraries WHERE uid = ?1", params![uid])?;
    Ok(())
}

// Reads the SQLite data_version pragma from a library db file.
// Returns 0 if the file cannot be opened.
pub fn read_data_version(file_path: &str) -> i64 {
    Connection::open(file_path)
        .ok()
        .and_then(|conn| {
            conn.query_row("PRAGMA data_version", [], |row| row.get::<_, i64>(0))
                .ok()
        })
        .unwrap_or(0)
}

// Checks which libraries have changed since the last recorded data_version.
// Returns a vec of library uids that need reprocessing.
pub fn detect_changed_libraries(conn: &Connection) -> Result<Vec<String>> {
    let libs = get_all_libraries(conn)?;
    let mut changed = Vec::new();
    for lib in libs {
        let current = read_data_version(&lib.file_path);
        if current != lib.last_data_version {
            changed.push(lib.uid);
        }
    }
    Ok(changed)
}

// Updates the stored data_version for a library after processing.
pub fn store_data_version(conn: &Connection, uid: &str, version: i64) -> Result<()> {
    conn.execute(
        "UPDATE libraries SET last_data_version = ?1 WHERE uid = ?2",
        params![version, uid],
    )?;
    Ok(())
}
