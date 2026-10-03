use crate::library_manager;
use crate::state::AppState;
use image::{imageops::FilterType, DynamicImage, ImageFormat};
use rusqlite::Connection;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use tauri::http::{Request, Response};
use tauri::{AppHandle, Manager, UriSchemeResponder};

const WORKERS: usize = 4;
const CARD_SIZE: u32 = 300;
const LARGE_SIZE: u32 = 800;

type Job = Box<dyn FnOnce() + Send + 'static>;

struct ArtworkRequest {
	kind: String,
	uid: String,
	size: u32,
	version: String,
}

fn pool() -> &'static mpsc::Sender<Job> {
	static POOL: OnceLock<mpsc::Sender<Job>> = OnceLock::new();
	POOL.get_or_init(|| {
		let (tx, rx) = mpsc::channel::<Job>();
		let rx = Arc::new(Mutex::new(rx));
		for _ in 0..WORKERS {
			let rx = Arc::clone(&rx);
			std::thread::spawn(move || loop {
				let job = match rx.lock() {
					Ok(guard) => guard.recv(),
					Err(_) => break,
				};
				match job {
					Ok(job) => {
						let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
					}
					Err(_) => break,
				}
			});
		}
		tx
	})
}

fn is_safe(value: &str) -> bool {
	!value.is_empty()
		&& value.len() < 128
		&& value
			.chars()
			.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn parse_request(request: &Request<Vec<u8>>) -> Option<ArtworkRequest> {
	let uri = request.uri();
	let path = uri
		.path()
		.trim_matches('/')
		.replace("%2F", "/")
		.replace("%2f", "/");
	let (kind, uid) = path.split_once('/')?;

	if !matches!(kind, "track" | "album" | "artist" | "banner" | "playlist") || !is_safe(uid) {
		return None;
	}

	let mut size = CARD_SIZE;
	let mut version = String::from("0");

	if let Some(query) = uri.query() {
		for pair in query.split('&') {
			if let Some((key, value)) = pair.split_once('=') {
				match key {
					"s" => {
						size = if value == "large" {
							LARGE_SIZE
						} else {
							CARD_SIZE
						}
					}
					"v" if is_safe(value) => version = value.to_string(),
					_ => {}
				}
			}
		}
	}

	Some(ArtworkRequest {
		kind: kind.to_string(),
		uid: uid.to_string(),
		size,
		version,
	})
}

fn from_source<F>(profile_uid: &str, uid: &str, entity: &str, read: F) -> Option<Vec<u8>>
where
	F: Fn(&Connection, &str) -> rusqlite::Result<Option<Vec<u8>>>,
{
	let blob = match library_manager::open_source_conn_for_entity(profile_uid, uid, entity) {
		Ok((conn, _)) => read(&conn, uid).ok().flatten(),
		Err(_) => {
			let conn = crate::open_merged_conn(profile_uid);
			read(&conn, uid).ok().flatten()
		}
	};
	blob.filter(|bytes| !bytes.is_empty())
}

fn load_blob(profile_uid: &str, req: &ArtworkRequest) -> Option<Vec<u8>> {
	let uid = req.uid.as_str();
	match req.kind.as_str() {
		"track" => from_source(profile_uid, uid, "tracks", crate::db::get_track_artwork_by_uid),
		"album" => from_source(profile_uid, uid, "albums", crate::db::get_album_artwork_by_uid),
		"artist" => from_source(
			profile_uid,
			uid,
			"artists",
			crate::db::get_artist_profile_art_by_uid,
		),
		"banner" => from_source(
			profile_uid,
			uid,
			"artists",
			crate::db::get_artist_banner_art_by_uid,
		),
		"playlist" => {
			let conn = crate::commands::playlists::open_best_playlists_conn(profile_uid);
			crate::db::get_playlist_artwork_by_uid(&conn, uid)
				.ok()
				.flatten()
				.filter(|bytes| !bytes.is_empty())
		}
		_ => None,
	}
}

fn render(blob: &[u8], size: u32) -> Option<Vec<u8>> {
	let img = image::load_from_memory(blob).ok()?;
	let img = if img.width() > size || img.height() > size {
		img.resize(size, size, FilterType::CatmullRom)
	} else {
		img
	};
	let rgb = DynamicImage::ImageRgb8(img.to_rgb8());
	let mut buf = Cursor::new(Vec::new());
	rgb.write_to(&mut buf, ImageFormat::Jpeg).ok()?;
	Some(buf.into_inner())
}

fn cache_root(profile_uid: &str) -> PathBuf {
	crate::profiles::get_artwork_cache_dir(profile_uid)
}

fn prune_stale(dir: &Path, version: &str) {
	let prefix = format!("{}_", version);
	if let Ok(entries) = std::fs::read_dir(dir) {
		for entry in entries.flatten() {
			if !entry.file_name().to_string_lossy().starts_with(&prefix) {
				let _ = std::fs::remove_file(entry.path());
			}
		}
	}
}

fn write_cache(dir: &Path, file: &Path, bytes: &[u8], version: &str) {
	static COUNTER: AtomicU64 = AtomicU64::new(0);
	prune_stale(dir, version);
	if std::fs::create_dir_all(dir).is_err() {
		return;
	}
	let tmp = file.with_extension(format!("{}.tmp", COUNTER.fetch_add(1, Ordering::Relaxed)));
	if std::fs::write(&tmp, bytes).is_ok() {
		if std::fs::rename(&tmp, file).is_err() {
			let _ = std::fs::remove_file(&tmp);
		}
	}
}

fn serve(profile_uid: &str, req: &ArtworkRequest) -> Option<Vec<u8>> {
	let cacheable = req.version != "0";
	let dir = cache_root(profile_uid).join(&req.kind).join(&req.uid);
	let file = dir.join(format!("{}_{}.jpg", req.version, req.size));

	if cacheable {
		if let Ok(bytes) = std::fs::read(&file) {
			return Some(bytes);
		}
	}

	let blob = load_blob(profile_uid, req)?;
	let rendered = render(&blob, req.size)?;

	if cacheable {
		write_cache(&dir, &file, &rendered, &req.version);
	}

	Some(rendered)
}

fn respond(responder: UriSchemeResponder, status: u16, body: Vec<u8>, cacheable: bool) {
	let cache_control = if cacheable {
		"public, max-age=31536000, immutable"
	} else {
		"no-cache"
	};
	let response = Response::builder()
		.status(status)
		.header("Content-Type", "image/jpeg")
		.header("Cache-Control", cache_control)
		.header("Access-Control-Allow-Origin", "*")
		.body(body);
	if let Ok(response) = response {
		responder.respond(response);
	}
}

pub fn handle(app: AppHandle, request: Request<Vec<u8>>, responder: UriSchemeResponder) {
	let req = match parse_request(&request) {
		Some(req) => req,
		None => {
			respond(responder, 400, Vec::new(), false);
			return;
		}
	};

	let job: Job = Box::new(move || {
		let profile_uid = app.state::<AppState>().get_uid();
		let cacheable = req.version != "0";
		match serve(&profile_uid, &req) {
			Some(bytes) => respond(responder, 200, bytes, cacheable),
			None => respond(responder, 404, Vec::new(), false),
		}
	});

	let _ = pool().send(job);
}

#[tauri::command]
pub fn clear_artwork_cache(state: tauri::State<AppState>) -> Result<(), String> {
	let dir = cache_root(&state.get_uid());
	if dir.exists() {
		std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
	}
	Ok(())
}