use base64::{engine::general_purpose::STANDARD, Engine};
use image::{imageops::FilterType, ImageFormat};
use std::io::Cursor;

pub fn make_thumb(blob: &[u8]) -> Option<String> {
    let img = image::load_from_memory(blob).ok()?;
    let thumb = img.resize_exact(16, 16, FilterType::Triangle);
    let mut buf = Cursor::new(Vec::new());
    thumb.write_to(&mut buf, ImageFormat::Jpeg).ok()?;
    Some(format!(
        "data:image/jpeg;base64,{}",
        STANDARD.encode(buf.get_ref())
    ))
}
