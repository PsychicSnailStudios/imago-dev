use crate::db::generate_uid;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scrobble {
	pub uid: String,
	pub timestamp: i64,
	pub track_uid: String,
	pub artist_uid: String,
	pub album_uid: Option<String>,
	pub source_uid: Option<String>,
	pub duration_played: i64,
	pub did_seek: bool,
	pub did_pause: bool,
	pub reason_start: Option<String>,
	pub reason_end: Option<String>,
	pub shuffle: Option<bool>,
	pub skipped: Option<bool>,
	pub offline: Option<bool>,
	pub playing_local: Option<bool>,
	pub track_name: Option<String>,
	pub track_artist: Option<String>,
	pub track_album: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrobbleUpdate {
	pub duration_played: i64,
	pub did_seek: bool,
	pub did_pause: bool,
	pub reason_end: Option<String>,
	pub skipped: Option<bool>,
}

pub fn init_analytics_db(conn: &Connection) -> Result<()> {
	conn.execute_batch(
		"
		CREATE TABLE IF NOT EXISTS scrobbles (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			uid TEXT NOT NULL UNIQUE,
			timestamp INTEGER NOT NULL,
			track_uid TEXT NOT NULL,
			artist_uid TEXT NOT NULL,
			duration_played INTEGER NOT NULL DEFAULT 0,
			did_seek INTEGER NOT NULL DEFAULT 0,
			did_pause INTEGER NOT NULL DEFAULT 0,
			reason_start TEXT,
			reason_end TEXT,
			shuffle INTEGER,
			skipped INTEGER,
			offline INTEGER,
			playing_local INTEGER,
			track_name TEXT,
			track_artist TEXT,
			track_album TEXT,
			album_uid TEXT,
			source_uid TEXT
		);
		CREATE INDEX IF NOT EXISTS idx_scrobbles_uid ON scrobbles (uid);
		CREATE INDEX IF NOT EXISTS idx_scrobbles_track_uid ON scrobbles (track_uid);
		CREATE INDEX IF NOT EXISTS idx_scrobbles_artist_uid ON scrobbles (artist_uid);
		CREATE INDEX IF NOT EXISTS idx_scrobbles_timestamp ON scrobbles (timestamp);
	",
	)?;

	let columns = [
		("reason_start", "TEXT"),
		("reason_end", "TEXT"),
		("shuffle", "INTEGER"),
		("skipped", "INTEGER"),
		("offline", "INTEGER"),
		("playing_local", "INTEGER"),
		("track_name", "TEXT"),
		("track_artist", "TEXT"),
		("track_album", "TEXT"),
		("album_uid", "TEXT"),
	];
	for (col, ty) in &columns {
		let _ = conn.execute(
			&format!("ALTER TABLE scrobbles ADD COLUMN {} {}", col, ty),
			[],
		);
	}

	Ok(())
}

pub fn log_scrobble(conn: &Connection, scrobble: &Scrobble) -> Result<()> {
	conn.execute(
		"INSERT INTO scrobbles (
			uid, timestamp, track_uid, artist_uid, duration_played, did_seek, did_pause,
			reason_start, reason_end, shuffle, skipped, offline, playing_local,
			track_name, track_artist, track_album, album_uid, source_uid
		) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)
		ON CONFLICT(uid) DO NOTHING",
		params![
			scrobble.uid,
			scrobble.timestamp,
			scrobble.track_uid,
			scrobble.artist_uid,
			scrobble.duration_played,
			scrobble.did_seek as i64,
			scrobble.did_pause as i64,
			scrobble.reason_start,
			scrobble.reason_end,
			scrobble.shuffle.map(|b| b as i64),
			scrobble.skipped.map(|b| b as i64),
			scrobble.offline.map(|b| b as i64),
			scrobble.playing_local.map(|b| b as i64),
			scrobble.track_name,
			scrobble.track_artist,
			scrobble.track_album,
			scrobble.album_uid,
			scrobble.source_uid,
		],
	)?;
	Ok(())
}

fn row_to_scrobble(row: &rusqlite::Row) -> rusqlite::Result<Scrobble> {
	Ok(Scrobble {
		uid: row.get(0)?,
		timestamp: row.get(1)?,
		track_uid: row.get(2)?,
		artist_uid: row.get(3)?,
		duration_played: row.get(4)?,
		did_seek: row.get::<_, i64>(5)? != 0,
		did_pause: row.get::<_, i64>(6)? != 0,
		reason_start: row.get(7)?,
		reason_end: row.get(8)?,
		shuffle: row.get::<_, Option<i64>>(9)?.map(|v| v != 0),
		skipped: row.get::<_, Option<i64>>(10)?.map(|v| v != 0),
		offline: row.get::<_, Option<i64>>(11)?.map(|v| v != 0),
		playing_local: row.get::<_, Option<i64>>(12)?.map(|v| v != 0),
		track_name: row.get(13)?,
		track_artist: row.get(14)?,
		track_album: row.get(15)?,
		album_uid: row.get(16)?,
		source_uid: row.get(17)?,
	})
}

const SELECT_COLS: &str =
	"uid, timestamp, track_uid, artist_uid, duration_played, did_seek, did_pause,
	reason_start, reason_end, shuffle, skipped, offline, playing_local,
	track_name, track_artist, track_album, album_uid, source_uid";

pub fn get_all_scrobbles(conn: &Connection) -> Result<Vec<Scrobble>> {
	let mut stmt = conn.prepare(&format!(
		"SELECT {} FROM scrobbles ORDER BY timestamp DESC",
		SELECT_COLS
	))?;
	let rows = stmt.query_map([], row_to_scrobble)?;
	rows.collect()
}

pub fn get_scrobbles_for_track(conn: &Connection, track_uid: &str) -> Result<Vec<Scrobble>> {
	let mut stmt = conn.prepare(&format!(
		"SELECT {} FROM scrobbles WHERE track_uid = ?1 ORDER BY timestamp DESC",
		SELECT_COLS
	))?;
	let rows = stmt.query_map(params![track_uid], row_to_scrobble)?;
	rows.collect()
}

pub fn update_scrobble(conn: &Connection, uid: &str, update: &ScrobbleUpdate) -> Result<()> {
	conn.execute(
		"UPDATE scrobbles
		SET duration_played = ?1, did_seek = ?2, did_pause = ?3,
		    reason_end = COALESCE(?4, reason_end),
		    skipped = COALESCE(?5, skipped)
		WHERE uid = ?6",
		params![
			update.duration_played,
			update.did_seek as i64,
			update.did_pause as i64,
			update.reason_end,
			update.skipped.map(|b| b as i64),
			uid,
		],
	)?;
	Ok(())
}

pub fn delete_scrobble(conn: &Connection, uid: &str) -> Result<()> {
	conn.execute("DELETE FROM scrobbles WHERE uid = ?1", params![uid])?;
	Ok(())
}

pub fn new_scrobble_uid() -> String {
	generate_uid("sc-")
}