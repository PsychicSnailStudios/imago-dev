use base64::{engine::general_purpose::STANDARD, Engine};
use image::{imageops::FilterType, ImageFormat};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Cursor;
use std::sync::{Mutex, OnceLock};

const MAX_CACHED: usize = 2048;

type ThumbCache = Mutex<HashMap<(u64, usize), Option<String>>>;

fn cache() -> &'static ThumbCache {
	static CACHE: OnceLock<ThumbCache> = OnceLock::new();
	CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn blob_key(blob: &[u8]) -> (u64, usize) {
	let mut hasher = DefaultHasher::new();
	blob.hash(&mut hasher);
	(hasher.finish(), blob.len())
}

fn render_thumb(blob: &[u8]) -> Option<String> {
	let img = image::load_from_memory(blob).ok()?;
	let thumb = img.resize_exact(16, 16, FilterType::Triangle);
	let mut buf = Cursor::new(Vec::new());
	thumb.write_to(&mut buf, ImageFormat::Jpeg).ok()?;
	Some(format!(
		"data:image/jpeg;base64,{}",
		STANDARD.encode(buf.get_ref())
	))
}

pub fn make_thumb(blob: &[u8]) -> Option<String> {
	let key = blob_key(blob);

	if let Ok(guard) = cache().lock() {
		if let Some(hit) = guard.get(&key) {
			return hit.clone();
		}
	}

	let result = render_thumb(blob);

	if let Ok(mut guard) = cache().lock() {
		if guard.len() >= MAX_CACHED {
			guard.clear();
		}
		guard.insert(key, result.clone());
	}

	result
}