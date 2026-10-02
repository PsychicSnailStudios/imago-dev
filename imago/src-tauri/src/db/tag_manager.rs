// src-tauri/src/db/tag_manager.rs
//
// Tag & genre dictionary with group support.
// Tags and genres are tracked in two tables:
//   - `tags`   — global tag dictionary
//   - `genres` — global genre dictionary
//   - `tag_groups` — named groups that bundle tags together
//   - `tag_group_members` — many-to-many: which tags belong to which group
//
// All four tables live in lib.db and are created by init_lib_db().
//
// Propagation (rename / delete) is done by scanning the JSON arrays in
// tracks.tags, albums.tags, artists.tags, tracks.genres, albums.genres,
// and artists.genres and rewriting them in-place.

use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

// ─── Data Structures ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub uid: String,
    pub name: String,
    pub kind: TagKind,         // "tag" | "genre"
    pub color: Option<String>, // hex color hint for the UI, e.g. "#f59e0b"
    pub created_at: i64,       // unix timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TagKind {
    Tag,
    Genre,
}

impl std::fmt::Display for TagKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagKind::Tag => write!(f, "tag"),
            TagKind::Genre => write!(f, "genre"),
        }
    }
}

impl TagKind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "genre" => TagKind::Genre,
            _ => TagKind::Tag,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagGroup {
    pub uid: String,
    pub name: String,
    pub kind: TagKind, // groups are either all-tags or all-genres
    pub color: Option<String>,
    pub member_uids: Vec<String>, // tag UIDs belonging to this group
    pub created_at: i64,
}

// ─── Schema (call from init_lib_db) ───────────────────────────────────────────

pub fn create_tag_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
		CREATE TABLE IF NOT EXISTS tags (
			uid         TEXT    PRIMARY KEY NOT NULL,
			name        TEXT    NOT NULL COLLATE NOCASE,
			kind        TEXT    NOT NULL DEFAULT 'tag',
			color       TEXT,
			created_at  INTEGER NOT NULL,
			UNIQUE(name, kind)
		);

		CREATE TABLE IF NOT EXISTS tag_groups (
			uid         TEXT    PRIMARY KEY NOT NULL,
			name        TEXT    NOT NULL UNIQUE COLLATE NOCASE,
			kind        TEXT    NOT NULL DEFAULT 'tag',
			color       TEXT,
			created_at  INTEGER NOT NULL
		);

		CREATE TABLE IF NOT EXISTS tag_group_members (
			group_uid   TEXT    NOT NULL REFERENCES tag_groups(uid) ON DELETE CASCADE,
			tag_uid     TEXT    NOT NULL REFERENCES tags(uid) ON DELETE CASCADE,
			PRIMARY KEY (group_uid, tag_uid)
		);

		CREATE INDEX IF NOT EXISTS idx_tag_group_members_tag
			ON tag_group_members(tag_uid);
		",
    )
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Rewrite a JSON array string, replacing `old_name` with `new_name`.
/// Returns the modified JSON or `None` if the value was not present.
fn json_rename(json: &str, old_name: &str, new_name: &str) -> Option<String> {
    let mut arr: Vec<String> = serde_json::from_str(json).ok()?;
    let mut changed = false;
    for item in arr.iter_mut() {
        if item.eq_ignore_ascii_case(old_name) {
            *item = new_name.to_string();
            changed = true;
        }
    }
    if changed {
        Some(serde_json::to_string(&arr).unwrap_or_else(|_| json.to_string()))
    } else {
        None
    }
}

/// Rewrite a JSON array string, removing `name`.
fn json_remove(json: &str, name: &str) -> Option<String> {
    let mut arr: Vec<String> = serde_json::from_str(json).ok()?;
    let before = arr.len();
    arr.retain(|item| !item.eq_ignore_ascii_case(name));
    if arr.len() < before {
        Some(serde_json::to_string(&arr).unwrap_or_else(|_| json.to_string()))
    } else {
        None
    }
}

// ─── Tag CRUD ─────────────────────────────────────────────────────────────────

/// Insert a new tag/genre into the dictionary.
/// Returns the uid. If a tag with the same name (case-insensitive) already
/// exists, the existing uid is returned and no row is inserted.
pub fn add_tag(
    conn: &Connection,
    name: &str,
    kind: TagKind,
    color: Option<&str>,
) -> Result<String> {
    let kind_str = kind.to_string();

    let existing: Option<String> = conn
        .query_row(
            "SELECT uid FROM tags WHERE name = ?1 COLLATE NOCASE AND kind = ?2",
            params![name, kind_str],
            |row| row.get(0),
        )
        .ok();

    if let Some(uid) = existing {
        return Ok(uid);
    }

    let uid = crate::db::generate_uid("tg");
    conn.execute(
        "INSERT OR IGNORE INTO tags (uid, name, kind, color, created_at) VALUES (?1,?2,?3,?4,?5)",
        params![uid, name, kind_str, color, now_ts()],
    )?;
    Ok(uid)
}

/// Convenience — called from scanner/enrichment when a tag string is encountered.
/// Silently upserts without error.
pub fn ensure_tag(conn: &Connection, name: &str, kind: TagKind) {
    let _ = add_tag(conn, name, kind, None);
}

pub fn get_all_tags(conn: &Connection) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT uid, name, kind, color, created_at FROM tags ORDER BY kind, name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Tag {
            uid: row.get(0)?,
            name: row.get(1)?,
            kind: TagKind::from_str(&row.get::<_, String>(2)?),
            color: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn get_tag_by_uid(conn: &Connection, uid: &str) -> Result<Option<Tag>> {
    conn.query_row(
        "SELECT uid, name, kind, color, created_at FROM tags WHERE uid = ?1",
        params![uid],
        |row| {
            Ok(Tag {
                uid: row.get(0)?,
                name: row.get(1)?,
                kind: TagKind::from_str(&row.get::<_, String>(2)?),
                color: row.get(3)?,
                created_at: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.into())
}

/// Rename a tag everywhere — dictionary row + all JSON arrays on entities.
pub fn rename_tag(conn: &Connection, uid: &str, new_name: &str) -> Result<()> {
    let tag = match get_tag_by_uid(conn, uid)? {
        Some(t) => t,
        None => return Ok(()),
    };
    let old_name = &tag.name;
    let field = match tag.kind {
        TagKind::Tag => "tags",
        TagKind::Genre => "genres",
    };

    // Update dictionary
    conn.execute(
        "UPDATE tags SET name = ?1 WHERE uid = ?2",
        params![new_name, uid],
    )?;

    // Propagate to tracks, albums, artists
    for table in &["tracks", "albums", "artists"] {
        propagate_rename(conn, table, field, old_name, new_name)?;
    }
    Ok(())
}

/// Delete a tag everywhere.
pub fn delete_tag(conn: &Connection, uid: &str) -> Result<()> {
    let tag = match get_tag_by_uid(conn, uid)? {
        Some(t) => t,
        None => return Ok(()),
    };
    let name = &tag.name;
    let field = match tag.kind {
        TagKind::Tag => "tags",
        TagKind::Genre => "genres",
    };

    // Remove from all entities first
    for table in &["tracks", "albums", "artists"] {
        propagate_remove(conn, table, field, name)?;
    }

    // Remove from dictionary (cascade removes group memberships)
    conn.execute("DELETE FROM tags WHERE uid = ?1", params![uid])?;
    Ok(())
}

/// Update a tag's color.
pub fn update_tag_color(conn: &Connection, uid: &str, color: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE tags SET color = ?1 WHERE uid = ?2",
        params![color, uid],
    )?;
    Ok(())
}

// ─── Tag Group CRUD ───────────────────────────────────────────────────────────

pub fn create_tag_group(
    conn: &Connection,
    name: &str,
    kind: TagKind,
    color: Option<&str>,
    member_uids: &[String],
) -> Result<TagGroup> {
    let uid = crate::db::generate_uid("tgg");
    let kind_str = kind.to_string();
    conn.execute(
        "INSERT INTO tag_groups (uid, name, kind, color, created_at) VALUES (?1,?2,?3,?4,?5)",
        params![uid, name, kind_str, color, now_ts()],
    )?;
    set_group_members(conn, &uid, member_uids)?;
    Ok(TagGroup {
        uid: uid.clone(),
        name: name.to_string(),
        kind,
        color: color.map(|s| s.to_string()),
        member_uids: member_uids.to_vec(),
        created_at: now_ts(),
    })
}

pub fn get_all_tag_groups(conn: &Connection) -> Result<Vec<TagGroup>> {
    let mut stmt = conn.prepare(
        "SELECT uid, name, kind, color, created_at FROM tag_groups ORDER BY kind, name COLLATE NOCASE",
    )?;
    let groups: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut result = Vec::new();
    for (uid, name, kind_str, color, created_at) in groups {
        let member_uids = get_group_member_uids(conn, &uid)?;
        result.push(TagGroup {
            uid,
            name,
            kind: TagKind::from_str(&kind_str),
            color,
            member_uids,
            created_at,
        });
    }
    Ok(result)
}

pub fn update_tag_group(
    conn: &Connection,
    uid: &str,
    name: Option<&str>,
    color: Option<Option<&str>>,
    member_uids: Option<&[String]>,
) -> Result<()> {
    if let Some(n) = name {
        conn.execute(
            "UPDATE tag_groups SET name = ?1 WHERE uid = ?2",
            params![n, uid],
        )?;
    }
    if let Some(c) = color {
        conn.execute(
            "UPDATE tag_groups SET color = ?1 WHERE uid = ?2",
            params![c, uid],
        )?;
    }
    if let Some(members) = member_uids {
        set_group_members(conn, uid, members)?;
    }
    Ok(())
}

pub fn delete_tag_group(conn: &Connection, uid: &str) -> Result<()> {
    // CASCADE handles tag_group_members
    conn.execute("DELETE FROM tag_groups WHERE uid = ?1", params![uid])?;
    Ok(())
}

fn set_group_members(conn: &Connection, group_uid: &str, member_uids: &[String]) -> Result<()> {
    conn.execute(
        "DELETE FROM tag_group_members WHERE group_uid = ?1",
        params![group_uid],
    )?;
    for tag_uid in member_uids {
        conn.execute(
            "INSERT OR IGNORE INTO tag_group_members (group_uid, tag_uid) VALUES (?1,?2)",
            params![group_uid, tag_uid],
        )?;
    }
    Ok(())
}

fn get_group_member_uids(conn: &Connection, group_uid: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT tgm.tag_uid FROM tag_group_members tgm
         JOIN tags t ON t.uid = tgm.tag_uid
         WHERE tgm.group_uid = ?1
         ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map(params![group_uid], |row| row.get(0))?;
    rows.collect()
}

// ─── Propagation helpers ──────────────────────────────────────────────────────

/// Walk every row in `table`.`field` (a JSON string array) and rename a value.
fn propagate_rename(
    conn: &Connection,
    table: &str,
    field: &str,
    old: &str,
    new: &str,
) -> Result<()> {
    let sql =
        format!("SELECT rowid, {field} FROM {table} WHERE {field} IS NOT NULL AND {field} != '[]'");
    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    for (rowid, json) in rows {
        if let Some(new_json) = json_rename(&json, old, new) {
            let upd = format!("UPDATE {table} SET {field} = ?1 WHERE rowid = ?2");
            conn.execute(&upd, params![new_json, rowid])?;
        }
    }
    Ok(())
}

/// Walk every row and remove a value from the JSON array.
fn propagate_remove(conn: &Connection, table: &str, field: &str, name: &str) -> Result<()> {
    let sql =
        format!("SELECT rowid, {field} FROM {table} WHERE {field} IS NOT NULL AND {field} != '[]'");
    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    for (rowid, json) in rows {
        if let Some(new_json) = json_remove(&json, name) {
            let upd = format!("UPDATE {table} SET {field} = ?1 WHERE rowid = ?2");
            conn.execute(&upd, params![new_json, rowid])?;
        }
    }
    Ok(())
}

// ─── Extension trait for `optional()` on rusqlite ────────────────────────────

trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>>;
}

impl<T> OptionalExt<T> for Result<T> {
    fn optional(self) -> Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
