use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaylistTrackEntry {
    pub uid: String,
    pub name: String,
    pub order: u32,
}

pub fn normalize_playlist_track_order(tracks: &mut Vec<PlaylistTrackEntry>) {
    let any_nonzero = tracks.iter().any(|t| t.order != 0);
    if !any_nonzero {
        for (i, t) in tracks.iter_mut().enumerate() {
            t.order = (i + 1) as u32;
        }
    } else {
        tracks.sort_by_key(|t| t.order);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Playlist {
    pub id: Option<i64>,
    pub uid: String,
    pub title: String,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub tracks: Option<String>,
    pub folder: Option<String>,
    pub artwork_blob: Option<Vec<u8>>,
    pub artwork_thumb: Option<String>,
    pub artwork_path: Option<String>,
    pub version: i64,
    pub versions_data: Option<String>,
    pub link_url: Option<String>,
    pub emulate_type: Option<String>,
    pub emulate_settings: Option<String>,
    pub pending_tracks: Option<String>,
    pub share_settings: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaylistUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub tracks: Option<String>,
    pub folder: Option<String>,
    pub artwork_blob: Option<Vec<u8>>,
    pub artwork_path: Option<String>,
    pub version: Option<i64>,
    pub versions_data: Option<String>,
    pub link_url: Option<String>,
    pub emulate_type: Option<String>,
    pub emulate_settings: Option<String>,
    pub pending_tracks: Option<String>,
    pub share_settings: Option<String>,
}

pub fn create_playlist(conn: &Connection, playlist: &Playlist) -> Result<()> {
    conn.execute(
		"INSERT INTO playlists (uid, title, description, owner, tracks, folder, artwork_blob, artwork_thumb, artwork_path, version, versions_data, link_url, emulate_type, emulate_settings, pending_tracks, share_settings)
		 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
		params![
			playlist.uid,
			playlist.title,
			playlist.description,
			playlist.owner,
			playlist.tracks,
			playlist.folder,
			playlist.artwork_blob,
			playlist.artwork_thumb,
			playlist.artwork_path,
			playlist.version,
			playlist.versions_data,
			playlist.link_url,
			playlist.emulate_type,
			playlist.emulate_settings,
			playlist.pending_tracks,
			playlist.share_settings
		],
	)?;
    Ok(())
}

pub fn get_all_playlists(conn: &Connection) -> Result<Vec<Playlist>> {
    let mut stmt = conn.prepare(
		"SELECT id, uid, title, description, owner, tracks, folder, artwork_path, artwork_thumb, version, versions_data, link_url, emulate_type, emulate_settings, pending_tracks, share_settings
		 FROM playlists ORDER BY folder, title",
	)?;
    let playlists = stmt
        .query_map([], |row| {
            Ok(Playlist {
                id: row.get(0)?,
                uid: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                owner: row.get(4)?,
                tracks: row.get(5)?,
                folder: row.get(6)?,
                artwork_blob: None,
                artwork_path: row.get(7)?,
                artwork_thumb: row.get(8)?,
                version: row.get(9)?,
                versions_data: row.get(10)?,
                link_url: row.get(11)?,
                emulate_type: row.get(12)?,
                emulate_settings: row.get(13)?,
                pending_tracks: row.get(14)?,
                share_settings: row.get(15)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(playlists)
}

pub fn get_playlist_by_id(conn: &Connection, id: i64) -> Result<Option<Playlist>> {
    let mut stmt = conn.prepare(
		"SELECT id, uid, title, description, owner, tracks, folder, artwork_blob, artwork_path, artwork_thumb, version, versions_data, link_url, emulate_type, emulate_settings, pending_tracks, share_settings
		 FROM playlists WHERE id = ?1",
	)?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Playlist {
            id: row.get(0)?,
            uid: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            owner: row.get(4)?,
            tracks: row.get(5)?,
            folder: row.get(6)?,
            artwork_blob: row.get(7)?,
            artwork_path: row.get(8)?,
            artwork_thumb: row.get(9)?,
            version: row.get(10)?,
            versions_data: row.get(11)?,
            link_url: row.get(12)?,
            emulate_type: row.get(13)?,
            emulate_settings: row.get(14)?,
            pending_tracks: row.get(15)?,
            share_settings: row.get(16)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_playlist_by_uid(conn: &Connection, uid: &str) -> Result<Option<Playlist>> {
    let mut stmt = conn.prepare(
		"SELECT id, uid, title, description, owner, tracks, folder, artwork_blob, artwork_path, artwork_thumb, version, versions_data, link_url, emulate_type, emulate_settings, pending_tracks, share_settings
		 FROM playlists WHERE uid = ?1",
	)?;
    let mut rows = stmt.query(params![uid])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Playlist {
            id: row.get(0)?,
            uid: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            owner: row.get(4)?,
            tracks: row.get(5)?,
            folder: row.get(6)?,
            artwork_blob: row.get(7)?,
            artwork_path: row.get(8)?,
            artwork_thumb: row.get(9)?,
            version: row.get(10)?,
            versions_data: row.get(11)?,
            link_url: row.get(12)?,
            emulate_type: row.get(13)?,
            emulate_settings: row.get(14)?,
            pending_tracks: row.get(15)?,
            share_settings: row.get(16)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_playlist_artwork(conn: &Connection, id: i64) -> Result<Option<Vec<u8>>> {
    let mut stmt = conn.prepare("SELECT artwork_blob FROM playlists WHERE id = ?1")?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(None)
    }
}

pub fn get_playlist_artwork_by_uid(conn: &Connection, uid: &str) -> Result<Option<Vec<u8>>> {
    let mut stmt = conn.prepare("SELECT artwork_blob FROM playlists WHERE uid = ?1")?;
    let mut rows = stmt.query(params![uid])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(None)
    }
}

pub fn update_playlist(conn: &Connection, id: i64, update: &PlaylistUpdate) -> Result<()> {
    if let Some(ref title) = update.title {
        conn.execute(
            "UPDATE playlists SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;
    }
    if let Some(ref description) = update.description {
        conn.execute(
            "UPDATE playlists SET description = ?1 WHERE id = ?2",
            params![description, id],
        )?;
    }
    if let Some(ref owner) = update.owner {
        conn.execute(
            "UPDATE playlists SET owner = ?1 WHERE id = ?2",
            params![owner, id],
        )?;
    }
    if let Some(ref tracks) = update.tracks {
        conn.execute(
            "UPDATE playlists SET tracks = ?1 WHERE id = ?2",
            params![tracks, id],
        )?;
    }
    if let Some(ref folder) = update.folder {
        conn.execute(
            "UPDATE playlists SET folder = ?1 WHERE id = ?2",
            params![folder, id],
        )?;
    }
    if let Some(ref blob) = update.artwork_blob {
        conn.execute(
            "UPDATE playlists SET artwork_blob = ?1 WHERE id = ?2",
            params![blob, id],
        )?;
        let thumb = crate::thumb::make_thumb(blob);
        conn.execute(
            "UPDATE playlists SET artwork_thumb = ?1 WHERE id = ?2",
            params![thumb, id],
        )?;
    }
    if let Some(ref path) = update.artwork_path {
        conn.execute(
            "UPDATE playlists SET artwork_path = ?1 WHERE id = ?2",
            params![path, id],
        )?;
    }
    if let Some(version) = update.version {
        conn.execute(
            "UPDATE playlists SET version = ?1 WHERE id = ?2",
            params![version, id],
        )?;
    }
    if let Some(ref versions_data) = update.versions_data {
        conn.execute(
            "UPDATE playlists SET versions_data = ?1 WHERE id = ?2",
            params![versions_data, id],
        )?;
    }
    if let Some(ref link_url) = update.link_url {
        conn.execute(
            "UPDATE playlists SET link_url = ?1 WHERE id = ?2",
            params![link_url, id],
        )?;
    }
    if let Some(ref emulate_type) = update.emulate_type {
        conn.execute(
            "UPDATE playlists SET emulate_type = ?1 WHERE id = ?2",
            params![emulate_type, id],
        )?;
    }
    if let Some(ref emulate_settings) = update.emulate_settings {
        conn.execute(
            "UPDATE playlists SET emulate_settings = ?1 WHERE id = ?2",
            params![emulate_settings, id],
        )?;
    }
    if let Some(ref pending_tracks) = update.pending_tracks {
        conn.execute(
            "UPDATE playlists SET pending_tracks = ?1 WHERE id = ?2",
            params![pending_tracks, id],
        )?;
    }
    if let Some(ref share_settings) = update.share_settings {
        conn.execute(
            "UPDATE playlists SET share_settings = ?1 WHERE id = ?2",
            params![share_settings, id],
        )?;
    }
    Ok(())
}

pub fn update_playlist_by_uid(conn: &Connection, uid: &str, update: &PlaylistUpdate) -> Result<()> {
    if let Some(ref title) = update.title {
        conn.execute(
            "UPDATE playlists SET title = ?1 WHERE uid = ?2",
            params![title, uid],
        )?;
    }
    if let Some(ref description) = update.description {
        conn.execute(
            "UPDATE playlists SET description = ?1 WHERE uid = ?2",
            params![description, uid],
        )?;
    }
    if let Some(ref owner) = update.owner {
        conn.execute(
            "UPDATE playlists SET owner = ?1 WHERE uid = ?2",
            params![owner, uid],
        )?;
    }
    if let Some(ref tracks) = update.tracks {
        conn.execute(
            "UPDATE playlists SET tracks = ?1 WHERE uid = ?2",
            params![tracks, uid],
        )?;
    }
    if let Some(ref folder) = update.folder {
        conn.execute(
            "UPDATE playlists SET folder = ?1 WHERE uid = ?2",
            params![folder, uid],
        )?;
    }
    if let Some(ref blob) = update.artwork_blob {
        conn.execute(
            "UPDATE playlists SET artwork_blob = ?1 WHERE uid = ?2",
            params![blob, uid],
        )?;
        let thumb = crate::thumb::make_thumb(blob);
        conn.execute(
            "UPDATE playlists SET artwork_thumb = ?1 WHERE uid = ?2",
            params![thumb, uid],
        )?;
    }
    if let Some(ref path) = update.artwork_path {
        conn.execute(
            "UPDATE playlists SET artwork_path = ?1 WHERE uid = ?2",
            params![path, uid],
        )?;
    }
    if let Some(version) = update.version {
        conn.execute(
            "UPDATE playlists SET version = ?1 WHERE uid = ?2",
            params![version, uid],
        )?;
    }
    if let Some(ref versions_data) = update.versions_data {
        conn.execute(
            "UPDATE playlists SET versions_data = ?1 WHERE uid = ?2",
            params![versions_data, uid],
        )?;
    }
    if let Some(ref link_url) = update.link_url {
        conn.execute(
            "UPDATE playlists SET link_url = ?1 WHERE uid = ?2",
            params![link_url, uid],
        )?;
    }
    if let Some(ref emulate_type) = update.emulate_type {
        conn.execute(
            "UPDATE playlists SET emulate_type = ?1 WHERE uid = ?2",
            params![emulate_type, uid],
        )?;
    }
    if let Some(ref emulate_settings) = update.emulate_settings {
        conn.execute(
            "UPDATE playlists SET emulate_settings = ?1 WHERE uid = ?2",
            params![emulate_settings, uid],
        )?;
    }
    if let Some(ref pending_tracks) = update.pending_tracks {
        conn.execute(
            "UPDATE playlists SET pending_tracks = ?1 WHERE uid = ?2",
            params![pending_tracks, uid],
        )?;
    }
    if let Some(ref share_settings) = update.share_settings {
        conn.execute(
            "UPDATE playlists SET share_settings = ?1 WHERE uid = ?2",
            params![share_settings, uid],
        )?;
    }
    Ok(())
}

pub fn rename_folder(conn: &Connection, old_path: &str, new_path: &str) -> Result<()> {
    let like_prefix = format!("{}/%", old_path);
    conn.execute(
        "UPDATE playlists SET folder = ?1 WHERE folder = ?2",
        params![new_path, old_path],
    )?;
    conn.execute(
        "UPDATE playlists SET folder = ?1 || SUBSTR(folder, ?2) WHERE folder LIKE ?3",
        params![new_path, old_path.len() as i64 + 1, like_prefix],
    )?;
    Ok(())
}

pub fn move_playlists_to_folder(
    conn: &Connection,
    old_folder: &str,
    new_folder: &str,
) -> Result<()> {
    let like_prefix = format!("{}/%", old_folder);
    let new_folder_val: Option<&str> = if new_folder.is_empty() {
        None
    } else {
        Some(new_folder)
    };
    conn.execute(
        "UPDATE playlists SET folder = ?1 WHERE folder = ?2",
        params![new_folder_val, old_folder],
    )?;
    conn.execute(
        "UPDATE playlists SET folder = ?1 || SUBSTR(folder, ?2) WHERE folder LIKE ?3",
        params![new_folder, old_folder.len() as i64 + 1, like_prefix],
    )?;
    Ok(())
}

pub fn delete_playlist(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM playlists WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn delete_playlist_by_uid(conn: &Connection, uid: &str) -> Result<()> {
    conn.execute("DELETE FROM playlists WHERE uid = ?1", params![uid])?;
    Ok(())
}
