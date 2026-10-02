use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Album {
    pub id: Option<i64>,
    pub uid: String,
    pub format: Option<String>,
    pub title: String,
    pub rating: Option<f32>,
    pub artists: Option<String>,
    pub album_artist: Option<String>,
    pub release_date: Option<String>,
    pub tags: Option<String>,
    pub genres: Option<String>,
    pub tracks: Option<String>,
    pub credits: Option<String>,
    pub label: Option<String>,
    pub artwork_blob: Option<Vec<u8>>,
    pub artwork_thumb: Option<String>,
    pub artwork_path: Option<String>,
    pub emulate_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlbumUpdate {
    pub format: Option<String>,
    pub title: Option<String>,
    pub rating: Option<f32>,
    pub artists: Option<String>,
    pub album_artist: Option<String>,
    pub release_date: Option<String>,
    pub tags: Option<String>,
    pub genres: Option<String>,
    pub tracks: Option<String>,
    pub credits: Option<String>,
    pub label: Option<String>,
    pub artwork_blob: Option<Vec<u8>>,
    pub artwork_path: Option<String>,
    pub emulate_type: Option<String>,
}

pub fn create_album(conn: &Connection, album: &Album) -> Result<()> {
    conn.execute(
		"INSERT INTO albums (uid, format, title, rating, artists, album_artist, release_date, tags, genres, tracks, credits, label, artwork_blob, artwork_thumb, artwork_path, emulate_type)
		 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
		params![
			album.uid, album.format, album.title, album.rating,
			album.artists, album.album_artist, album.release_date,
			album.tags, album.genres, album.tracks,
			album.credits, album.label,
			album.artwork_blob, album.artwork_thumb, album.artwork_path,
			album.emulate_type
		],
	)?;
    Ok(())
}

pub fn get_all_albums(conn: &Connection) -> Result<Vec<Album>> {
    let mut stmt = conn.prepare(
		"SELECT id, uid, format, title, rating, artists, album_artist, release_date, tags, genres, tracks, credits, label, artwork_path, artwork_thumb, emulate_type
		 FROM albums ORDER BY album_artist, title"
	)?;
    let albums = stmt
        .query_map([], |row| {
            Ok(Album {
                id: row.get(0)?,
                uid: row.get(1)?,
                format: row.get(2)?,
                title: row.get(3)?,
                rating: row.get(4)?,
                artists: row.get(5)?,
                album_artist: row.get(6)?,
                release_date: row.get(7)?,
                tags: row.get(8)?,
                genres: row.get(9)?,
                tracks: row.get(10)?,
                credits: row.get(11)?,
                label: row.get(12)?,
                artwork_blob: None,
                artwork_path: row.get(13)?,
                artwork_thumb: row.get(14)?,
                emulate_type: row.get(15)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(albums)
}

pub fn get_album_by_id(conn: &Connection, id: i64) -> Result<Option<Album>> {
    let mut stmt = conn.prepare(
		"SELECT id, uid, format, title, rating, artists, album_artist, release_date, tags, genres, tracks, credits, label, artwork_blob, artwork_path, artwork_thumb, emulate_type
		 FROM albums WHERE id = ?1"
	)?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Album {
            id: row.get(0)?,
            uid: row.get(1)?,
            format: row.get(2)?,
            title: row.get(3)?,
            rating: row.get(4)?,
            artists: row.get(5)?,
            album_artist: row.get(6)?,
            release_date: row.get(7)?,
            tags: row.get(8)?,
            genres: row.get(9)?,
            tracks: row.get(10)?,
            credits: row.get(11)?,
            label: row.get(12)?,
            artwork_blob: row.get(13)?,
            artwork_path: row.get(14)?,
            artwork_thumb: row.get(15)?,
            emulate_type: row.get(16)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_album_by_uid(conn: &Connection, uid: &str) -> Result<Option<Album>> {
    let mut stmt = conn.prepare(
		"SELECT id, uid, format, title, rating, artists, album_artist, release_date, tags, genres, tracks, credits, label, artwork_blob, artwork_path, artwork_thumb, emulate_type
		 FROM albums WHERE uid = ?1"
	)?;
    let mut rows = stmt.query(params![uid])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Album {
            id: row.get(0)?,
            uid: row.get(1)?,
            format: row.get(2)?,
            title: row.get(3)?,
            rating: row.get(4)?,
            artists: row.get(5)?,
            album_artist: row.get(6)?,
            release_date: row.get(7)?,
            tags: row.get(8)?,
            genres: row.get(9)?,
            tracks: row.get(10)?,
            credits: row.get(11)?,
            label: row.get(12)?,
            artwork_blob: row.get(13)?,
            artwork_path: row.get(14)?,
            artwork_thumb: row.get(15)?,
            emulate_type: row.get(16)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_album_artwork(conn: &Connection, id: i64) -> Result<Option<Vec<u8>>> {
    let mut stmt = conn.prepare("SELECT artwork_blob FROM albums WHERE id = ?1")?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(None)
    }
}

pub fn get_album_artwork_by_uid(conn: &Connection, uid: &str) -> Result<Option<Vec<u8>>> {
    let mut stmt = conn.prepare("SELECT artwork_blob FROM albums WHERE uid = ?1")?;
    let mut rows = stmt.query(params![uid])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(None)
    }
}

pub fn update_album(conn: &Connection, id: i64, update: &AlbumUpdate) -> Result<()> {
    if let Some(ref format) = update.format {
        conn.execute(
            "UPDATE albums SET format = ?1 WHERE id = ?2",
            params![format, id],
        )?;
    }
    if let Some(ref title) = update.title {
        conn.execute(
            "UPDATE albums SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;
    }
    if let Some(rating) = update.rating {
        conn.execute(
            "UPDATE albums SET rating = ?1 WHERE id = ?2",
            params![rating, id],
        )?;
    }
    if let Some(ref artists) = update.artists {
        conn.execute(
            "UPDATE albums SET artists = ?1 WHERE id = ?2",
            params![artists, id],
        )?;
    }
    if let Some(ref album_artist) = update.album_artist {
        conn.execute(
            "UPDATE albums SET album_artist = ?1 WHERE id = ?2",
            params![album_artist, id],
        )?;
    }
    if let Some(ref release_date) = update.release_date {
        conn.execute(
            "UPDATE albums SET release_date = ?1 WHERE id = ?2",
            params![release_date, id],
        )?;
    }
    if let Some(ref tags) = update.tags {
        conn.execute(
            "UPDATE albums SET tags = ?1 WHERE id = ?2",
            params![tags, id],
        )?;
    }
    if let Some(ref genres) = update.genres {
        conn.execute(
            "UPDATE albums SET genres = ?1 WHERE id = ?2",
            params![genres, id],
        )?;
    }
    if let Some(ref tracks) = update.tracks {
        conn.execute(
            "UPDATE albums SET tracks = ?1 WHERE id = ?2",
            params![tracks, id],
        )?;
    }
    if let Some(ref credits) = update.credits {
        conn.execute(
            "UPDATE albums SET credits = ?1 WHERE id = ?2",
            params![credits, id],
        )?;
    }
    if let Some(ref label) = update.label {
        conn.execute(
            "UPDATE albums SET label = ?1 WHERE id = ?2",
            params![label, id],
        )?;
    }
    if let Some(ref blob) = update.artwork_blob {
        conn.execute(
            "UPDATE albums SET artwork_blob = ?1 WHERE id = ?2",
            params![blob, id],
        )?;
        let thumb = crate::thumb::make_thumb(blob);
        conn.execute(
            "UPDATE albums SET artwork_thumb = ?1 WHERE id = ?2",
            params![thumb, id],
        )?;
    }
    if let Some(ref path) = update.artwork_path {
        conn.execute(
            "UPDATE albums SET artwork_path = ?1 WHERE id = ?2",
            params![path, id],
        )?;
    }
    if let Some(ref emulate_type) = update.emulate_type {
        conn.execute(
            "UPDATE albums SET emulate_type = ?1 WHERE id = ?2",
            params![emulate_type, id],
        )?;
    }
    Ok(())
}

pub fn update_album_by_uid(conn: &Connection, uid: &str, update: &AlbumUpdate) -> Result<()> {
    if let Some(ref format) = update.format {
        conn.execute(
            "UPDATE albums SET format = ?1 WHERE uid = ?2",
            params![format, uid],
        )?;
    }
    if let Some(ref title) = update.title {
        conn.execute(
            "UPDATE albums SET title = ?1 WHERE uid = ?2",
            params![title, uid],
        )?;
    }
    if let Some(rating) = update.rating {
        conn.execute(
            "UPDATE albums SET rating = ?1 WHERE uid = ?2",
            params![rating, uid],
        )?;
    }
    if let Some(ref artists) = update.artists {
        conn.execute(
            "UPDATE albums SET artists = ?1 WHERE uid = ?2",
            params![artists, uid],
        )?;
    }
    if let Some(ref album_artist) = update.album_artist {
        conn.execute(
            "UPDATE albums SET album_artist = ?1 WHERE uid = ?2",
            params![album_artist, uid],
        )?;
    }
    if let Some(ref release_date) = update.release_date {
        conn.execute(
            "UPDATE albums SET release_date = ?1 WHERE uid = ?2",
            params![release_date, uid],
        )?;
    }
    if let Some(ref tags) = update.tags {
        conn.execute(
            "UPDATE albums SET tags = ?1 WHERE uid = ?2",
            params![tags, uid],
        )?;
    }
    if let Some(ref genres) = update.genres {
        conn.execute(
            "UPDATE albums SET genres = ?1 WHERE uid = ?2",
            params![genres, uid],
        )?;
    }
    if let Some(ref tracks) = update.tracks {
        conn.execute(
            "UPDATE albums SET tracks = ?1 WHERE uid = ?2",
            params![tracks, uid],
        )?;
    }
    if let Some(ref credits) = update.credits {
        conn.execute(
            "UPDATE albums SET credits = ?1 WHERE uid = ?2",
            params![credits, uid],
        )?;
    }
    if let Some(ref label) = update.label {
        conn.execute(
            "UPDATE albums SET label = ?1 WHERE uid = ?2",
            params![label, uid],
        )?;
    }
    if let Some(ref blob) = update.artwork_blob {
        conn.execute(
            "UPDATE albums SET artwork_blob = ?1 WHERE uid = ?2",
            params![blob, uid],
        )?;
        let thumb = crate::thumb::make_thumb(blob);
        conn.execute(
            "UPDATE albums SET artwork_thumb = ?1 WHERE uid = ?2",
            params![thumb, uid],
        )?;
    }
    if let Some(ref path) = update.artwork_path {
        conn.execute(
            "UPDATE albums SET artwork_path = ?1 WHERE uid = ?2",
            params![path, uid],
        )?;
    }
    if let Some(ref emulate_type) = update.emulate_type {
        conn.execute(
            "UPDATE albums SET emulate_type = ?1 WHERE uid = ?2",
            params![emulate_type, uid],
        )?;
    }
    Ok(())
}

pub fn delete_album(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM albums WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn delete_album_by_uid(conn: &Connection, uid: &str) -> Result<()> {
    conn.execute("DELETE FROM albums WHERE uid = ?1", params![uid])?;
    Ok(())
}
