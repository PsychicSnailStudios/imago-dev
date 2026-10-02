use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Track {
    pub id: Option<i64>,
    pub uid: String,
    pub path: String,
    pub last_modified: i64,
    pub title: Option<String>,
    pub artists: Option<String>,
    pub album_artist: Option<String>,
    pub albums: Option<String>,
    pub genres: Option<String>,
    pub year: Option<String>,
    pub rating: Option<f32>,
    pub tags: Option<String>,
    pub duration_ms: Option<i64>,
    pub bpm: Option<f32>,
    pub key: Option<String>,
    pub credits: Option<String>,
    pub label: Option<String>,
    pub artwork_blob: Option<Vec<u8>>,
    pub artwork_thumb: Option<String>,
    pub artwork_path: Option<String>,
    pub user_options: Option<String>,
    pub format: Option<String>,
    pub bitrate: Option<i64>,
    pub remote_path: Option<String>,
    pub remote_data: Option<String>,
    pub track_data: Option<String>,
    pub date_added: Option<i64>,
}

// ─── JSON field helpers ─────────────────────────────────────────────────────
//
// artists / album_artist / albums / genres / tags are plain JSON text columns.
// The backend never deserialises them into typed structs (that's what caused
// the earlier slowdown) — it only reaches in when it genuinely needs a single
// value out of them. Everything else is passed straight through to the
// frontend, which owns parsing/serialising via parsers.ts.

pub fn first_artist_name(artists_json: &Option<String>) -> Option<String> {
    artists_json
        .as_deref()
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .and_then(|v| v.into_iter().next())
}

pub fn resolved_artist_name(
    album_artist: &Option<String>,
    artists_json: &Option<String>,
) -> Option<String> {
    album_artist
        .clone()
        .or_else(|| first_artist_name(artists_json))
}

pub fn first_album_name(albums_json: &Option<String>) -> Option<String> {
    albums_json
        .as_deref()
        .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
        .and_then(|v| v.into_iter().next())
        .and_then(|e| {
            e.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
}

pub fn first_track_number(albums_json: &Option<String>) -> Option<i64> {
    albums_json
        .as_deref()
        .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
        .and_then(|v| v.into_iter().next())
        .and_then(|e| e.get("track_number").and_then(|n| n.as_i64()))
}

pub fn first_album_uid(albums_json: &Option<String>) -> Option<String> {
    albums_json
        .as_deref()
        .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
        .and_then(|v| v.into_iter().next())
        .and_then(|e| e.get("uid").and_then(|n| n.as_str()).map(|s| s.to_string()))
        .filter(|u| !u.is_empty())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DuplicateGroup {
    pub tracks: Vec<Track>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MetadataUpdate {
    pub title: Option<String>,
    pub artists: Option<String>,
    pub album_artist: Option<String>,
    pub albums: Option<String>,
    pub year: Option<String>,
    pub genres: Option<String>,
    pub bpm: Option<f32>,
    pub rating: Option<f32>,
    pub tags: Option<String>,
    pub key: Option<String>,
    pub credits: Option<String>,
    pub label: Option<String>,
    #[serde(skip)]
    pub artwork_blob: Option<Vec<u8>>,
    pub artwork_path: Option<String>,
    pub user_options: Option<String>,
    pub format: Option<String>,
    pub bitrate: Option<i64>,
    pub remote_path: Option<String>,
    pub remote_data: Option<String>,
    pub track_data: Option<String>,
}

pub fn upsert_track(conn: &Connection, track: &Track) -> Result<()> {
    let track_uid = if let Some(ref rp) = track.remote_path {
        if !rp.is_empty() {
            let existing_uid: Option<String> = conn
                .query_row(
                    "SELECT uid FROM tracks WHERE remote_path = ?1",
                    params![rp],
                    |row| row.get(0),
                )
                .ok();
            existing_uid.unwrap_or_else(|| track.uid.clone())
        } else {
            track.uid.clone()
        }
    } else {
        track.uid.clone()
    };

    let track_uid = if !track.path.is_empty() {
        let existing_uid: Option<String> = conn
            .query_row(
                "SELECT uid FROM tracks WHERE path = ?1",
                params![&track.path],
                |row| row.get(0),
            )
            .ok();
        existing_uid.unwrap_or(track_uid)
    } else {
        track_uid
    };

    let uid_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM tracks WHERE uid = ?1",
            params![&track_uid],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    if uid_exists {
        conn.execute(
            "UPDATE tracks SET
				path          = ?2,
				last_modified = ?3,
				title         = ?4,
				artists       = ?5,
				album_artist  = ?6,
				albums        = ?7,
				genres        = ?8,
				year          = ?9,
				rating        = ?10,
				duration_ms   = ?11,
				bpm           = ?12,
				key           = ?13,
				credits       = ?14,
				label         = ?15,
				artwork_blob  = COALESCE(?16, artwork_blob),
				artwork_thumb = COALESCE(?17, artwork_thumb),
				artwork_path  = ?18,
				user_options  = ?19,
				format        = ?20,
				bitrate       = ?21,
				remote_path   = ?22,
				remote_data   = ?23,
				track_data    = ?24
			WHERE uid = ?1",
            params![
                track_uid,
                track.path,
                track.last_modified,
                track.title,
                track.artists,
                track.album_artist,
                track.albums,
                track.genres,
                track.year,
                track.rating,
                track.duration_ms,
                track.bpm,
                track.key,
                track.credits,
                track.label,
                track.artwork_blob,
                track.artwork_thumb,
                track.artwork_path,
                track.user_options,
                track.format,
                track.bitrate,
                track.remote_path,
                track.remote_data,
                track.track_data,
            ],
        )?;
    } else {
        conn.execute(
			"INSERT INTO tracks (uid, path, last_modified, title, artists, album_artist, albums, genres, year, rating, duration_ms, bpm, key, credits, label, artwork_blob, artwork_thumb, artwork_path, user_options, format, bitrate, remote_path, remote_data, track_data)
			 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)
			 ON CONFLICT(uid) DO UPDATE SET
				last_modified = excluded.last_modified,
				title         = excluded.title,
				artists       = excluded.artists,
				album_artist  = excluded.album_artist,
				albums        = excluded.albums,
				genres        = excluded.genres,
				year          = excluded.year,
				rating        = excluded.rating,
				duration_ms   = excluded.duration_ms,
				bpm           = excluded.bpm,
				key           = excluded.key,
				credits       = excluded.credits,
				label         = excluded.label,
				artwork_blob  = excluded.artwork_blob,
				artwork_thumb = excluded.artwork_thumb,
				artwork_path  = excluded.artwork_path,
				format        = excluded.format,
				bitrate       = excluded.bitrate,
				remote_path   = excluded.remote_path,
				remote_data   = excluded.remote_data,
				track_data    = excluded.track_data",
			params![
				track_uid,
				track.path,
				track.last_modified,
				track.title,
				track.artists,
				track.album_artist,
				track.albums,
				track.genres,
				track.year,
				track.rating,
				track.duration_ms,
				track.bpm,
				track.key,
				track.credits,
				track.label,
				track.artwork_blob,
				track.artwork_thumb,
				track.artwork_path,
				track.user_options,
				track.format,
				track.bitrate,
				track.remote_path,
				track.remote_data,
				track.track_data,
			],
		)?;
    }

    Ok(())
}

pub fn delete_track(conn: &Connection, path: &str) -> Result<()> {
    conn.execute("DELETE FROM tracks WHERE path = ?1", params![path])?;
    Ok(())
}

pub fn delete_track_by_id(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM tracks WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn delete_track_by_uid(conn: &Connection, uid: &str) -> Result<()> {
    conn.execute("DELETE FROM tracks WHERE uid = ?1", params![uid])?;
    Ok(())
}

pub fn get_all_tracks(conn: &Connection) -> Result<Vec<Track>> {
    let mut stmt = conn.prepare(
		"SELECT id, uid, path, last_modified, title, artists, album_artist, albums, genres, year, rating, tags, duration_ms, bpm, key, credits, label, format, bitrate, artwork_path, artwork_thumb, user_options, remote_path, remote_data, track_data, date_added
		 FROM tracks ORDER BY album_artist, albums, title"
	)?;
    let tracks = stmt
        .query_map([], |row| {
            Ok(Track {
                id: row.get(0)?,
                uid: row.get(1)?,
                path: row.get(2)?,
                last_modified: row.get(3)?,
                title: row.get(4)?,
                artists: row.get(5)?,
                album_artist: row.get(6)?,
                albums: row.get(7)?,
                genres: row.get(8)?,
                year: row.get(9)?,
                rating: row.get(10)?,
                tags: row.get(11)?,
                duration_ms: row.get(12)?,
                bpm: row.get(13)?,
                key: row.get(14)?,
                credits: row.get(15)?,
                label: row.get(16)?,
                format: row.get(17)?,
                bitrate: row.get(18)?,
                artwork_blob: None,
                artwork_path: row.get(19)?,
                artwork_thumb: row.get(20)?,
                user_options: row.get(21)?,
                remote_path: row.get(22)?,
                remote_data: row.get(23)?,
                track_data: row.get(24)?,
                date_added: row.get(25)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(tracks)
}

pub fn get_track_by_uid(conn: &Connection, uid: &str) -> Result<Option<Track>> {
    let mut stmt = conn.prepare(
		"SELECT id, uid, path, last_modified, title, artists, album_artist, albums, genres, year, rating, tags, duration_ms, bpm, key, credits, label, format, bitrate, artwork_path, artwork_thumb, user_options, remote_path, remote_data, track_data, date_added
		 FROM tracks WHERE uid = ?1"
	)?;
    let mut rows = stmt.query(params![uid])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Track {
            id: row.get(0)?,
            uid: row.get(1)?,
            path: row.get(2)?,
            last_modified: row.get(3)?,
            title: row.get(4)?,
            artists: row.get(5)?,
            album_artist: row.get(6)?,
            albums: row.get(7)?,
            genres: row.get(8)?,
            year: row.get(9)?,
            rating: row.get(10)?,
            tags: row.get(11)?,
            duration_ms: row.get(12)?,
            bpm: row.get(13)?,
            key: row.get(14)?,
            credits: row.get(15)?,
            label: row.get(16)?,
            format: row.get(17)?,
            bitrate: row.get(18)?,
            artwork_blob: None,
            artwork_path: row.get(19)?,
            artwork_thumb: row.get(20)?,
            user_options: row.get(21)?,
            remote_path: row.get(22)?,
            remote_data: row.get(23)?,
            track_data: row.get(24)?,
            date_added: row.get(25)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_track_artwork(conn: &Connection, id: i64) -> Result<Option<Vec<u8>>> {
    let mut stmt = conn.prepare("SELECT artwork_blob FROM tracks WHERE id = ?1")?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(None)
    }
}

pub fn get_track_artwork_by_uid(conn: &Connection, uid: &str) -> Result<Option<Vec<u8>>> {
    let mut stmt = conn.prepare("SELECT artwork_blob FROM tracks WHERE uid = ?1")?;
    let mut rows = stmt.query(params![uid])?;
    if let Some(row) = rows.next()? {
        Ok(row.get(0)?)
    } else {
        Ok(None)
    }
}

pub fn find_duplicates(conn: &Connection) -> Result<Vec<DuplicateGroup>> {
    let mut stmt = conn.prepare(
		"SELECT id, uid, path, last_modified, title, artists, album_artist, albums, genres, year, rating, tags, duration_ms, bpm, key, credits, label, format, bitrate, artwork_path, artwork_thumb, user_options, remote_path, remote_data, track_data, date_added
		 FROM (
			 SELECT a.id, a.uid, a.path, a.last_modified, a.title, a.artists, a.album_artist, a.albums, a.genres, a.year, a.rating, a.tags, a.duration_ms, a.bpm, a.key, a.credits, a.label, a.format, a.bitrate, a.artwork_path, a.artwork_thumb, a.user_options, a.remote_path, a.remote_data, a.track_data, a.date_added
			 FROM tracks a
			 INNER JOIN tracks b ON (
				 a.id < b.id
				 AND LOWER(TRIM(a.title)) = LOWER(TRIM(b.title))
				 AND LOWER(TRIM(COALESCE(a.album_artist, JSON_EXTRACT(a.artists, '$[0]')))) = LOWER(TRIM(COALESCE(b.album_artist, JSON_EXTRACT(b.artists, '$[0]'))))
				 AND ABS(COALESCE(a.duration_ms, 0) - COALESCE(b.duration_ms, 0)) <= 1000
			 )
			 UNION
			 SELECT b.id, b.uid, b.path, b.last_modified, b.title, b.artists, b.album_artist, b.albums, b.genres, b.year, b.rating, b.tags, b.duration_ms, b.bpm, b.key, b.credits, b.label, b.format, b.bitrate, b.artwork_path, b.artwork_thumb, b.user_options, b.remote_path, b.remote_data, b.track_data, b.date_added
			 FROM tracks a
			 INNER JOIN tracks b ON (
				 a.id < b.id
				 AND LOWER(TRIM(a.title)) = LOWER(TRIM(b.title))
				 AND LOWER(TRIM(COALESCE(a.album_artist, JSON_EXTRACT(a.artists, '$[0]')))) = LOWER(TRIM(COALESCE(b.album_artist, JSON_EXTRACT(b.artists, '$[0]'))))
				 AND ABS(COALESCE(a.duration_ms, 0) - COALESCE(b.duration_ms, 0)) <= 1000
			 )
		 )
		 ORDER BY title, album_artist, duration_ms"
	)?;

    let all_tracks = stmt
        .query_map([], |row| {
            Ok(Track {
                id: row.get(0)?,
                uid: row.get(1)?,
                path: row.get(2)?,
                last_modified: row.get(3)?,
                title: row.get(4)?,
                artists: row.get(5)?,
                album_artist: row.get(6)?,
                albums: row.get(7)?,
                genres: row.get(8)?,
                year: row.get(9)?,
                rating: row.get(10)?,
                tags: row.get(11)?,
                duration_ms: row.get(12)?,
                bpm: row.get(13)?,
                key: row.get(14)?,
                credits: row.get(15)?,
                label: row.get(16)?,
                format: row.get(17)?,
                bitrate: row.get(18)?,
                artwork_blob: None,
                artwork_path: row.get(19)?,
                artwork_thumb: row.get(20)?,
                user_options: row.get(21)?,
                remote_path: row.get(22)?,
                remote_data: row.get(23)?,
                track_data: row.get(24)?,
                date_added: row.get(25)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    let mut groups: Vec<DuplicateGroup> = Vec::new();
    let mut i = 0;
    while i < all_tracks.len() {
        let mut group = vec![all_tracks[i].clone()];
        let mut j = i + 1;
        while j < all_tracks.len() {
            let a = &all_tracks[i];
            let b = &all_tracks[j];
            let same_title = a.title.as_deref().map(|s| s.to_lowercase())
                == b.title.as_deref().map(|s| s.to_lowercase());
            let same_artist = a.album_artist.as_ref().map(|s| s.to_lowercase())
                == b.album_artist.as_ref().map(|s| s.to_lowercase());
            let duration_close = match (a.duration_ms, b.duration_ms) {
                (Some(da), Some(db)) => (da - db).abs() <= 1000,
                _ => false,
            };
            if same_title && same_artist && duration_close {
                group.push(all_tracks[j].clone());
                j += 1;
            } else {
                break;
            }
        }
        groups.push(DuplicateGroup { tracks: group });
        i = j;
    }

    Ok(groups)
}

pub fn update_track_metadata(conn: &Connection, id: i64, update: &MetadataUpdate) -> Result<()> {
    if let Some(ref title) = update.title {
        conn.execute(
            "UPDATE tracks SET title = ?1 WHERE id = ?2",
            params![title, id],
        )?;
    }
    if let Some(ref artists) = update.artists {
        conn.execute(
            "UPDATE tracks SET artists = ?1 WHERE id = ?2",
            params![artists, id],
        )?;
    }
    if let Some(ref album_artist) = update.album_artist {
        conn.execute(
            "UPDATE tracks SET album_artist = ?1 WHERE id = ?2",
            params![album_artist, id],
        )?;
    }
    if let Some(ref albums) = update.albums {
        conn.execute(
            "UPDATE tracks SET albums = ?1 WHERE id = ?2",
            params![albums, id],
        )?;
    }
    if let Some(ref year) = update.year {
        conn.execute(
            "UPDATE tracks SET year = ?1 WHERE id = ?2",
            params![year, id],
        )?;
    }
    if let Some(ref genres) = update.genres {
        conn.execute(
            "UPDATE tracks SET genres = ?1 WHERE id = ?2",
            params![genres, id],
        )?;
    }
    if let Some(bpm) = update.bpm {
        conn.execute("UPDATE tracks SET bpm = ?1 WHERE id = ?2", params![bpm, id])?;
    }
    if let Some(rating) = update.rating {
        conn.execute(
            "UPDATE tracks SET rating = ?1 WHERE id = ?2",
            params![rating, id],
        )?;
    }
    if let Some(ref tags) = update.tags {
        conn.execute(
            "UPDATE tracks SET tags = ?1 WHERE id = ?2",
            params![tags, id],
        )?;
    }
    if let Some(ref key) = update.key {
        conn.execute("UPDATE tracks SET key = ?1 WHERE id = ?2", params![key, id])?;
    }
    if let Some(ref credits) = update.credits {
        conn.execute(
            "UPDATE tracks SET credits = ?1 WHERE id = ?2",
            params![credits, id],
        )?;
    }
    if let Some(ref label) = update.label {
        conn.execute(
            "UPDATE tracks SET label = ?1 WHERE id = ?2",
            params![label, id],
        )?;
    }
    if let Some(ref blob) = update.artwork_blob {
        conn.execute(
            "UPDATE tracks SET artwork_blob = ?1 WHERE id = ?2",
            params![blob, id],
        )?;
        let thumb = crate::thumb::make_thumb(blob);
        conn.execute(
            "UPDATE tracks SET artwork_thumb = ?1 WHERE id = ?2",
            params![thumb, id],
        )?;
    }
    if let Some(ref path) = update.artwork_path {
        conn.execute(
            "UPDATE tracks SET artwork_path = ?1 WHERE id = ?2",
            params![path, id],
        )?;
    }
    if let Some(ref user_options) = update.user_options {
        conn.execute(
            "UPDATE tracks SET user_options = ?1 WHERE id = ?2",
            params![user_options, id],
        )?;
    }
    if let Some(ref format) = update.format {
        conn.execute(
            "UPDATE tracks SET format = ?1 WHERE id = ?2",
            params![format, id],
        )?;
    }
    if let Some(bitrate) = update.bitrate {
        conn.execute(
            "UPDATE tracks SET bitrate = ?1 WHERE id = ?2",
            params![bitrate, id],
        )?;
    }
    if let Some(ref remote_path) = update.remote_path {
        conn.execute(
            "UPDATE tracks SET remote_path = ?1 WHERE id = ?2",
            params![remote_path, id],
        )?;
    }
    if let Some(ref remote_data) = update.remote_data {
        conn.execute(
            "UPDATE tracks SET remote_data = ?1 WHERE id = ?2",
            params![remote_data, id],
        )?;
    }
    if let Some(ref track_data) = update.track_data {
        conn.execute(
            "UPDATE tracks SET track_data = ?1 WHERE id = ?2",
            params![track_data, id],
        )?;
    }
    Ok(())
}

pub fn update_track_metadata_by_uid(
    conn: &Connection,
    uid: &str,
    update: &MetadataUpdate,
) -> Result<()> {
    if let Some(ref title) = update.title {
        conn.execute(
            "UPDATE tracks SET title = ?1 WHERE uid = ?2",
            params![title, uid],
        )?;
    }
    if let Some(ref artists) = update.artists {
        conn.execute(
            "UPDATE tracks SET artists = ?1 WHERE uid = ?2",
            params![artists, uid],
        )?;
    }
    if let Some(ref album_artist) = update.album_artist {
        conn.execute(
            "UPDATE tracks SET album_artist = ?1 WHERE uid = ?2",
            params![album_artist, uid],
        )?;
    }
    if let Some(ref albums) = update.albums {
        conn.execute(
            "UPDATE tracks SET albums = ?1 WHERE uid = ?2",
            params![albums, uid],
        )?;
    }
    if let Some(ref year) = update.year {
        conn.execute(
            "UPDATE tracks SET year = ?1 WHERE uid = ?2",
            params![year, uid],
        )?;
    }
    if let Some(ref genres) = update.genres {
        conn.execute(
            "UPDATE tracks SET genres = ?1 WHERE uid = ?2",
            params![genres, uid],
        )?;
    }
    if let Some(bpm) = update.bpm {
        conn.execute(
            "UPDATE tracks SET bpm = ?1 WHERE uid = ?2",
            params![bpm, uid],
        )?;
    }
    if let Some(rating) = update.rating {
        conn.execute(
            "UPDATE tracks SET rating = ?1 WHERE uid = ?2",
            params![rating, uid],
        )?;
    }
    if let Some(ref tags) = update.tags {
        conn.execute(
            "UPDATE tracks SET tags = ?1 WHERE uid = ?2",
            params![tags, uid],
        )?;
    }
    if let Some(ref key) = update.key {
        conn.execute(
            "UPDATE tracks SET key = ?1 WHERE uid = ?2",
            params![key, uid],
        )?;
    }
    if let Some(ref credits) = update.credits {
        conn.execute(
            "UPDATE tracks SET credits = ?1 WHERE uid = ?2",
            params![credits, uid],
        )?;
    }
    if let Some(ref label) = update.label {
        conn.execute(
            "UPDATE tracks SET label = ?1 WHERE uid = ?2",
            params![label, uid],
        )?;
    }
    if let Some(ref blob) = update.artwork_blob {
        conn.execute(
            "UPDATE tracks SET artwork_blob = ?1 WHERE uid = ?2",
            params![blob, uid],
        )?;
        let thumb = crate::thumb::make_thumb(blob);
        conn.execute(
            "UPDATE tracks SET artwork_thumb = ?1 WHERE uid = ?2",
            params![thumb, uid],
        )?;
    }
    if let Some(ref path) = update.artwork_path {
        conn.execute(
            "UPDATE tracks SET artwork_path = ?1 WHERE uid = ?2",
            params![path, uid],
        )?;
    }
    if let Some(ref user_options) = update.user_options {
        conn.execute(
            "UPDATE tracks SET user_options = ?1 WHERE uid = ?2",
            params![user_options, uid],
        )?;
    }
    if let Some(ref format) = update.format {
        conn.execute(
            "UPDATE tracks SET format = ?1 WHERE uid = ?2",
            params![format, uid],
        )?;
    }
    if let Some(bitrate) = update.bitrate {
        conn.execute(
            "UPDATE tracks SET bitrate = ?1 WHERE uid = ?2",
            params![bitrate, uid],
        )?;
    }
    if let Some(ref remote_path) = update.remote_path {
        conn.execute(
            "UPDATE tracks SET remote_path = ?1 WHERE uid = ?2",
            params![remote_path, uid],
        )?;
    }
    if let Some(ref remote_data) = update.remote_data {
        conn.execute(
            "UPDATE tracks SET remote_data = ?1 WHERE uid = ?2",
            params![remote_data, uid],
        )?;
    }
    if let Some(ref track_data) = update.track_data {
        conn.execute(
            "UPDATE tracks SET track_data = ?1 WHERE uid = ?2",
            params![track_data, uid],
        )?;
    }
    Ok(())
}
