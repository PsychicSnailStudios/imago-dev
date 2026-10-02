use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistEntry {
    pub uid: String,
    pub entity_type: String,
    pub blocked_at: i64,
    pub reason: Option<String>,
    pub cascade: bool,
    pub source_lib_uid: String,
}

// cascade_delete values:
// 0 = single only (never cascade)
// 1 = always cascade
// 2 = always ask (default)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDeletePreference {
    pub lib_uid: String,
    pub cascade_delete: i64,
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn add_to_blocklist(
    conn: &Connection,
    uid: &str,
    entity_type: &str,
    reason: Option<&str>,
    cascade: bool,
    source_lib_uid: &str,
) -> Result<()> {
    conn.execute(
		"INSERT OR IGNORE INTO blocklist (uid, entity_type, blocked_at, reason, cascade, source_lib_uid)
		VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
		params![
			uid,
			entity_type,
			now_ts(),
			reason,
			cascade as i64,
			source_lib_uid,
		],
	)?;
    Ok(())
}

pub fn remove_from_blocklist(conn: &Connection, uid: &str) -> Result<()> {
    conn.execute("DELETE FROM blocklist WHERE uid = ?1", params![uid])?;
    Ok(())
}

pub fn get_all_blocklist_entries(conn: &Connection) -> Result<Vec<BlocklistEntry>> {
    let mut stmt = conn.prepare(
        "SELECT uid, entity_type, blocked_at, reason, cascade, source_lib_uid
		FROM blocklist ORDER BY blocked_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(BlocklistEntry {
            uid: row.get(0)?,
            entity_type: row.get(1)?,
            blocked_at: row.get(2)?,
            reason: row.get(3)?,
            cascade: row.get::<_, i64>(4)? != 0,
            source_lib_uid: row.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn is_blocked(conn: &Connection, uid: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM blocklist WHERE uid = ?1",
        params![uid],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

// Returns the full set of blocked UIDs as a HashSet for fast lookup during merge builds.
pub fn get_blocked_uid_set(conn: &Connection) -> Result<std::collections::HashSet<String>> {
    let mut stmt = conn.prepare("SELECT uid FROM blocklist")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut set = std::collections::HashSet::new();
    for row in rows {
        set.insert(row?);
    }
    Ok(set)
}

pub fn get_delete_preference(
    conn: &Connection,
    lib_uid: &str,
) -> Result<Option<LibraryDeletePreference>> {
    let mut stmt = conn.prepare(
        "SELECT lib_uid, cascade_delete FROM library_delete_preferences WHERE lib_uid = ?1",
    )?;
    let mut rows = stmt.query(params![lib_uid])?;
    if let Some(row) = rows.next()? {
        Ok(Some(LibraryDeletePreference {
            lib_uid: row.get(0)?,
            cascade_delete: row.get(1)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn set_delete_preference(conn: &Connection, lib_uid: &str, cascade_delete: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO library_delete_preferences (lib_uid, cascade_delete) VALUES (?1, ?2)
		ON CONFLICT(lib_uid) DO UPDATE SET cascade_delete = excluded.cascade_delete",
        params![lib_uid, cascade_delete],
    )?;
    Ok(())
}
