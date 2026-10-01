use rusqlite::OptionalExtension;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Artist {
    pub id: Option<i64>,
    pub uid: String,
    pub name: String,
    pub aka: Option<String>,
    pub about: Option<String>,
    pub tags: Option<String>,
    pub genres: Option<String>,
    pub websites: Option<String>,
    pub members: Option<String>,
    pub profile_art_blob: Option<Vec<u8>>,
    pub profile_art_thumb: Option<String>,
    pub profile_art_path: Option<String>,
    pub banner_art_blob: Option<Vec<u8>>,
    pub banner_art_thumb: Option<String>,
    pub banner_art_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArtistUpdate {
    pub name: Option<String>,
    pub aka: Option<String>,
    pub about: Option<String>,
    pub tags: Option<String>,
    pub genres: Option<String>,
    pub websites: Option<String>,
    pub members: Option<String>,
    pub profile_art_blob: Option<Vec<u8>>,
    pub profile_art_path: Option<String>,
    pub banner_art_blob: Option<Vec<u8>>,
    pub banner_art_path: Option<String>,
}

pub fn create_artist(conn: &Connection, artist: &Artist) -> Result<()> {
    conn.execute(
		"INSERT INTO artists (uid, name, aka, about, tags, genres, websites, members, profile_art_blob, profile_art_thumb, profile_art_path, banner_art_blob, banner_art_path)
		 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
		params![
			artist.uid, artist.name, artist.aka, artist.about,
			artist.tags, artist.genres, artist.websites, artist.members,
			artist.profile_art_blob, artist.profile_art_thumb, artist.profile_art_path,
			artist.banner_art_blob, artist.banner_art_path
		],
	)?;
    Ok(())
}

pub fn get_all_artists(conn: &Connection) -> Result<Vec<Artist>> {
    let mut stmt = conn.prepare(
		"SELECT id, uid, name, aka, about, tags, genres, websites, members, profile_art_path, profile_art_thumb, banner_art_path
		 FROM artists ORDER BY name"
	)?;
    let artists = stmt
        .query_map([], |row| {
            Ok(Artist {
                id: row.get(0)?,
                uid: row.get(1)?,
                name: row.get(2)?,
                aka: row.get(3)?,
                about: row.get(4)?,
                tags: row.get(5)?,
                genres: row.get(6)?,
                websites: row.get(7)?,
                members: row.get(8)?,
                profile_art_blob: None,
                profile_art_path: row.get(9)?,
                profile_art_thumb: row.get(10)?,
                banner_art_blob: None,
                banner_art_thumb: None,
                banner_art_path: row.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(artists)
}

pub fn get_artist_by_id(conn: &Connection, id: i64) -> Result<Option<Artist>> {
    let mut stmt = conn.prepare(
		"SELECT id, uid, name, aka, about, tags, genres, websites, members, profile_art_blob, profile_art_thumb, profile_art_path, banner_art_blob, banner_art_path
		 FROM artists WHERE id = ?1"
	)?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Artist {
            id: row.get(0)?,
            uid: row.get(1)?,
            name: row.get(2)?,
            aka: row.get(3)?,
            about: row.get(4)?,
            tags: row.get(5)?,
            genres: row.get(6)?,
            websites: row.get(7)?,
            members: row.get(8)?,
            profile_art_blob: row.get(9)?,
            profile_art_thumb: row.get(10)?,
            profile_art_path: row.get(11)?,
            banner_art_blob: row.get(12)?,
            banner_art_thumb: None,
            banner_art_path: row.get(13)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_artist_by_uid(conn: &Connection, uid: &str) -> Result<Option<Artist>> {
    let mut stmt = conn.prepare(
		"SELECT id, uid, name, aka, about, tags, genres, websites, members, profile_art_blob, profile_art_thumb, profile_art_path, banner_art_blob, banner_art_path
		 FROM artists WHERE uid = ?1"
	)?;
    let mut rows = stmt.query(params![uid])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Artist {
            id: row.get(0)?,
            uid: row.get(1)?,
            name: row.get(2)?,
            aka: row.get(3)?,
            about: row.get(4)?,
            tags: row.get(5)?,
            genres: row.get(6)?,
            websites: row.get(7)?,
            members: row.get(8)?,
            profile_art_blob: row.get(9)?,
            profile_art_thumb: row.get(10)?,
            profile_art_path: row.get(11)?,
            banner_art_blob: row.get(12)?,
            banner_art_thumb: None,
            banner_art_path: row.get(13)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_artist_uid_by_name(conn: &Connection, name: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT uid FROM artists WHERE LOWER(name) = LOWER(?1) LIMIT 1",
        params![name],
        |row| row.get(0),
    )
    .optional()
}

pub fn get_artist_profile_art(conn: &Connection, id: i64) -> Result<Option<Vec<u8>>> {
    let mut stmt = conn.prepare("SELECT profile_art_blob FROM artists WHERE id = ?1")?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(None)
    }
}

pub fn get_artist_profile_art_by_uid(conn: &Connection, uid: &str) -> Result<Option<Vec<u8>>> {
    let mut stmt = conn.prepare("SELECT profile_art_blob FROM artists WHERE uid = ?1")?;
    let mut rows = stmt.query(params![uid])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(None)
    }
}

pub fn get_artist_banner_art(conn: &Connection, id: i64) -> Result<Option<Vec<u8>>> {
    let mut stmt = conn.prepare("SELECT banner_art_blob FROM artists WHERE id = ?1")?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(None)
    }
}

pub fn get_artist_banner_art_by_uid(conn: &Connection, uid: &str) -> Result<Option<Vec<u8>>> {
    let mut stmt = conn.prepare("SELECT banner_art_blob FROM artists WHERE uid = ?1")?;
    let mut rows = stmt.query(params![uid])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(None)
    }
}

pub fn update_artist(conn: &Connection, id: i64, update: &ArtistUpdate) -> Result<()> {
    if let Some(ref name) = update.name {
        conn.execute(
            "UPDATE artists SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
    }
    if let Some(ref aka) = update.aka {
        conn.execute(
            "UPDATE artists SET aka = ?1 WHERE id = ?2",
            params![aka, id],
        )?;
    }
    if let Some(ref about) = update.about {
        conn.execute(
            "UPDATE artists SET about = ?1 WHERE id = ?2",
            params![about, id],
        )?;
    }
    if let Some(ref tags) = update.tags {
        conn.execute(
            "UPDATE artists SET tags = ?1 WHERE id = ?2",
            params![tags, id],
        )?;
    }
    if let Some(ref genres) = update.genres {
        conn.execute(
            "UPDATE artists SET genres = ?1 WHERE id = ?2",
            params![genres, id],
        )?;
    }
    if let Some(ref websites) = update.websites {
        conn.execute(
            "UPDATE artists SET websites = ?1 WHERE id = ?2",
            params![websites, id],
        )?;
    }
    if let Some(ref members) = update.members {
        conn.execute(
            "UPDATE artists SET members = ?1 WHERE id = ?2",
            params![members, id],
        )?;
    }
    if let Some(ref blob) = update.profile_art_blob {
        conn.execute(
            "UPDATE artists SET profile_art_blob = ?1 WHERE id = ?2",
            params![blob, id],
        )?;
        let thumb = crate::thumb::make_thumb(blob);
        conn.execute(
            "UPDATE artists SET profile_art_thumb = ?1 WHERE id = ?2",
            params![thumb, id],
        )?;
    }
    if let Some(ref path) = update.profile_art_path {
        conn.execute(
            "UPDATE artists SET profile_art_path = ?1 WHERE id = ?2",
            params![path, id],
        )?;
    }
    if let Some(ref blob) = update.banner_art_blob {
        conn.execute(
            "UPDATE artists SET banner_art_blob = ?1 WHERE id = ?2",
            params![blob, id],
        )?;
    }
    if let Some(ref path) = update.banner_art_path {
        conn.execute(
            "UPDATE artists SET banner_art_path = ?1 WHERE id = ?2",
            params![path, id],
        )?;
    }
    Ok(())
}

pub fn update_artist_by_uid(conn: &Connection, uid: &str, update: &ArtistUpdate) -> Result<()> {
    if let Some(ref name) = update.name {
        conn.execute(
            "UPDATE artists SET name = ?1 WHERE uid = ?2",
            params![name, uid],
        )?;
    }
    if let Some(ref aka) = update.aka {
        conn.execute(
            "UPDATE artists SET aka = ?1 WHERE uid = ?2",
            params![aka, uid],
        )?;
    }
    if let Some(ref about) = update.about {
        conn.execute(
            "UPDATE artists SET about = ?1 WHERE uid = ?2",
            params![about, uid],
        )?;
    }
    if let Some(ref tags) = update.tags {
        conn.execute(
            "UPDATE artists SET tags = ?1 WHERE uid = ?2",
            params![tags, uid],
        )?;
    }
    if let Some(ref genres) = update.genres {
        conn.execute(
            "UPDATE artists SET genres = ?1 WHERE uid = ?2",
            params![genres, uid],
        )?;
    }
    if let Some(ref websites) = update.websites {
        conn.execute(
            "UPDATE artists SET websites = ?1 WHERE uid = ?2",
            params![websites, uid],
        )?;
    }
    if let Some(ref members) = update.members {
        conn.execute(
            "UPDATE artists SET members = ?1 WHERE uid = ?2",
            params![members, uid],
        )?;
    }
    if let Some(ref blob) = update.profile_art_blob {
        conn.execute(
            "UPDATE artists SET profile_art_blob = ?1 WHERE uid = ?2",
            params![blob, uid],
        )?;
        let thumb = crate::thumb::make_thumb(blob);
        conn.execute(
            "UPDATE artists SET profile_art_thumb = ?1 WHERE uid = ?2",
            params![thumb, uid],
        )?;
    }
    if let Some(ref path) = update.profile_art_path {
        conn.execute(
            "UPDATE artists SET profile_art_path = ?1 WHERE uid = ?2",
            params![path, uid],
        )?;
    }
    if let Some(ref blob) = update.banner_art_blob {
        conn.execute(
            "UPDATE artists SET banner_art_blob = ?1 WHERE uid = ?2",
            params![blob, uid],
        )?;
    }
    if let Some(ref path) = update.banner_art_path {
        conn.execute(
            "UPDATE artists SET banner_art_path = ?1 WHERE uid = ?2",
            params![path, uid],
        )?;
    }
    Ok(())
}

pub fn delete_artist(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM artists WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn delete_artist_by_uid(conn: &Connection, uid: &str) -> Result<()> {
    conn.execute("DELETE FROM artists WHERE uid = ?1", params![uid])?;
    Ok(())
}
