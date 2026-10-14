use super::track_manager::{get_track_by_uid, Track};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathEntry {
    pub path: String,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub bitrate: Option<i64>,
    #[serde(default)]
    pub last_modified: Option<i64>,
}

pub fn path_key(path: &str) -> String {
    path.replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

pub fn is_under(path: &str, key: &str) -> bool {
    let pk = path_key(path);
    pk == key || pk.starts_with(&format!("{}/", key))
}

fn is_lossless(format: &str, bitrate: i64) -> bool {
    let f = format.to_lowercase();
    matches!(
        f.as_str(),
        "flac" | "wav" | "aiff" | "aif" | "alac" | "ape" | "wv" | "tta"
    ) || (f == "m4a" && bitrate >= 800)
}

pub fn quality_score(format: Option<&str>, bitrate: Option<i64>) -> i64 {
    let br = bitrate.unwrap_or(0).max(0);
    match format {
        Some(f) if is_lossless(f, br) => 1_000_000 + br,
        _ => br,
    }
}

impl PathEntry {
    pub fn score(&self) -> i64 {
        quality_score(self.format.as_deref(), self.bitrate)
    }
}

pub fn parse_entries(json: &Option<String>) -> Vec<PathEntry> {
    json.as_deref()
        .and_then(|j| serde_json::from_str::<Vec<PathEntry>>(j).ok())
        .unwrap_or_default()
}

fn format_and_bitrate(data: &Option<String>) -> (Option<String>, Option<i64>) {
    let Some(value) = data
        .as_deref()
        .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
    else {
        return (None, None);
    };
    let format = value
        .get("format")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let bitrate = value
        .get("bitrate")
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)));
    (format, bitrate)
}

pub fn local_entry(track: &Track) -> Option<PathEntry> {
    if track.path.is_empty() || track.path == track.uid {
        return None;
    }
    let (format, bitrate) = format_and_bitrate(&track.track_data);
    Some(PathEntry {
        path: track.path.clone(),
        format: format.or_else(|| track.format.clone()),
        bitrate: bitrate.or(track.bitrate),
        last_modified: Some(track.last_modified),
    })
}

pub fn remote_entry(track: &Track) -> Option<PathEntry> {
    let remote = track.remote_path.as_deref().filter(|p| !p.is_empty())?;
    let (mut format, mut bitrate) = format_and_bitrate(&track.remote_data);
    if format.is_none() && bitrate.is_none() && track.path.is_empty() {
        let (f, b) = format_and_bitrate(&track.track_data);
        format = f.or_else(|| track.format.clone());
        bitrate = b.or(track.bitrate);
    }
    Some(PathEntry {
        path: remote.to_string(),
        format,
        bitrate,
        last_modified: Some(track.last_modified),
    })
}

pub fn all_entries(track: &Track) -> (Vec<PathEntry>, Vec<PathEntry>) {
    let mut locals: Vec<PathEntry> = local_entry(track).into_iter().collect();
    locals.extend(parse_entries(&track.extra_paths));
    let mut remotes: Vec<PathEntry> = remote_entry(track).into_iter().collect();
    remotes.extend(parse_entries(&track.extra_remote_paths));
    (locals, remotes)
}

fn dedupe_sort(mut entries: Vec<PathEntry>) -> Vec<PathEntry> {
    let mut seen: HashSet<String> = HashSet::new();
    entries.retain(|e| !e.path.is_empty() && seen.insert(path_key(&e.path)));
    entries.sort_by(|a, b| b.score().cmp(&a.score()));
    entries
}

fn patch_json(existing: &Option<String>, pairs: &[(&str, serde_json::Value)]) -> String {
    let mut value = existing
        .as_deref()
        .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = value.as_object_mut() {
        for (k, v) in pairs {
            obj.insert((*k).to_string(), v.clone());
        }
    }
    value.to_string()
}

fn entries_json(entries: &[PathEntry]) -> Option<String> {
    if entries.is_empty() {
        None
    } else {
        serde_json::to_string(entries).ok()
    }
}

pub fn write_entries(
    conn: &Connection,
    uid: &str,
    locals: Vec<PathEntry>,
    remotes: Vec<PathEntry>,
) -> Result<()> {
    let locals = dedupe_sort(locals);
    let remotes = dedupe_sort(remotes);

    let (track_data, remote_data): (Option<String>, Option<String>) = conn.query_row(
        "SELECT track_data, remote_data FROM tracks WHERE uid = ?1",
        params![uid],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let main_local = locals.first();
    let main_remote = remotes.first();
    let extra_locals: &[PathEntry] = if locals.len() > 1 { &locals[1..] } else { &[] };
    let extra_remotes: &[PathEntry] = if remotes.len() > 1 { &remotes[1..] } else { &[] };

    let new_track_data = match main_local {
        Some(e) => Some(patch_json(
            &track_data,
            &[
                ("format", serde_json::json!(e.format)),
                ("bitrate", serde_json::json!(e.bitrate)),
                ("is_ghost", serde_json::Value::Null),
            ],
        )),
        None => track_data.clone(),
    };
    let new_remote_data = match main_remote {
        Some(e) => Some(patch_json(
            &remote_data,
            &[
                ("format", serde_json::json!(e.format)),
                ("bitrate", serde_json::json!(e.bitrate)),
            ],
        )),
        None => remote_data.clone(),
    };

    conn.execute(
        "UPDATE tracks SET
            path = ?2,
            remote_path = ?3,
            extra_paths = ?4,
            extra_remote_paths = ?5,
            format = CASE WHEN ?6 IS NOT NULL THEN ?6 ELSE format END,
            bitrate = CASE WHEN ?7 IS NOT NULL THEN ?7 ELSE bitrate END,
            last_modified = COALESCE(?8, last_modified),
            track_data = ?9,
            remote_data = ?10
         WHERE uid = ?1",
        params![
            uid,
            main_local.map(|e| e.path.clone()).unwrap_or_default(),
            main_remote.map(|e| e.path.clone()).unwrap_or_default(),
            entries_json(extra_locals),
            entries_json(extra_remotes),
            main_local.and_then(|e| e.format.clone()),
            main_local.and_then(|e| e.bitrate),
            main_local.and_then(|e| e.last_modified),
            new_track_data,
            new_remote_data,
        ],
    )?;
    Ok(())
}

pub fn merge_track_into(conn: &Connection, keep_uid: &str, incoming: &Track) -> Result<()> {
    let Some(keep) = get_track_by_uid(conn, keep_uid)? else {
        return Ok(());
    };
    let (mut locals, mut remotes) = all_entries(&keep);
    let (incoming_locals, incoming_remotes) = all_entries(incoming);
    locals.extend(incoming_locals);
    remotes.extend(incoming_remotes);
    write_entries(conn, keep_uid, locals, remotes)
}

pub fn extra_path_keys(conn: &Connection, uid: &str) -> Vec<String> {
    match get_track_by_uid(conn, uid) {
        Ok(Some(track)) => parse_entries(&track.extra_paths)
            .into_iter()
            .chain(parse_entries(&track.extra_remote_paths))
            .map(|e| e.path.replace('\\', "/").to_lowercase())
            .collect(),
        _ => Vec::new(),
    }
}

pub fn refresh_extra_entry(conn: &Connection, owner_uid: &str, incoming: &Track) -> Result<()> {
    let Some(owner) = get_track_by_uid(conn, owner_uid)? else {
        return Ok(());
    };
    let (mut locals, mut remotes) = all_entries(&owner);
    for fresh in local_entry(incoming).into_iter() {
        let key = path_key(&fresh.path);
        for entry in locals.iter_mut().filter(|e| path_key(&e.path) == key) {
            entry.format = fresh.format.clone();
            entry.bitrate = fresh.bitrate;
            entry.last_modified = fresh.last_modified;
        }
    }
    for fresh in remote_entry(incoming).into_iter() {
        let key = path_key(&fresh.path);
        for entry in remotes.iter_mut().filter(|e| path_key(&e.path) == key) {
            entry.format = fresh.format.clone();
            entry.bitrate = fresh.bitrate;
            entry.last_modified = fresh.last_modified;
        }
    }
    write_entries(conn, owner_uid, locals, remotes)
}

fn delete_rows(conn: &Connection, uids: &[String]) -> Result<()> {
    for uid in uids {
        conn.execute(
            "DELETE FROM lyrics WHERE track_id IN (SELECT id FROM tracks WHERE uid = ?1)",
            params![uid],
        )?;
        conn.execute("DELETE FROM tracks WHERE uid = ?1", params![uid])?;
    }
    Ok(())
}

fn detach_candidates(
    conn: &Connection,
    uids: &[String],
    matches: &dyn Fn(&str) -> bool,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut deleted: Vec<String> = Vec::new();
    let mut modified: Vec<String> = Vec::new();

    for uid in uids {
        let Some(track) = get_track_by_uid(conn, uid)? else {
            continue;
        };
        let (locals, remotes) = all_entries(&track);
        let total = locals.len() + remotes.len();
        let kept_locals: Vec<PathEntry> = locals.into_iter().filter(|e| !matches(&e.path)).collect();
        let kept_remotes: Vec<PathEntry> =
            remotes.into_iter().filter(|e| !matches(&e.path)).collect();
        if kept_locals.len() + kept_remotes.len() == total {
            continue;
        }
        if kept_locals.is_empty() && kept_remotes.is_empty() {
            deleted.push(uid.clone());
        } else {
            write_entries(conn, uid, kept_locals, kept_remotes)?;
            modified.push(uid.clone());
        }
    }

    delete_rows(conn, &deleted)?;
    Ok((deleted, modified))
}

pub fn detach_prefix(conn: &Connection, key: &str) -> Result<(Vec<String>, Vec<String>)> {
    let uids: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT uid FROM tracks
             WHERE instr(LOWER(REPLACE(COALESCE(path, ''), char(92), '/')), ?1) = 1
                OR instr(LOWER(COALESCE(remote_path, '')), ?1) = 1
                OR COALESCE(extra_paths, '') != ''
                OR COALESCE(extra_remote_paths, '') != ''",
        )?;
        let rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>>>()?
    };
    let key = key.to_string();
    detach_candidates(conn, &uids, &|p| is_under(p, &key))
}

pub fn detach_exact(conn: &Connection, path: &str) -> Result<(Vec<String>, Vec<String>)> {
    let escaped = serde_json::to_string(path)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();
    let like = format!("%{}%", escaped);
    let uids: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT uid FROM tracks
             WHERE path = ?1 OR remote_path = ?1
                OR LOWER(REPLACE(COALESCE(path, ''), char(92), '/')) = ?3
                OR LOWER(COALESCE(remote_path, '')) = ?3
                OR extra_paths LIKE ?2 OR extra_remote_paths LIKE ?2",
        )?;
        let rows = stmt.query_map(params![path, like, path_key(path)], |row| {
            row.get::<_, String>(0)
        })?;
        rows.collect::<Result<Vec<_>>>()?
    };
    let key = path_key(path);
    detach_candidates(conn, &uids, &|p| path_key(p) == key)
}

fn names_of(json: &Option<String>) -> Vec<String> {
    json.as_deref()
        .and_then(|j| serde_json::from_str::<Vec<serde_json::Value>>(j).ok())
        .unwrap_or_default()
        .iter()
        .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn same_album(a: &Option<String>, b: &Option<String>) -> bool {
    let a = names_of(a);
    let b = names_of(b);
    if a.is_empty() && b.is_empty() {
        return true;
    }
    if a.is_empty() || b.is_empty() {
        return false;
    }
    a.iter().any(|x| b.contains(x))
}

fn union_albums(keep: &Option<String>, other: &Option<String>) -> Option<String> {
    let mut list: Vec<serde_json::Value> = keep
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    let extra: Vec<serde_json::Value> = other
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    for entry in extra {
        let uid = entry.get("uid").and_then(|v| v.as_str()).unwrap_or("");
        let name = entry
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_lowercase();
        let exists = list.iter().any(|e| {
            let eu = e.get("uid").and_then(|v| v.as_str()).unwrap_or("");
            let en = e
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();
            (!uid.is_empty() && eu == uid) || (!name.is_empty() && en == name)
        });
        if !exists {
            list.push(entry);
        }
    }
    if list.is_empty() {
        keep.clone()
    } else {
        serde_json::to_string(&list).ok()
    }
}

fn union_tags(keep: &Option<String>, other: &Option<String>) -> Option<String> {
    let mut list: Vec<String> = keep
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    let extra: Vec<String> = other
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    for tag in extra {
        if !list.iter().any(|t| t.eq_ignore_ascii_case(&tag)) {
            list.push(tag);
        }
    }
    if list.is_empty() {
        keep.clone()
    } else {
        serde_json::to_string(&list).ok()
    }
}

fn repoint_album_tracks(conn: &Connection, drop_uid: &str, keep_uid: &str) -> Result<Vec<String>> {
    let mut changed: Vec<String> = Vec::new();
    let rows: Vec<(String, String)> = {
        let mut stmt =
            conn.prepare("SELECT uid, tracks FROM albums WHERE tracks LIKE ?1")?;
        let collected = stmt
            .query_map(params![format!("%{}%", drop_uid)], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();
        collected
    };
    for (album_uid, json) in rows {
        let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&json) else {
            continue;
        };
        let mut rebuilt: Vec<serde_json::Value> = Vec::new();
        let mut seen_keep = false;
        for mut entry in list {
            let uid = entry
                .get("uid")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let target = if uid == drop_uid { keep_uid.to_string() } else { uid };
            if target == keep_uid {
                if seen_keep {
                    continue;
                }
                seen_keep = true;
                if let Some(obj) = entry.as_object_mut() {
                    obj.insert("uid".to_string(), serde_json::Value::String(target));
                }
            }
            rebuilt.push(entry);
        }
        if let Ok(out) = serde_json::to_string(&rebuilt) {
            conn.execute(
                "UPDATE albums SET tracks = ?1 WHERE uid = ?2",
                params![out, album_uid],
            )?;
            changed.push(album_uid);
        }
    }
    Ok(changed)
}

pub fn merge_track_rows(conn: &Connection, keep_uid: &str, drop_uid: &str) -> Result<Vec<String>> {
    if keep_uid == drop_uid {
        return Ok(Vec::new());
    }
    let (Some(keep), Some(dropped)) = (
        get_track_by_uid(conn, keep_uid)?,
        get_track_by_uid(conn, drop_uid)?,
    ) else {
        return Ok(Vec::new());
    };

    merge_track_into(conn, keep_uid, &dropped)?;

    conn.execute(
        "UPDATE tracks SET
            title        = COALESCE(title,        ?2),
            artists      = COALESCE(artists,      ?3),
            album_artist = COALESCE(album_artist, ?4),
            genres       = COALESCE(genres,       ?5),
            year         = COALESCE(year,         ?6),
            duration_ms  = COALESCE(duration_ms,  ?7),
            bpm          = COALESCE(bpm,          ?8),
            key          = COALESCE(key,          ?9),
            credits      = COALESCE(credits,      ?10),
            label        = COALESCE(label,        ?11),
            rating       = COALESCE(rating,       ?12),
            albums       = ?13,
            tags         = ?14
         WHERE uid = ?1",
        params![
            keep_uid,
            dropped.title,
            dropped.artists,
            dropped.album_artist,
            dropped.genres,
            dropped.year,
            dropped.duration_ms,
            dropped.bpm,
            dropped.key,
            dropped.credits,
            dropped.label,
            dropped.rating,
            union_albums(&keep.albums, &dropped.albums),
            union_tags(&keep.tags, &dropped.tags),
        ],
    )?;

    conn.execute(
        "UPDATE tracks SET
            artwork_blob  = COALESCE(artwork_blob,  (SELECT artwork_blob  FROM tracks WHERE uid = ?2)),
            artwork_thumb = COALESCE(artwork_thumb, (SELECT artwork_thumb FROM tracks WHERE uid = ?2)),
            artwork_path  = COALESCE(artwork_path,  (SELECT artwork_path  FROM tracks WHERE uid = ?2))
         WHERE uid = ?1",
        params![keep_uid, drop_uid],
    )?;

    conn.execute(
        "UPDATE lyrics SET track_id = (SELECT id FROM tracks WHERE uid = ?1)
         WHERE track_id = (SELECT id FROM tracks WHERE uid = ?2)
           AND NOT EXISTS (
               SELECT 1 FROM lyrics WHERE track_id = (SELECT id FROM tracks WHERE uid = ?1)
           )",
        params![keep_uid, drop_uid],
    )?;

    let changed_albums = repoint_album_tracks(conn, drop_uid, keep_uid)?;
    delete_rows(conn, &[drop_uid.to_string()])?;
    Ok(changed_albums)
}

pub fn keeper_score(track: &Track) -> (i64, i64) {
    let mut user_data = 0;
    if track.rating.is_some() {
        user_data += 1;
    }
    if track
        .tags
        .as_deref()
        .map(|t| t != "[]" && !t.is_empty())
        .unwrap_or(false)
    {
        user_data += 1;
    }
    let (locals, remotes) = all_entries(track);
    let best = locals
        .iter()
        .chain(remotes.iter())
        .map(|e| e.score())
        .max()
        .unwrap_or(0);
    (user_data, best)
}
