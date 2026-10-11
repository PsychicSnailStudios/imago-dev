use crate::db::library_registry::get_all_libraries;
use crate::library_manager;
use crate::state::AppState;
use reqwest::{Client, Method};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::http::{Request, Response};
use tauri::{AppHandle, Manager, UriSchemeResponder};

const PROPFIND_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<propfind xmlns="DAV:"><prop><resourcetype/><getcontentlength/><getlastmodified/></prop></propfind>"#;

const STREAM_CHUNK: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct RemoteFile {
	pub url: String,
	pub rel_path: String,
	pub size: u64,
	pub modified: i64,
}

#[derive(Clone, Debug)]
struct DavEntry {
	url: String,
	decoded_path: String,
	is_dir: bool,
	size: u64,
	modified: i64,
}

pub fn is_http_url(s: &str) -> bool {
	let lower = s.trim_start().to_lowercase();
	lower.starts_with("http://") || lower.starts_with("https://")
}

pub fn client() -> &'static Client {
	static CLIENT: OnceLock<Client> = OnceLock::new();
	CLIENT.get_or_init(|| {
		Client::builder()
			.timeout(Duration::from_secs(120))
			.user_agent("Sonodex/1.0 (music library app)")
			.build()
			.expect("failed to build http client")
	})
}

pub fn percent_decode(input: &str) -> String {
	let bytes = input.as_bytes();
	let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
	let mut i = 0;
	while i < bytes.len() {
		if bytes[i] == b'%' && i + 2 < bytes.len() {
			let hi = (bytes[i + 1] as char).to_digit(16);
			let lo = (bytes[i + 2] as char).to_digit(16);
			if let (Some(h), Some(l)) = (hi, lo) {
				out.push((h * 16 + l) as u8);
				i += 3;
				continue;
			}
		}
		out.push(bytes[i]);
		i += 1;
	}
	String::from_utf8_lossy(&out).to_string()
}

fn encode_userinfo(s: &str) -> String {
	let mut out = String::new();
	for b in s.bytes() {
		if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
			out.push(b as char);
		} else {
			out.push_str(&format!("%{:02X}", b));
		}
	}
	out
}

pub fn url_with_basic_auth(url: &str, creds: Option<&str>) -> String {
	match creds.and_then(|c| c.split_once(':')) {
		Some((user, pass)) if is_http_url(url) => {
			let scheme_end = url.find("://").map(|i| i + 3).unwrap_or(0);
			format!(
				"{}{}:{}@{}",
				&url[..scheme_end],
				encode_userinfo(user),
				encode_userinfo(pass),
				&url[scheme_end..]
			)
		}
		_ => url.to_string(),
	}
}

pub fn encode_url_spaces(url: &str) -> String {
	url.trim().replace(' ', "%20")
}

fn origin_of(url: &str) -> String {
	let after_scheme = match url.find("://") {
		Some(i) => i + 3,
		None => return url.to_lowercase(),
	};
	match url[after_scheme..].find('/') {
		Some(i) => url[..after_scheme + i].to_lowercase(),
		None => url.to_lowercase(),
	}
}

fn path_of(url: &str) -> String {
	let after_scheme = match url.find("://") {
		Some(i) => i + 3,
		None => return String::new(),
	};
	let rest = &url[after_scheme..];
	let path = match rest.find('/') {
		Some(i) => &rest[i..],
		None => "/",
	};
	path.split(['?', '#']).next().unwrap_or("/").to_string()
}

fn xml_unescape(s: &str) -> String {
	s.replace("&amp;", "&")
		.replace("&lt;", "<")
		.replace("&gt;", ">")
		.replace("&quot;", "\"")
		.replace("&apos;", "'")
}

struct XmlTag<'a> {
	local: &'a str,
	closing: bool,
	start: usize,
	end: usize,
}

fn xml_tags(xml: &str) -> Vec<XmlTag<'_>> {
	let mut tags = Vec::new();
	let bytes = xml.as_bytes();
	let mut i = 0;
	while i < bytes.len() {
		if bytes[i] != b'<' {
			i += 1;
			continue;
		}
		let end = match xml[i..].find('>') {
			Some(e) => i + e + 1,
			None => break,
		};
		let inner = &xml[i + 1..end - 1];
		if inner.starts_with('?') || inner.starts_with('!') {
			i = end;
			continue;
		}
		let closing = inner.starts_with('/');
		let name_part = inner.trim_start_matches('/');
		let name = name_part
			.split(|c: char| c.is_whitespace() || c == '/')
			.next()
			.unwrap_or("");
		let local = name.rsplit(':').next().unwrap_or(name);
		tags.push(XmlTag {
			local,
			closing,
			start: i,
			end,
		});
		i = end;
	}
	tags
}

fn first_text(xml: &str, tags: &[XmlTag<'_>], local: &str) -> Option<String> {
	for t in tags {
		if !t.closing && t.local.eq_ignore_ascii_case(local) {
			let rest = &xml[t.end..];
			let stop = rest.find('<').unwrap_or(rest.len());
			let text = rest[..stop].trim();
			if text.is_empty() {
				return None;
			}
			return Some(xml_unescape(text));
		}
	}
	None
}

fn parse_multistatus(xml: &str, request_url: &str) -> Vec<DavEntry> {
	let tags = xml_tags(xml);
	let origin = origin_of(request_url);
	let mut entries = Vec::new();

	let mut idx = 0;
	while idx < tags.len() {
		let open = &tags[idx];
		if open.closing || !open.local.eq_ignore_ascii_case("response") {
			idx += 1;
			continue;
		}
		let mut close_idx = idx + 1;
		while close_idx < tags.len()
			&& !(tags[close_idx].closing && tags[close_idx].local.eq_ignore_ascii_case("response"))
		{
			close_idx += 1;
		}
		let seg_start = open.start;
		let seg_end = if close_idx < tags.len() {
			tags[close_idx].end
		} else {
			xml.len()
		};
		let segment = &xml[seg_start..seg_end];
		let seg_tags = xml_tags(segment);

		if let Some(href) = first_text(segment, &seg_tags, "href") {
			let is_dir = seg_tags
				.iter()
				.any(|t| !t.closing && t.local.eq_ignore_ascii_case("collection"));
			let size = first_text(segment, &seg_tags, "getcontentlength")
				.and_then(|s| s.parse::<u64>().ok())
				.unwrap_or(0);
			let modified = first_text(segment, &seg_tags, "getlastmodified")
				.and_then(|s| chrono::DateTime::parse_from_rfc2822(&s).ok())
				.map(|d| d.timestamp())
				.unwrap_or(0);

			let url = if is_http_url(&href) {
				href.clone()
			} else if href.starts_with('/') {
				format!("{}{}", origin, href)
			} else {
				format!("{}/{}", request_url.trim_end_matches('/'), href)
			};
			let decoded_path = percent_decode(&path_of(&url));

			entries.push(DavEntry {
				url,
				decoded_path,
				is_dir,
				size,
				modified,
			});
		}

		idx = close_idx + 1;
	}

	entries
}

pub fn auth(rb: reqwest::RequestBuilder, creds: Option<&str>) -> reqwest::RequestBuilder {
	library_manager::with_auth(rb, creds)
}

async fn propfind(url: &str, creds: Option<&str>) -> Result<Vec<DavEntry>, String> {
	let method = Method::from_bytes(b"PROPFIND").map_err(|e| e.to_string())?;
	let resp = auth(client().request(method, url), creds)
		.header("Depth", "1")
		.header("Content-Type", "application/xml; charset=utf-8")
		.body(PROPFIND_BODY)
		.send()
		.await
		.map_err(|e| format!("Could not reach {}: {}", url, e))?;

	let status = resp.status();
	if status.as_u16() == 401 || status.as_u16() == 403 {
		return Err(format!(
			"The server refused access ({}). Check the username and password stored with this library.",
			status
		));
	}
	if !(status.as_u16() == 207 || status.is_success()) {
		return Err(format!("Listing {} failed: {}", url, status));
	}

	let body = resp.text().await.map_err(|e| e.to_string())?;
	Ok(parse_multistatus(&body, url))
}

pub async fn collect_audio_files(
	root: &str,
	creds: Option<&str>,
	is_audio: &dyn Fn(&str) -> bool,
) -> Result<Vec<RemoteFile>, String> {
	let root = encode_url_spaces(root);
	let root_dir = format!("{}/", root.trim_end_matches('/'));
	let root_path = percent_decode(&path_of(&root_dir));
	let root_trim = root_path.trim_matches('/').to_string();
	let root_name = root_trim.rsplit('/').next().unwrap_or("").to_string();

	let mut queue: Vec<String> = vec![root_dir.clone()];
	let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
	let mut files: Vec<RemoteFile> = Vec::new();

	while let Some(dir_url) = queue.pop() {
		let dir_key = percent_decode(&path_of(&dir_url))
			.trim_end_matches('/')
			.to_lowercase();
		if !visited.insert(dir_key.clone()) {
			continue;
		}

		let entries = propfind(&dir_url, creds).await?;
		for entry in entries {
			let entry_key = entry.decoded_path.trim_end_matches('/').to_lowercase();
			if entry_key == dir_key {
				continue;
			}
			if entry.is_dir {
				let mut next = entry.url.clone();
				if !next.ends_with('/') {
					next.push('/');
				}
				queue.push(next);
				continue;
			}

			let name = entry.decoded_path.rsplit('/').next().unwrap_or("");
			if !is_audio(name) {
				continue;
			}

			let rel = entry
				.decoded_path
				.trim_matches('/')
				.strip_prefix(root_trim.as_str())
				.unwrap_or(entry.decoded_path.trim_matches('/'))
				.trim_matches('/')
				.to_string();
			let rel_path = if root_name.is_empty() {
				rel
			} else {
				format!("{}/{}", root_name, rel)
			};

			files.push(RemoteFile {
				url: entry.url,
				rel_path,
				size: entry.size,
				modified: entry.modified,
			});
		}
	}

	files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
	Ok(files)
}

pub async fn fetch_prefix(
	url: &str,
	creds: Option<&str>,
	max_bytes: Option<u64>,
) -> Result<Vec<u8>, String> {
	let mut rb = auth(client().get(url), creds);
	if let Some(max) = max_bytes {
		rb = rb.header("Range", format!("bytes=0-{}", max.saturating_sub(1)));
	}
	let mut resp = rb
		.send()
		.await
		.map_err(|e| format!("Download failed: {}", e))?;
	if !resp.status().is_success() {
		return Err(format!("Download failed: {}", resp.status()));
	}

	let mut data: Vec<u8> = Vec::new();
	while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
		data.extend_from_slice(&chunk);
		if let Some(max) = max_bytes {
			if data.len() as u64 >= max {
				data.truncate(max as usize);
				break;
			}
		}
	}
	Ok(data)
}

// ─── Credentials lookup ───────────────────────────────────────────────────────

fn credential_cache() -> &'static Mutex<HashMap<String, (Instant, Option<String>)>> {
	static CACHE: OnceLock<Mutex<HashMap<String, (Instant, Option<String>)>>> = OnceLock::new();
	CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn clear_credential_cache() {
	credential_cache().lock().unwrap().clear();
}

pub fn credentials_for_url(profile_uid: &str, url: &str) -> Option<String> {
	let origin = origin_of(url);
	let cache_key = format!("{}|{}", profile_uid, origin);

	if let Some((at, value)) = credential_cache().lock().unwrap().get(&cache_key) {
		if at.elapsed() < Duration::from_secs(30) {
			return value.clone();
		}
	}

	let settings = library_manager::open_tuned(crate::profiles::get_settings_db_path(profile_uid)).ok()?;
	let libs = get_all_libraries(&settings).ok()?;

	let mut found: Option<String> = None;
	for lib in &libs {
		let token = match lib.write_token.as_deref().map(|t| t.trim()).filter(|t| !t.is_empty()) {
			Some(t) => t.to_string(),
			None => continue,
		};
		if lib
			.sync_url
			.as_deref()
			.map(|u| is_http_url(u) && origin_of(u) == origin)
			.unwrap_or(false)
		{
			found = Some(token);
			break;
		}
		if let Ok(conn) = library_manager::open_tuned(&lib.file_path) {
			let has_path = crate::commands::paths::get_paths_from_lib_db(&conn)
				.unwrap_or_default()
				.iter()
				.any(|p| is_http_url(&p.path) && origin_of(&p.path) == origin);
			if has_path {
				found = Some(token);
				break;
			}
		}
	}

	credential_cache()
		.lock()
		.unwrap()
		.insert(cache_key, (Instant::now(), found.clone()));
	found
}

// ─── Streaming proxy (davstream:// protocol) ──────────────────────────────────

fn respond(responder: UriSchemeResponder, status: u16, body: Vec<u8>, extra: Vec<(&str, String)>) {
	let mut builder = Response::builder()
		.status(status)
		.header("Access-Control-Allow-Origin", "*")
		.header("Access-Control-Expose-Headers", "Content-Range, Content-Length, Accept-Ranges")
		.header("Accept-Ranges", "bytes");
	for (k, v) in extra {
		builder = builder.header(k, v);
	}
	match builder.body(body) {
		Ok(r) => responder.respond(r),
		Err(_) => responder.respond(
			Response::builder()
				.status(500)
				.body(Vec::<u8>::new())
				.unwrap(),
		),
	}
}

fn parse_range(header: &str) -> Option<(u64, Option<u64>)> {
	let spec = header.trim().strip_prefix("bytes=")?;
	let first = spec.split(',').next()?.trim();
	let (start, end) = first.split_once('-')?;
	let start: u64 = start.trim().parse().ok()?;
	let end: Option<u64> = if end.trim().is_empty() {
		None
	} else {
		end.trim().parse().ok()
	};
	Some((start, end))
}

async fn serve_range(
	target: String,
	creds: Option<String>,
	range: Option<(u64, Option<u64>)>,
) -> Result<(u16, Vec<u8>, Vec<(&'static str, String)>), String> {
	let (start, end) = match range {
		Some((s, e)) => (s, Some(e.unwrap_or(s + STREAM_CHUNK - 1).min(s + STREAM_CHUNK - 1))),
		None => (0, None),
	};

	let mut rb = auth(client().get(&target), creds.as_deref());
	if let Some(e) = end {
		rb = rb.header("Range", format!("bytes={}-{}", start, e));
	}
	let mut resp = rb.send().await.map_err(|e| e.to_string())?;
	let upstream = resp.status().as_u16();
	if upstream == 416 {
		return Ok((416, Vec::new(), Vec::new()));
	}
	if !resp.status().is_success() {
		return Ok((upstream, Vec::new(), Vec::new()));
	}

	let content_type = resp
		.headers()
		.get("content-type")
		.and_then(|v| v.to_str().ok())
		.unwrap_or("audio/mpeg")
		.to_string();
	let content_range = resp
		.headers()
		.get("content-range")
		.and_then(|v| v.to_str().ok())
		.map(|s| s.to_string());
	let content_length = resp
		.headers()
		.get("content-length")
		.and_then(|v| v.to_str().ok())
		.and_then(|s| s.parse::<u64>().ok());

	if upstream == 206 {
		let mut data = Vec::new();
		while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
			data.extend_from_slice(&chunk);
		}
		let mut headers = vec![("Content-Type", content_type)];
		if let Some(cr) = content_range {
			headers.push(("Content-Range", cr));
		}
		headers.push(("Content-Length", data.len().to_string()));
		return Ok((206, data, headers));
	}

	let wanted_end = end;
	let mut skipped: u64 = 0;
	let mut data: Vec<u8> = Vec::new();
	while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
		let mut slice: &[u8] = &chunk;
		if skipped < start {
			let need = (start - skipped) as usize;
			if slice.len() <= need {
				skipped += slice.len() as u64;
				continue;
			}
			slice = &slice[need..];
			skipped = start;
		}
		data.extend_from_slice(slice);
		if let Some(e) = wanted_end {
			let want = (e - start + 1) as usize;
			if data.len() >= want {
				data.truncate(want);
				break;
			}
		}
	}

	if range.is_some() {
		let total = content_length.unwrap_or(start + data.len() as u64);
		let last = start + data.len() as u64 - 1;
		let headers = vec![
			("Content-Type", content_type),
			("Content-Range", format!("bytes {}-{}/{}", start, last, total)),
			("Content-Length", data.len().to_string()),
		];
		Ok((206, data, headers))
	} else {
		let headers = vec![
			("Content-Type", content_type),
			("Content-Length", data.len().to_string()),
		];
		Ok((200, data, headers))
	}
}

pub fn handle_stream(app: AppHandle, request: Request<Vec<u8>>, responder: UriSchemeResponder) {
	let raw_path = request.uri().path().trim_start_matches('/').to_string();
	let target = percent_decode(&raw_path);
	if !is_http_url(&target) {
		respond(responder, 400, Vec::new(), Vec::new());
		return;
	}

	let range = request
		.headers()
		.get("range")
		.and_then(|v| v.to_str().ok())
		.and_then(parse_range);

	tauri::async_runtime::spawn(async move {
		let profile_uid = app.state::<AppState>().get_uid();
		let lookup_target = target.clone();
		let creds = tauri::async_runtime::spawn_blocking(move || {
			credentials_for_url(&profile_uid, &lookup_target)
		})
		.await
		.ok()
		.flatten();

		match serve_range(target, creds, range).await {
			Ok((status, body, headers)) => respond(responder, status, body, headers),
			Err(e) => {
				eprintln!("[davstream] {e}");
				respond(responder, 502, Vec::new(), Vec::new());
			}
		}
	});
}
