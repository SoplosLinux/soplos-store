// Remote icons and screenshots for the webview. Debian's are JPEG XL,
// which WebKitGTK cannot show, so they are decoded here and returned as
// PNG. Every image is cached on disk after the first download.

use base64::Engine;

use crate::{cache, http};

#[tauri::command]
pub async fn fetch_image(url: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || load(&url)).await.ok().flatten()
}

fn load(url: &str) -> Option<String> {
    if !url.starts_with("https://") {
        return None;
    }
    let is_jxl = url.ends_with(".jxl");
    let path = cache::dir("images").join(cache::key(url) + if is_jxl { ".png" } else { "" });
    let data = match std::fs::read(&path) {
        Ok(data) => data,
        Err(_) => {
            let raw = http::get(url, &[])?;
            let data = if is_jxl { jxl_to_png(&raw)? } else { raw };
            let _ = std::fs::write(&path, &data);
            data
        }
    };
    let mime = if is_jxl || url.ends_with(".png") {
        "image/png"
    } else if url.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "image/jpeg"
    };
    Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(data)))
}

fn jxl_to_png(bytes: &[u8]) -> Option<Vec<u8>> {
    let image = jxl_oxide::JxlImage::builder().read(bytes).ok()?;
    let render = image.render_frame(0).ok()?;
    let frame = render.image_all_channels();
    let (width, height, channels) = (frame.width() as u32, frame.height() as u32, frame.channels());
    if width == 0 || height == 0 || channels == 0 {
        return None;
    }
    let to_byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for px in frame.buf().chunks(channels) {
        // Grey images have one channel (two with alpha).
        let (r, g, b, a) = match channels {
            1 => (px[0], px[0], px[0], 1.0),
            2 => (px[0], px[0], px[0], px[1]),
            3 => (px[0], px[1], px[2], 1.0),
            _ => (px[0], px[1], px[2], px[3]),
        };
        rgba.extend_from_slice(&[to_byte(r), to_byte(g), to_byte(b), to_byte(a)]);
    }
    let buffer = image::RgbaImage::from_raw(width, height, rgba)?;
    let mut png = Vec::new();
    buffer.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).ok()?;
    Some(png)
}
