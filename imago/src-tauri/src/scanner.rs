use crate::db::{
    create_album, create_artist, generate_uid, get_all_albums, get_all_artists,
    get_setting, update_album, update_track_metadata_by_uid, upsert_track,
    Album, AlbumUpdate, Artist, MetadataUpdate, Track,
};
use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use lofty::tag::Accessor;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;
use std::time::UNIX_EPOCH;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

// (title_lower, album_artist_lower) -> (id, uid, tracks_json, label)
type AlbumCache = HashMap<(String, String), (i64, String, String, Option<String>)>;
// name_lower -> (id, uid)
type ArtistCache = HashMap<String, (i64, String)>;

const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "aac", "wav", "aiff", "ogg"];

const ARTIST_TAG_DELIMITERS: &[&str] = &[" / ", "; ", ", ", ",", " feat. ", " ft. ", " featuring "];
const ARTIST_FILENAME_DELIMITERS: &[&str] = &[" / ", "; ", " feat. ", " ft. ", " featuring "];
const GENRE_DELIMITERS: &[&str] = &[" / ", "; ", ", "];

fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn get_last_modified(path: &Path) -> i64 {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn split_on_delimiters(input: &str, delimiters: &[&str]) -> Vec<String> {
    let mut results = vec![input.to_string()];
    for delim in delimiters {
        results = results
            .into_iter()
            .flat_map(|s| {
                s.split(delim)
                    .map(|p| p.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .filter(|s| !s.is_empty())
            .collect();
    }
    results
}

fn normalize_rating(raw: &str) -> Option<f32> {
    let val = raw.trim().parse::<f32>().ok()?;
    if val <= 10.0 {
        Some(val)
    } else if val <= 255.0 {
        Some((val / 255.0) * 10.0)
    } else {
        None
    }
}

fn try_parse_ampersand(artists: Vec<String>) -> Vec<String> {
    let lower_existing: Vec<String> = artists.iter().map(|a| a.to_lowercase()).collect();
    let mut result: Vec<String> = Vec::new();

    for artist in &artists {
        if artist.contains('&') {
            let parts: Vec<String> = artist
                .split('&')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();

            let new_parts: Vec<&String> = parts
                .iter()
                .filter(|p| !lower_existing.contains(&p.to_lowercase()))
                .collect();

            if new_parts.is_empty() || new_parts.len() <= 2 {
                for part in &parts {
                    let lower = part.to_lowercase();
                    if !result.iter().any(|r: &String| r.to_lowercase() == lower) {
                        result.push(part.clone());
                    }
                }
                continue;
            }
        }

        let lower = artist.to_lowercase();
        if !result.iter().any(|r: &String| r.to_lowercase() == lower) {
            result.push(artist.clone());
        }
    }

    result
}

fn extract_feat_artists(title: &str) -> Vec<String> {
    let lower = title.to_lowercase();
    let patterns = ["feat.", "ft.", "featuring"];

    for pat in &patterns {
        if let Some(pos) = lower.find(pat) {
            let after_pat = &title[pos + pat.len()..];
            let raw = after_pat
                .trim_start_matches(|c: char| c == '.' || c == ' ')
                .trim_end_matches(')')
                .trim_end_matches(']')
                .trim();
            let parts: Vec<String> = raw
                .split(&[',', '&', '/'][..])
                .map(|p| {
                    p.trim()
                        .trim_matches(|c| c == '(' || c == '[' || c == ')' || c == ']')
                        .trim()
                        .to_string()
                })
                .filter(|p| !p.is_empty())
                .collect();
            if !parts.is_empty() {
                return parts;
            }
        }
    }
    Vec::new()
}

fn is_verified_folder_structure(path: &Path) -> bool {
    let components: Vec<&str> = path
        .parent()
        .map(|p| {
            p.components()
                .filter_map(|c| c.as_os_str().to_str())
                .collect()
        })
        .unwrap_or_default();

    if components.len() < 2 {
        return false;
    }

    let folder = components[components.len() - 1];
    let artist = components[components.len() - 2];

    let generic = [
        "music",
        "audio",
        "downloads",
        "files",
        "media",
        "tracks",
        "songs",
    ];
    if generic.contains(&folder.to_lowercase().as_str())
        || generic.contains(&artist.to_lowercase().as_str())
    {
        return false;
    }

    folder.len() >= 2 && artist.len() >= 2
}

#[derive(Debug)]
struct FilenameMetadata {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    year: Option<String>,
    disc: Option<u32>,
}

fn parse_filename(path: &Path, custom_pattern: &str) -> FilenameMetadata {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    if !custom_pattern.is_empty() {
        return parse_custom_pattern(&stem, custom_pattern);
    }

    let patterns: &[&[&str]] = &[
        &["{artist}", "{album}", "{year}", "{title}"],
        &["{year}", "{artist}", "{album}", "{title}"],
        &["{artist}", "{title}"],
        &["{title}"],
    ];

    for pattern in patterns {
        if let Some(meta) = try_parse_pattern(&stem, pattern) {
            return meta;
        }
    }

    FilenameMetadata {
        title: Some(stem),
        artist: None,
        album: None,
        year: None,
        disc: None,
    }
}

fn try_parse_pattern(stem: &str, pattern: &[&str]) -> Option<FilenameMetadata> {
    let parts: Vec<&str> = stem.splitn(pattern.len(), " - ").collect();
    if parts.len() != pattern.len() {
        return None;
    }

    let mut meta = FilenameMetadata {
        title: None,
        artist: None,
        album: None,
        year: None,
        disc: None,
    };

    for (i, &field) in pattern.iter().enumerate() {
        let value = parts[i].trim().to_string();
        match field {
            "{title}" => meta.title = Some(value),
            "{artist}" => meta.artist = Some(value),
            "{album}" => meta.album = Some(value),
            "{year}" => {
                if value.len() == 4 && value.parse::<u32>().is_ok() {
                    meta.year = Some(value);
                } else {
                    return None;
                }
            }
            _ => {}
        }
    }

    Some(meta)
}

fn parse_custom_pattern(stem: &str, pattern: &str) -> FilenameMetadata {
    let delimiters: Vec<&str> = pattern
        .split(|c: char| c == '{' || c == '}')
        .filter(|s| !["title", "artist", "album", "year"].contains(s) && !s.is_empty())
        .collect();

    let fields: Vec<&str> = pattern
        .split(|c: char| !c.is_alphanumeric() && c != '{' && c != '}')
        .filter(|s| s.starts_with('{') && s.ends_with('}'))
        .map(|s| s.trim_matches(|c| c == '{' || c == '}'))
        .collect();

    let mut remaining = stem;
    let mut values: Vec<String> = Vec::new();

    for (i, delim) in delimiters.iter().enumerate() {
        if i < fields.len() {
            if let Some(pos) = remaining.find(delim) {
                values.push(remaining[..pos].trim().to_string());
                remaining = &remaining[pos + delim.len()..];
            } else {
                break;
            }
        }
    }
    values.push(remaining.trim().to_string());

    let mut meta = FilenameMetadata {
        title: None,
        artist: None,
        album: None,
        year: None,
        disc: None,
    };
    for (i, field) in fields.iter().enumerate() {
        if let Some(value) = values.get(i) {
            match *field {
                "title" => meta.title = Some(value.clone()),
                "artist" => meta.artist = Some(value.clone()),
                "album" => meta.album = Some(value.clone()),
                "year" => meta.year = Some(value.clone()),
                _ => {}
            }
        }
    }

    meta
}

fn extract_year_from_folder(folder: &str) -> Option<(String, String)> {
    if let Some(pos) = folder.find(" - ") {
        let maybe_year = &folder[..pos];
        if maybe_year.len() == 4 && maybe_year.parse::<u32>().is_ok() {
            return Some((folder[pos + 3..].trim().to_string(), maybe_year.to_string()));
        }
    }

    if folder.ends_with(')') {
        if let Some(open) = folder.rfind('(') {
            let maybe_year = &folder[open + 1..folder.len() - 1];
            if maybe_year.len() == 4 && maybe_year.parse::<u32>().is_ok() {
                return Some((folder[..open].trim().to_string(), maybe_year.to_string()));
            }
        }
    }

    None
}

fn is_disc_folder(name: &str) -> bool {
    let lower = name.to_lowercase();
    let prefixes = [
        "disc",
        "disk",
        "cd",
        "side",
        "volume",
        "vol",
        "digital media",
    ];
    for prefix in &prefixes {
        if lower.starts_with(prefix) {
            let rest = lower[prefix.len()..]
                .trim_start_matches(|c: char| c == '.' || c == ' ' || c == '-');
            if rest.is_empty() {
                return true;
            }
            let num_str: String = rest.chars().take_while(|c| c.is_alphanumeric()).collect();
            if !num_str.is_empty() {
                return true;
            }
        }
    }
    false
}

fn parse_disc_from_folder(path: &Path) -> Option<u32> {
    let folder = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())?;

    let lower = folder.to_lowercase();

    let prefixes = [
        "disc",
        "disk",
        "cd",
        "side",
        "volume",
        "vol",
        "digital media",
    ];
    for prefix in &prefixes {
        if lower.starts_with(prefix) {
            let rest = lower[prefix.len()..]
                .trim_start_matches(|c: char| c == '.' || c == ' ' || c == '-');
            if rest.is_empty() {
                return Some(1);
            }
            let num_str: String = rest.chars().take_while(|c| c.is_alphanumeric()).collect();
            if num_str.is_empty() {
                continue;
            }
            if let Ok(n) = num_str.parse::<u32>() {
                return Some(n);
            }
            let letter_disc = match num_str.as_str() {
                "a" => Some(1u32),
                "b" => Some(2),
                "c" => Some(3),
                "d" => Some(4),
                "e" => Some(5),
                "f" => Some(6),
                _ => None,
            };
            if letter_disc.is_some() {
                return letter_disc;
            }
        }
    }
    None
}

fn parse_folder_path(path: &Path) -> FilenameMetadata {
    let components: Vec<&str> = path
        .parent()
        .map(|p| {
            p.components()
                .filter_map(|c| c.as_os_str().to_str())
                .collect()
        })
        .unwrap_or_default();

    if components.len() < 2 {
        return FilenameMetadata {
            title: None,
            artist: None,
            album: None,
            year: None,
            disc: None,
        };
    }

    let immediate = components[components.len() - 1];

    let (folder, artist_idx) = if is_disc_folder(immediate) && components.len() >= 3 {
        (components[components.len() - 2], components.len() - 3)
    } else {
        (immediate, components.len() - 2)
    };

    if artist_idx >= components.len() {
        return FilenameMetadata {
            title: None,
            artist: None,
            album: None,
            year: None,
            disc: None,
        };
    }

    let artist = components[artist_idx].to_string();

    if artist_idx >= 1 {
        let maybe_year = components[artist_idx - 1];
        if maybe_year.len() == 4 && maybe_year.parse::<u32>().is_ok() {
            return FilenameMetadata {
                title: None,
                artist: Some(artist),
                album: Some(folder.to_string()),
                year: Some(maybe_year.to_string()),
                disc: None,
            };
        }
    }

    if let Some(captures) = extract_year_from_folder(folder) {
        return FilenameMetadata {
            title: None,
            artist: Some(artist),
            album: Some(captures.0),
            year: Some(captures.1),
            disc: None,
        };
    }

    FilenameMetadata {
        title: None,
        artist: Some(artist),
        album: Some(folder.to_string()),
        year: None,
        disc: None,
    }
}

pub fn find_remote_local_counterpart(conn: &Connection, incoming: &Track) -> Option<String> {
    let incoming_title = incoming.title.as_deref()?.to_lowercase();

    let incoming_artist_owned: Option<String> = incoming
        .album_artist
        .as_ref()
        .map(|s| s.to_lowercase())
        .or_else(|| crate::db::first_artist_name(&incoming.artists).map(|s| s.to_lowercase()));

    let incoming_duration = incoming.duration_ms.unwrap_or(0);
    let incoming_is_remote = incoming
        .remote_path
        .as_deref()
        .map(|rp| !rp.is_empty())
        .unwrap_or(false)
        || incoming.path.is_empty();

    let mut stmt = conn
        .prepare(
            "SELECT uid, path, remote_path, duration_ms, album_artist, artists
				FROM tracks
				WHERE LOWER(TRIM(title)) = LOWER(TRIM(?1))",
        )
        .ok()?;

    let rows: Vec<(
        String,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<String>,
        Option<String>,
    )> = {
        stmt.query_map(rusqlite::params![incoming_title], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .ok()?
        .filter_map(|r| r.ok())
        .collect()
    };

    for (uid, path, remote_path, duration_ms, album_artist_json, artists_json) in rows {
        let existing_is_remote = remote_path
            .as_deref()
            .map(|rp| !rp.is_empty())
            .unwrap_or(false)
            || path.as_deref().map(|p| p.is_empty()).unwrap_or(true);

        if existing_is_remote == incoming_is_remote {
            continue;
        }

        let dur = duration_ms.unwrap_or(0);
        if (dur - incoming_duration).abs() > 1000 {
            continue;
        }

        let existing_artist: Option<String> = album_artist_json
            .as_ref()
            .map(|s| s.to_lowercase())
            .or_else(|| crate::db::first_artist_name(&artists_json).map(|s| s.to_lowercase()));

        if existing_artist == incoming_artist_owned {
            return Some(uid);
        }
    }

    None
}

pub fn merge_paths_into_existing(conn: &Connection, existing_uid: &str, incoming: &Track) {
    let row: Option<(Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT path, remote_path FROM tracks WHERE uid = ?1",
            rusqlite::params![existing_uid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    let (existing_path, existing_remote) = match row {
        Some(r) => r,
        None => return,
    };

    let need_path = existing_path
        .as_deref()
        .map(|p| p.is_empty())
        .unwrap_or(true);
    let need_remote = existing_remote
        .as_deref()
        .map(|rp| rp.is_empty())
        .unwrap_or(true);

    if need_path {
        if !incoming.path.is_empty() {
            conn.execute(
                "UPDATE tracks SET path = ?1 WHERE uid = ?2",
                rusqlite::params![incoming.path, existing_uid],
            )
            .ok();
        }
    }

    if need_remote {
        if let Some(ref rp) = incoming.remote_path {
            if !rp.is_empty() {
                conn.execute(
                    "UPDATE tracks SET remote_path = ?1 WHERE uid = ?2",
                    rusqlite::params![rp, existing_uid],
                )
                .ok();
            }
        }
    }

    let _ = conn.execute(
        "UPDATE tracks SET
			title        = COALESCE(title,        ?2),
			artists      = COALESCE(artists,      ?3),
			album_artist = COALESCE(album_artist, ?4),
			albums       = COALESCE(albums,       ?5),
			genres       = COALESCE(genres,       ?6),
			year         = COALESCE(year,         ?7),
			duration_ms  = COALESCE(duration_ms,  ?8),
			bpm          = COALESCE(bpm,          ?9),
			key          = COALESCE(key,          ?10),
			label        = COALESCE(label,        ?11),
			artwork_blob = COALESCE(artwork_blob, ?12),
			artwork_path = COALESCE(artwork_path, ?13),
			format       = COALESCE(format,       ?14),
			bitrate      = COALESCE(bitrate,      ?15)
		WHERE uid = ?1",
        rusqlite::params![
            existing_uid,
            incoming.title,
            incoming.artists,
            incoming.album_artist,
            incoming.albums,
            incoming.genres,
            incoming.year,
            incoming.duration_ms,
            incoming.bpm,
            incoming.key,
            incoming.label,
            incoming.artwork_blob,
            incoming.artwork_path,
            incoming.format,
            incoming.bitrate,
        ],
    );
}

pub fn read_track(path: &Path) -> Option<Track> {
    read_track_with_settings(
        path,
        "tag",
        "tag",
        "tag",
        "tag",
        "",
        ARTIST_TAG_DELIMITERS,
        ARTIST_FILENAME_DELIMITERS,
        GENRE_DELIMITERS,
        false,
    )
}

pub fn read_track_with_settings(
    path: &Path,
    priority_title: &str,
    priority_artist: &str,
    priority_album: &str,
    priority_year: &str,
    custom_pattern: &str,
    artist_tag_delimiters: &[&str],
    artist_filename_delimiters: &[&str],
    genre_delimiters: &[&str],
    try_ampersand: bool,
) -> Option<Track> {
    let last_modified = get_last_modified(path);
    let path_str = path.to_string_lossy().to_string();

    let tagged_file = Probe::open(path)
        .ok()?
        .guess_file_type()
        .ok()?
        .read()
        .ok()?;

    let properties = tagged_file.properties();
    let duration_ms = Some(properties.duration().as_millis() as i64);
    let bitrate = properties.overall_bitrate().map(|b| b as i64);

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    let filename_meta = parse_filename(path, custom_pattern);
    let folder_meta = parse_folder_path(path);
    let folder_verified = is_verified_folder_structure(path);

    let (
        tag_title,
        tag_artist,
        tag_album,
        tag_album_artist,
        tag_genres,
        tag_year,
        tag_rating,
        tag_track_number,
        tag_disc_number,
        bpm,
        key,
        artwork_blob,
        tag_label,
    ) = if let Some(tag) = tag {
        let artwork_blob = tag.pictures().first().map(|p| p.data().to_vec());
        let rating = tag
            .get_string(&lofty::tag::ItemKey::Popularimeter)
            .and_then(|s| normalize_rating(s));
        let track_number = tag
            .get_string(&lofty::tag::ItemKey::TrackNumber)
            .and_then(|s| {
                s.split('/')
                    .next()
                    .and_then(|n| n.trim().parse::<u32>().ok())
            });
        let disc_number = tag
            .get_string(&lofty::tag::ItemKey::DiscNumber)
            .and_then(|s| {
                s.split('/')
                    .next()
                    .and_then(|n| n.trim().parse::<u32>().ok())
            });
        let label = tag
            .get_string(&lofty::tag::ItemKey::Publisher)
            .map(|s| s.to_string())
            .or_else(|| {
                tag.get_string(&lofty::tag::ItemKey::CopyrightMessage)
                    .map(|s| s.to_string())
            });
        (
            tag.title().map(|s| s.to_string()),
            tag.artist().map(|s| s.to_string()),
            tag.album().map(|s| s.to_string()),
            tag.get_string(&lofty::tag::ItemKey::AlbumArtist)
                .map(|s| s.to_string()),
            tag.genre().map(|s| s.to_string()),
            tag.year().map(|y| y.to_string()),
            rating,
            track_number,
            disc_number,
            tag.get_string(&lofty::tag::ItemKey::Bpm)
                .and_then(|s| s.parse::<f32>().ok()),
            tag.get_string(&lofty::tag::ItemKey::InitialKey)
                .map(|s| s.to_string()),
            artwork_blob,
            label,
        )
    } else {
        (
            None, None, None, None, None, None, None, None, None, None, None, None, None,
        )
    };

    let tag_artist = tag_artist.filter(|s| !s.trim().is_empty());
    let tag_album = tag_album.filter(|s| !s.trim().is_empty());
    let tag_album_artist = tag_album_artist.filter(|s| !s.trim().is_empty());

    let title = if priority_title == "filename" {
        filename_meta
            .title
            .clone()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| tag_title.clone().filter(|s| !s.trim().is_empty()))
    } else {
        tag_title
            .clone()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| filename_meta.title.clone().filter(|s| !s.trim().is_empty()))
    };

    let tag_artists: Option<Vec<String>> = tag_artist
        .as_deref()
        .map(|s| split_on_delimiters(s, artist_tag_delimiters));

    let filename_artists: Option<Vec<String>> = filename_meta
        .artist
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| split_on_delimiters(s, artist_filename_delimiters));

    let has_tag_artist = tag_artist.is_some();
    let has_filename_artist = filename_meta
        .artist
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    let folder_artists: Option<Vec<String>> =
        if folder_verified && !has_tag_artist && !has_filename_artist {
            folder_meta
                .artist
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .map(|s| split_on_delimiters(s, artist_tag_delimiters))
        } else {
            None
        };

    let mut resolved_artists: Option<Vec<String>> = match priority_artist {
        "filename" => filename_artists.or(tag_artists).or(folder_artists),
        "folder" => folder_artists.or(tag_artists).or(filename_artists),
        _ => tag_artists.or(filename_artists).or(folder_artists),
    };

    let title_str = title.as_deref().unwrap_or("");
    let filename_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let mut feat_artists = extract_feat_artists(title_str);
    for fa in extract_feat_artists(filename_stem) {
        if !feat_artists
            .iter()
            .any(|e: &String| e.to_lowercase() == fa.to_lowercase())
        {
            feat_artists.push(fa);
        }
    }

    if !feat_artists.is_empty() {
        let base = resolved_artists.unwrap_or_default();
        let mut merged = base;
        for fa in feat_artists {
            if !merged
                .iter()
                .any(|e: &String| e.to_lowercase() == fa.to_lowercase())
            {
                merged.push(fa);
            }
        }
        resolved_artists = Some(merged);
    }

    if try_ampersand {
        if let Some(artists) = resolved_artists {
            resolved_artists = Some(try_parse_ampersand(artists));
        }
    }

    // Resolve album_artist name (plain string at this stage)
    let album_artist_split: Option<Vec<String>> = tag_album_artist.as_deref().map(|s| {
        let parts = split_on_delimiters(s, artist_tag_delimiters);
        if try_ampersand {
            try_parse_ampersand(parts)
        } else {
            parts
        }
    });

    let album_artist_name: Option<String> = album_artist_split
        .and_then(|v| v.into_iter().next())
        .or_else(|| resolved_artists.as_ref().and_then(|v| v.first().cloned()));

    // Ensure album_artist appears first in the artists list
    let mut resolved_artists = resolved_artists;
    if let Some(ref aa) = album_artist_name {
        if let Some(ref mut list) = resolved_artists {
            let aa_lower = aa.to_lowercase();
            if let Some(pos) = list.iter().position(|a| a.to_lowercase() == aa_lower) {
                if pos != 0 {
                    let item = list.remove(pos);
                    list.insert(0, item);
                }
            } else if !list.is_empty() {
                list.insert(0, aa.clone());
            }
        }
    }

    let artists = resolved_artists
        .as_ref()
        .and_then(|names| serde_json::to_string(names).ok());

    let album_artist = album_artist_name.clone();

    let tag_album_val = tag_album.filter(|s| !s.trim().is_empty());
    let filename_album_val = filename_meta.album.filter(|s| !s.trim().is_empty());

    let primary_album_name: Option<String> = match priority_album {
        "filename" => filename_album_val.or(tag_album_val),
        _ => tag_album_val.or(filename_album_val),
    };

    let folder_disc = if tag_disc_number.is_none() {
        parse_disc_from_folder(path)
    } else {
        None
    };
    let effective_disc = tag_disc_number.or(folder_disc);

    let mut album_entries: Vec<serde_json::Value> = Vec::new();

    if let Some(ref a) = primary_album_name {
        album_entries.push(serde_json::json!({
            "uid": "",
            "name": a,
            "track_number": tag_track_number.unwrap_or(0),
            "disc": effective_disc.unwrap_or(0),
        }));
    }

    if folder_verified {
        if let Some(ref fa) = folder_meta.album {
            let fa_trimmed = fa.trim();
            if !fa_trimmed.is_empty() {
                let already_present = album_entries.iter().any(|e| {
                    e.get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| n.to_lowercase())
                        == Some(fa_trimmed.to_lowercase())
                });
                if !already_present {
                    album_entries.push(serde_json::json!({
                        "uid": "",
                        "name": fa_trimmed,
                        "track_number": tag_track_number.unwrap_or(0),
                        "disc": effective_disc.unwrap_or(0),
                    }));
                }
            }
        }
    }

    let albums = if album_entries.is_empty() {
        None
    } else {
        serde_json::to_string(&album_entries).ok()
    };

    let genres = tag_genres
        .map(|g| split_on_delimiters(&g, genre_delimiters))
        .and_then(|v| serde_json::to_string(&v).ok());

    let tag_year_val = tag_year.filter(|s| !s.trim().is_empty());
    let filename_year_val = filename_meta.year.filter(|s| !s.trim().is_empty());
    let folder_year_val = if folder_verified {
        folder_meta.year.filter(|s| !s.trim().is_empty())
    } else {
        None
    };

    let year = match priority_year {
        "filename" => filename_year_val.or(tag_year_val).or(folder_year_val),
        "folder" => folder_year_val.or(tag_year_val).or(filename_year_val),
        _ => tag_year_val.or(filename_year_val).or(folder_year_val),
    };

    let format = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_uppercase());

    let is_remote = path_str.starts_with("http://")
        || path_str.starts_with("https://")
        || path_str.starts_with("\\\\")
        || path_str.starts_with("//");

    let local_path = if is_remote {
        String::new()
    } else {
        path_str.clone()
    };

    let remote_path = if is_remote {
        Some(path_str.clone())
    } else {
        Some(String::new())
    };

    Some(Track {
        id: None,
        uid: generate_uid("t"),
        path: local_path,
        remote_path,
        last_modified,
        title,
        artists,
        album_artist,
        albums,
        genres,
        year,
        rating: tag_rating,
        tags: None,
        duration_ms,
        bpm,
        key,
        user_options: None,
        credits: None,
        label: tag_label,
        artwork_blob: artwork_blob.clone(),
        artwork_path: None,
        artwork_thumb: artwork_blob.as_deref().and_then(crate::thumb::make_thumb),
        format: format.clone(),
        bitrate,
        remote_data: None,
        track_data: serde_json::to_string(&serde_json::json!({
            "format": format,
            "bitrate": bitrate,
            "is_ghost": null,
        }))
        .ok(),
        date_added: None,
    })
}

fn find_or_create_album_cached(
    conn: &Connection,
    cache: &mut AlbumCache,
    title: &str,
    album_artist_name: Option<&str>,
    year: Option<&str>,
    genres: Option<&str>,
    artwork_blob: Option<Vec<u8>>,
    track_uid: &str,
    track_title: Option<&str>,
    track_number: Option<u32>,
    label: Option<&str>,
) -> Option<(i64, String)> {
    let title_lower = title.to_lowercase();
    let artist_lower = album_artist_name
        .map(|a| a.to_lowercase())
        .unwrap_or_default();
    let cache_key = (title_lower.clone(), artist_lower.clone());

    if let Some((cached_id, cached_uid, tracks_json, cached_label)) = cache.get(&cache_key).cloned()
    {
        let existing_tracks: Vec<serde_json::Value> =
            serde_json::from_str(&tracks_json).unwrap_or_default();

        let needs_label_update = cached_label.is_none() && label.is_some();
        let track_already_present = existing_tracks
            .iter()
            .any(|t| t["uid"].as_str() == Some(track_uid));

        if !track_already_present || needs_label_update {
            let mut updated = existing_tracks;
            if !track_already_present {
                updated.push(serde_json::json!({
                    "uid": track_uid,
                    "name": track_title.unwrap_or(""),
                    "track_number": track_number
                }));
            }
            let new_tracks_json = serde_json::to_string(&updated).ok()?;

            update_album(
                conn,
                cached_id,
                &AlbumUpdate {
                    format: None,
                    title: None,
                    rating: None,
                    artists: None,
                    album_artist: None,
                    release_date: None,
                    tags: None,
                    genres: None,
                    tracks: Some(new_tracks_json.clone()),
                    credits: None,
                    label: if needs_label_update {
                        label.map(|s| s.to_string())
                    } else {
                        None
                    },
                    artwork_blob: None,
                    artwork_path: None,
                    emulate_type: None,
                },
            )
            .ok()?;

            if let Some(entry) = cache.get_mut(&cache_key) {
                entry.2 = new_tracks_json;
                if needs_label_update {
                    entry.3 = label.map(|s| s.to_string());
                }
            }
        }

        return Some((cached_id, cached_uid));
    }

    let uid = generate_uid("a");
    let tracks_json = serde_json::to_string(&vec![serde_json::json!({
        "uid": track_uid,
        "name": track_title.unwrap_or(""),
        "track_number": track_number
    })])
    .unwrap_or_else(|_| "[]".to_string());

    let album_artist_val = album_artist_name.map(|n| n.to_string());
    let artists_val =
        album_artist_name.and_then(|n| serde_json::to_string(&vec![n.to_string()]).ok());
    let genres_val = genres.map(|g| g.to_string());

    let album = Album {
        id: None,
        uid: uid.clone(),
        format: None,
        title: title.to_string(),
        rating: None,
        artists: artists_val,
        album_artist: album_artist_val,
        release_date: year.map(|s| s.to_string()),
        tags: None,
        genres: genres_val,
        tracks: Some(tracks_json.clone()),
        credits: None,
        label: label.map(|s| s.to_string()),
        artwork_blob: artwork_blob.clone(),
        artwork_thumb: artwork_blob.as_deref().and_then(crate::thumb::make_thumb),
        artwork_path: None,
        emulate_type: None,
    };

    create_album(conn, &album).ok()?;

    let album_id: i64 = conn
        .query_row(
            "SELECT id FROM albums WHERE uid = ?1",
            rusqlite::params![uid],
            |row| row.get(0),
        )
        .ok()?;

    cache.insert(
        cache_key,
        (
            album_id,
            uid.clone(),
            tracks_json,
            label.map(|s| s.to_string()),
        ),
    );

    Some((album_id, uid))
}

fn find_or_create_album(
    conn: &Connection,
    title: &str,
    album_artist_name: Option<&str>,
    year: Option<&str>,
    genres: Option<&str>,
    artwork_blob: Option<Vec<u8>>,
    track_uid: &str,
    track_title: Option<&str>,
    track_number: Option<u32>,
    label: Option<&str>,
) -> Option<(i64, String)> {
    let existing = get_all_albums(conn).ok()?;

    let title_lower = title.to_lowercase();
    let artist_lower = album_artist_name.map(|a| a.to_lowercase());

    if let Some(album) = existing.iter().find(|a| {
        a.title.to_lowercase() == title_lower
            && a.album_artist.as_ref().map(|s| s.to_lowercase()) == artist_lower
    }) {
        let album_id = album.id?;
        let uid = album.uid.clone();

        let existing_tracks: Vec<serde_json::Value> = album
            .tracks
            .as_deref()
            .and_then(|t| serde_json::from_str(t).ok())
            .unwrap_or_default();

        let needs_label_update = album.label.is_none() && label.is_some();
        let track_already_present = existing_tracks
            .iter()
            .any(|t| t["uid"].as_str() == Some(track_uid));

        if !track_already_present || needs_label_update {
            let mut updated = existing_tracks;
            if !track_already_present {
                updated.push(serde_json::json!({
                    "uid": track_uid,
                    "name": track_title.unwrap_or(""),
                    "track_number": track_number
                }));
            }
            let tracks_json = serde_json::to_string(&updated).ok()?;

            update_album(
                conn,
                album_id,
                &AlbumUpdate {
                    format: None,
                    title: None,
                    rating: None,
                    artists: None,
                    album_artist: None,
                    release_date: None,
                    tags: None,
                    genres: None,
                    tracks: Some(tracks_json),
                    credits: None,
                    label: if needs_label_update {
                        label.map(|s| s.to_string())
                    } else {
                        None
                    },
                    artwork_blob: None,
                    artwork_path: None,
                    emulate_type: None,
                },
            )
            .ok()?;
        }

        return Some((album_id, uid));
    }

    let uid = generate_uid("a");
    let tracks_json = serde_json::to_string(&vec![serde_json::json!({
        "uid": track_uid,
        "name": track_title.unwrap_or(""),
        "track_number": track_number
    })])
    .unwrap_or_else(|_| "[]".to_string());

    let album_artist_val = album_artist_name.map(|n| n.to_string());
    let artists_val =
        album_artist_name.and_then(|n| serde_json::to_string(&vec![n.to_string()]).ok());
    let genres_val = genres.map(|g| g.to_string());

    let album = Album {
        id: None,
        uid: uid.clone(),
        format: None,
        title: title.to_string(),
        rating: None,
        artists: artists_val,
        album_artist: album_artist_val,
        release_date: year.map(|s| s.to_string()),
        tags: None,
        genres: genres_val,
        tracks: Some(tracks_json),
        credits: None,
        label: label.map(|s| s.to_string()),
        artwork_blob: artwork_blob.clone(),
        artwork_thumb: artwork_blob.as_deref().and_then(crate::thumb::make_thumb),
        artwork_path: None,
        emulate_type: None,
    };

    create_album(conn, &album).ok()?;

    let created = get_all_albums(conn).ok()?;
    let created_album = created.iter().find(|a| a.uid == uid)?;

    Some((created_album.id?, uid))
}

fn find_or_create_artist_cached(
    conn: &Connection,
    cache: &mut ArtistCache,
    name: &str,
) -> Option<(i64, String)> {
    let name_lower = name.to_lowercase();

    if let Some(&(id, ref uid)) = cache.get(&name_lower) {
        return Some((id, uid.clone()));
    }

    let uid = generate_uid("ar");
    let artist = Artist {
        id: None,
        uid: uid.clone(),
        name: name.to_string(),
        aka: None,
        about: None,
        tags: None,
        genres: None,
        websites: None,
        members: None,
        profile_art_blob: None,
        profile_art_path: None,
        banner_art_blob: None,
        banner_art_path: None,
        profile_art_thumb: None,
        banner_art_thumb: None,
    };

    create_artist(conn, &artist).ok()?;

    let artist_id: i64 = conn
        .query_row(
            "SELECT id FROM artists WHERE uid = ?1",
            rusqlite::params![uid],
            |row| row.get(0),
        )
        .ok()?;

    cache.insert(name_lower, (artist_id, uid.clone()));
    Some((artist_id, uid))
}

fn find_or_create_artist(conn: &Connection, name: &str) -> Option<(i64, String)> {
    let existing = get_all_artists(conn).ok()?;
    let name_lower = name.to_lowercase();

    if let Some(artist) = existing
        .iter()
        .find(|a| a.name.to_lowercase() == name_lower)
    {
        return Some((artist.id?, artist.uid.clone()));
    }

    let uid = generate_uid("ar");
    let artist = Artist {
        id: None,
        uid: uid.clone(),
        name: name.to_string(),
        aka: None,
        about: None,
        tags: None,
        genres: None,
        websites: None,
        members: None,
        profile_art_blob: None,
        profile_art_path: None,
        banner_art_blob: None,
        banner_art_path: None,
        profile_art_thumb: None,
        banner_art_thumb: None,
    };

    create_artist(conn, &artist).ok()?;

    let created = get_all_artists(conn).ok()?;
    let created_artist = created.iter().find(|a| a.uid == uid)?;

    Some((created_artist.id?, uid))
}

pub fn process_track(conn: &Connection, track: &Track) {
    let mut album_cache: AlbumCache = HashMap::new();
    let mut artist_cache: ArtistCache = HashMap::new();
    process_track_cached(conn, track, &mut album_cache, &mut artist_cache);
}

pub fn process_track_cached(
    conn: &Connection,
    track: &Track,
    album_cache: &mut AlbumCache,
    artist_cache: &mut ArtistCache,
) {
    let track_uid = track.uid.clone();
    let track_title = track.title.as_deref();

    let create_artists = get_setting(conn, "scan_create_artists")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true);

    let create_albums = get_setting(conn, "scan_create_albums")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true);

    if create_albums {
        if let Some(albums_str) = &track.albums {
            if let Ok(albums_arr) = serde_json::from_str::<Vec<serde_json::Value>>(albums_str) {
                let mut updated_album_entries: Vec<serde_json::Value> = Vec::new();

                let album_artist_name: Option<&str> = track.album_artist.as_deref();
                let genres_str: Option<String> = track.genres.clone();

                for album_entry in &albums_arr {
                    let entry_name = album_entry
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("");
                    if !entry_name.is_empty() {
                        let entry_track_number =
                            album_entry.get("track_number").and_then(|n| n.as_i64());
                        let entry_disc = album_entry.get("disc").and_then(|n| n.as_i64());
                        let track_number = entry_track_number.filter(|n| *n > 0).map(|n| n as u32);

                        if let Some((_album_id, album_uid)) = find_or_create_album_cached(
                            conn,
                            album_cache,
                            entry_name,
                            album_artist_name,
                            track.year.as_deref(),
                            genres_str.as_deref(),
                            track.artwork_blob.clone(),
                            &track_uid,
                            track_title,
                            track_number,
                            track.label.as_deref(),
                        ) {
                            updated_album_entries.push(serde_json::json!({
                                "uid": album_uid,
                                "name": entry_name,
                                "track_number": entry_track_number.unwrap_or(0),
                                "disc": entry_disc.unwrap_or(0),
                            }));
                        }
                    }
                }

                if !updated_album_entries.is_empty() {
                    if let Ok(patched_albums) = serde_json::to_string(&updated_album_entries) {
                        let _ = conn.execute(
                            "UPDATE tracks SET albums = ?1 WHERE uid = ?2",
                            rusqlite::params![patched_albums, track_uid],
                        );
                    }
                }
            }
        }
    }

    if create_artists {
        let mut all_names: Vec<String> = Vec::new();

        if let Some(names) = track
            .artists
            .as_deref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        {
            for name in names {
                if !all_names
                    .iter()
                    .any(|n: &String| n.to_lowercase() == name.to_lowercase())
                {
                    all_names.push(name);
                }
            }
        }
        if let Some(ref aa) = track.album_artist {
            if !aa.is_empty()
                && !all_names
                    .iter()
                    .any(|n: &String| n.to_lowercase() == aa.to_lowercase())
            {
                all_names.push(aa.clone());
            }
        }

        for name in &all_names {
            if !name.is_empty() {
                find_or_create_artist_cached(conn, artist_cache, name);
            }
        }
    }
}

pub fn scan_directory_with_progress(conn: &Connection, dir: &str, app: &AppHandle) {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
		 PRAGMA synchronous = NORMAL;
		 PRAGMA cache_size = -32000;",
    )
    .ok();

    let priority_title = get_setting(conn, "filename_priority_title")
        .ok()
        .flatten()
        .unwrap_or_else(|| "tag".to_string());
    let priority_artist = get_setting(conn, "filename_priority_artist")
        .ok()
        .flatten()
        .unwrap_or_else(|| "tag".to_string());
    let priority_album = get_setting(conn, "filename_priority_album")
        .ok()
        .flatten()
        .unwrap_or_else(|| "tag".to_string());
    let priority_year = get_setting(conn, "filename_priority_year")
        .ok()
        .flatten()
        .unwrap_or_else(|| "tag".to_string());
    let custom_pattern = get_setting(conn, "filename_custom_pattern")
        .ok()
        .flatten()
        .unwrap_or_default();
    let try_ampersand = get_setting(conn, "scan_try_parse_ampersand")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(true);

    let artist_tag_delimiters_owned: Vec<String> = get_setting(conn, "artist_tag_delimiters")
        .ok()
        .flatten()
        .map(|v| {
            v.split('|')
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_else(|| {
            ARTIST_TAG_DELIMITERS
                .iter()
                .map(|s| s.to_string())
                .collect()
        });
    let artist_tag_delimiters: Vec<&str> = artist_tag_delimiters_owned
        .iter()
        .map(|s| s.as_str())
        .collect();

    let artist_filename_delimiters_owned: Vec<String> =
        get_setting(conn, "artist_filename_delimiters")
            .ok()
            .flatten()
            .map(|v| {
                v.split('|')
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_else(|| {
                ARTIST_FILENAME_DELIMITERS
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            });
    let artist_filename_delimiters: Vec<&str> = artist_filename_delimiters_owned
        .iter()
        .map(|s| s.as_str())
        .collect();

    let genre_delimiters_owned: Vec<String> = get_setting(conn, "genre_delimiters")
        .ok()
        .flatten()
        .map(|v| {
            v.split('|')
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_else(|| GENRE_DELIMITERS.iter().map(|s| s.to_string()).collect());
    let genre_delimiters: Vec<&str> = genre_delimiters_owned.iter().map(|s| s.as_str()).collect();

    let all_files: Vec<_> = WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && is_supported(e.path()))
        .collect();

    let total = all_files.len();
    let mut scanned = 0;

    app.emit(
        "scan:progress",
        serde_json::json!({ "scanned": 0, "total": total }),
    )
    .ok();

    // Pre-load album and artist caches from existing DB records to avoid per-track full table scans
    let mut album_cache: AlbumCache = {
        let existing = get_all_albums(conn).unwrap_or_default();
        existing
            .into_iter()
            .filter_map(|a| {
                let id = a.id?;
                let title_lower = a.title.to_lowercase();
                let artist_lower = a
                    .album_artist
                    .as_ref()
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                let tracks_json = a.tracks.unwrap_or_else(|| "[]".to_string());
                Some((
                    (title_lower, artist_lower),
                    (id, a.uid, tracks_json, a.label),
                ))
            })
            .collect()
    };

    let mut artist_cache: ArtistCache = {
        let existing = get_all_artists(conn).unwrap_or_default();
        existing
            .into_iter()
            .filter_map(|a| {
                let id = a.id?;
                Some((a.name.to_lowercase(), (id, a.uid)))
            })
            .collect()
    };

    conn.execute_batch("BEGIN DEFERRED").ok();

    for entry in all_files {
        if let Some(track) = read_track_with_settings(
            entry.path(),
            &priority_title,
            &priority_artist,
            &priority_album,
            &priority_year,
            &custom_pattern,
            &artist_tag_delimiters,
            &artist_filename_delimiters,
            &genre_delimiters,
            try_ampersand,
        ) {
            if let Some(existing_uid) = find_remote_local_counterpart(conn, &track) {
                merge_paths_into_existing(conn, &existing_uid, &track);
                scanned += 1;
                if scanned % 25 == 0 || scanned == total {
                    app.emit(
                        "scan:progress",
                        serde_json::json!({ "scanned": scanned, "total": total }),
                    )
                    .ok();
                }
                continue;
            }

            if upsert_track(conn, &track).is_ok() {
                let actual_uid = if let Some(ref rp) = track.remote_path {
                    if !rp.is_empty() {
                        conn.query_row(
                            "SELECT uid FROM tracks WHERE remote_path = ?1",
                            rusqlite::params![rp],
                            |row| row.get::<_, String>(0),
                        )
                        .unwrap_or_else(|_| track.uid.clone())
                    } else {
                        track.uid.clone()
                    }
                } else {
                    track.uid.clone()
                };
                let track_for_process = crate::db::Track {
                    uid: actual_uid,
                    ..track.clone()
                };
                process_track_cached(
                    conn,
                    &track_for_process,
                    &mut album_cache,
                    &mut artist_cache,
                );

                if let Some(names) = track
                    .tags
                    .as_deref()
                    .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                {
                    for name in &names {
                        crate::db::tag_manager::ensure_tag(
                            conn,
                            name,
                            crate::db::tag_manager::TagKind::Tag,
                        );
                    }
                }
                if let Some(names) = track
                    .genres
                    .as_deref()
                    .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                {
                    for name in &names {
                        crate::db::tag_manager::ensure_tag(
                            conn,
                            name,
                            crate::db::tag_manager::TagKind::Genre,
                        );
                    }
                }
            }
        }

        scanned += 1;
        if scanned % 25 == 0 || scanned == total {
            app.emit(
                "scan:progress",
                serde_json::json!({ "scanned": scanned, "total": total }),
            )
            .ok();
        }
    }

    conn.execute_batch("COMMIT").ok();

    let duplicates = crate::db::find_duplicates(conn).unwrap_or_default();
    if !duplicates.is_empty() {
        app.emit("duplicates:found", duplicates.len()).ok();
    }

    app.emit("scan:done", ()).ok();
}