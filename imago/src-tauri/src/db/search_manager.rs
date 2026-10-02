use crate::db::album_manager::Album;
use crate::db::artist_manager::Artist;
use crate::db::track_manager::Track;
use rusqlite::{params, Connection, Result};

// ─── Track search ─────────────────────────────────────────────────────────────
//
// Searches across title, album_artist (JSON name field), artists (JSON array),
// albums (JSON array), tags, and genres using LIKE. Results are ranked by
// match quality: title match scores highest, then artist, then the rest.
// This mirrors the Fuse.js weights that were used on the frontend:
//   title (0.5), artists (0.25), album_artist (0.15), albums (0.1),
//   tags (0.05), genres (0.05)

pub fn search_tracks(conn: &Connection, query: &str) -> Result<Vec<Track>> {
    let pattern = format!("%{}%", query.to_lowercase());

    let mut stmt = conn.prepare(
        "SELECT
            id, uid, path, last_modified, title, artists, album_artist, albums,
            genres, year, rating, tags, duration_ms, bpm, key, credits, label,
            format, bitrate, artwork_path, artwork_thumb, user_options,
            remote_path, remote_data, track_data, date_added,
            -- Score: lower = better match priority (we ORDER ASC)
            CASE
                WHEN LOWER(COALESCE(title, '')) LIKE ?1 THEN 0
                WHEN LOWER(COALESCE(album_artist, '')) LIKE ?1 THEN 1
                WHEN LOWER(COALESCE(artists, '')) LIKE ?1 THEN 2
                WHEN LOWER(COALESCE(albums, '')) LIKE ?1 THEN 3
                WHEN LOWER(COALESCE(tags, '')) LIKE ?1 THEN 4
                WHEN LOWER(COALESCE(genres, '')) LIKE ?1 THEN 5
                ELSE 6
            END AS score
        FROM tracks
        WHERE
            LOWER(COALESCE(title, '')) LIKE ?1
            OR LOWER(COALESCE(album_artist, '')) LIKE ?1
            OR LOWER(COALESCE(artists, '')) LIKE ?1
            OR LOWER(COALESCE(albums, '')) LIKE ?1
            OR LOWER(COALESCE(tags, '')) LIKE ?1
            OR LOWER(COALESCE(genres, '')) LIKE ?1
        ORDER BY score, title",
    )?;

    let tracks = stmt
        .query_map(params![pattern], |row| {
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

// ─── Album search ─────────────────────────────────────────────────────────────
//
// Weights: title (0.5), artists (0.25), album_artist (0.15), year (0.1),
//          tags (0.05), genres (0.05)

pub fn search_albums(conn: &Connection, query: &str) -> Result<Vec<Album>> {
    let pattern = format!("%{}%", query.to_lowercase());

    let mut stmt = conn.prepare(
        "SELECT
            id, uid, format, title, rating, artists, album_artist, release_date,
            tags, genres, tracks, credits, label, artwork_path, artwork_thumb,
            emulate_type,
            CASE
                WHEN LOWER(COALESCE(title, '')) LIKE ?1 THEN 0
                WHEN LOWER(COALESCE(album_artist, '')) LIKE ?1 THEN 1
                WHEN LOWER(COALESCE(artists, '')) LIKE ?1 THEN 2
                WHEN LOWER(COALESCE(release_date, '')) LIKE ?1 THEN 3
                WHEN LOWER(COALESCE(tags, '')) LIKE ?1 THEN 4
                WHEN LOWER(COALESCE(genres, '')) LIKE ?1 THEN 5
                ELSE 6
            END AS score
        FROM albums
        WHERE
            LOWER(COALESCE(title, '')) LIKE ?1
            OR LOWER(COALESCE(album_artist, '')) LIKE ?1
            OR LOWER(COALESCE(artists, '')) LIKE ?1
            OR LOWER(COALESCE(release_date, '')) LIKE ?1
            OR LOWER(COALESCE(tags, '')) LIKE ?1
            OR LOWER(COALESCE(genres, '')) LIKE ?1
        ORDER BY score, title",
    )?;

    let albums = stmt
        .query_map(params![pattern], |row| {
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

// ─── Artist search ────────────────────────────────────────────────────────────
//
// Weights: name (0.5), aka (0.35), tags (0.05), genres (0.05)

pub fn search_artists(conn: &Connection, query: &str) -> Result<Vec<Artist>> {
    let pattern = format!("%{}%", query.to_lowercase());

    let mut stmt = conn.prepare(
        "SELECT
            id, uid, name, aka, about, tags, genres, websites, members,
            profile_art_path, profile_art_thumb, banner_art_path,
            CASE
                WHEN LOWER(COALESCE(name, '')) LIKE ?1 THEN 0
                WHEN LOWER(COALESCE(aka, '')) LIKE ?1 THEN 1
                WHEN LOWER(COALESCE(tags, '')) LIKE ?1 THEN 2
                WHEN LOWER(COALESCE(genres, '')) LIKE ?1 THEN 3
                ELSE 4
            END AS score
        FROM artists
        WHERE
            LOWER(COALESCE(name, '')) LIKE ?1
            OR LOWER(COALESCE(aka, '')) LIKE ?1
            OR LOWER(COALESCE(tags, '')) LIKE ?1
            OR LOWER(COALESCE(genres, '')) LIKE ?1
        ORDER BY score, name",
    )?;

    let artists = stmt
        .query_map(params![pattern], |row| {
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

// ─── Tracks by UIDs ──────────────────────────────────────────────────────────
//
// Fetches a batch of tracks by their UIDs in a single query.
// Order is not guaranteed by the DB; the caller sorts as needed.

pub fn get_tracks_by_uids(conn: &Connection, uids: &[String]) -> Result<Vec<Track>> {
    if uids.is_empty() {
        return Ok(vec![]);
    }

    // Build a parameterised IN clause
    let placeholders = uids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!(
        "SELECT id, uid, path, last_modified, title, artists, album_artist, albums,
                genres, year, rating, tags, duration_ms, bpm, key, credits, label,
                format, bitrate, artwork_path, artwork_thumb, user_options,
                remote_path, remote_data, track_data, date_added
         FROM tracks
         WHERE uid IN ({})",
        placeholders
    );

    let mut stmt = conn.prepare(&sql)?;

    let tracks = stmt
        .query_map(rusqlite::params_from_iter(uids.iter()), |row| {
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

// ─── Artist albums ────────────────────────────────────────────────────────────
//
// Returns all albums whose album_artist matches the artist's name (or one of
// its aka names). Also catches featured appearances via a LIKE scan on the
// artists JSON array (a plain array of name strings).
// aka_uids is the list of alias artist UIDs to include alongside the primary.

pub fn get_artist_albums(
    conn: &Connection,
    artist_uid: &str,
    aka_uids: &[String],
) -> Result<Vec<Album>> {
    // Resolve every uid (primary + akas) to its artist name
    let mut all_uids = vec![artist_uid.to_string()];
    all_uids.extend_from_slice(aka_uids);

    let names: Vec<String> = all_uids
        .iter()
        .filter_map(|uid| {
            crate::db::artist_manager::get_artist_by_uid(conn, uid)
                .ok()
                .flatten()
        })
        .map(|a| a.name)
        .collect();

    if names.is_empty() {
        return Ok(vec![]);
    }

    // Build OR conditions for each name
    let name_conditions: Vec<String> = names
        .iter()
        .enumerate()
        .map(|(i, _)| {
            format!(
                "LOWER(album_artist) = ?{idx} OR LOWER(artists) LIKE ?{like_idx}",
                idx = i * 2 + 1,
                like_idx = i * 2 + 2,
            )
        })
        .collect();

    let sql = format!(
        "SELECT id, uid, format, title, rating, artists, album_artist, release_date,
                tags, genres, tracks, credits, label, artwork_path, artwork_thumb, emulate_type
         FROM albums
         WHERE {}
         ORDER BY album_artist, title",
        name_conditions.join(" OR ")
    );

    let mut stmt = conn.prepare(&sql)?;

    // Interleave the exact-match value and LIKE pattern for each name
    let params_vec: Vec<Box<dyn rusqlite::ToSql>> = names
        .iter()
        .flat_map(|name| {
            let lower = name.to_lowercase();
            let like_pattern = format!("%\"{}\"%", lower);
            let eq_val: Box<dyn rusqlite::ToSql> = Box::new(lower);
            let like_val: Box<dyn rusqlite::ToSql> = Box::new(like_pattern);
            vec![eq_val, like_val]
        })
        .collect();

    let albums = stmt
        .query_map(
            rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
            |row| {
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
            },
        )?
        .collect::<Result<Vec<_>>>()?;

    // Deduplicate by uid (a featuring appearance + album_artist match could double-return)
    let mut seen = std::collections::HashSet::new();
    let deduped = albums
        .into_iter()
        .filter(|a| seen.insert(a.uid.clone()))
        .collect();

    Ok(deduped)
}
