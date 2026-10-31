//! Embedded assets. `build.rs` embeds everything under assets/{sprites,music,sfx,fonts,data}
//! so the game ships as one binary. Look files up by path relative to assets/,
//! e.g. `assets::get("sprites/hero.png")`.

include!(concat!(env!("OUT_DIR"), "/assets_gen.rs"));

pub fn get(name: &str) -> Option<&'static [u8]> {
    ASSETS.iter().find(|(n, _)| *n == name).map(|(_, b)| *b)
}

pub fn get_str(name: &str) -> Option<&'static str> {
    get(name).and_then(|b| std::str::from_utf8(b).ok())
}

/// All embedded asset paths that start with `prefix` (e.g. "sfx/").
pub fn list(prefix: &str) -> impl Iterator<Item = &'static str> + '_ {
    ASSETS.iter().map(|(n, _)| *n).filter(move |n| n.starts_with(prefix))
}

/// Decode a PNG into (width, height, RGBA8 bytes).
pub fn decode_png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let n = (w * h) as usize;
    let src = &buf[..info.buffer_size()];
    let mut out = vec![0u8; n * 4];
    match info.color_type {
        png::ColorType::Rgba => out.copy_from_slice(&src[..n * 4]),
        png::ColorType::Rgb => {
            for i in 0..n {
                out[i * 4..i * 4 + 3].copy_from_slice(&src[i * 3..i * 3 + 3]);
                out[i * 4 + 3] = 255;
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for i in 0..n {
                let g = src[i * 2];
                out[i * 4..i * 4 + 4].copy_from_slice(&[g, g, g, src[i * 2 + 1]]);
            }
        }
        png::ColorType::Grayscale => {
            for i in 0..n {
                let g = src[i];
                out[i * 4..i * 4 + 4].copy_from_slice(&[g, g, g, 255]);
            }
        }
        png::ColorType::Indexed => return None,
    }
    Some((w, h, out))
}

/// Write an RGBA8 image to a PNG file (used by headless mode and tools).
pub fn write_png(path: &std::path::Path, w: u32, h: u32, rgba: &[u8]) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(std::io::Error::other)?;
    writer.write_image_data(rgba).map_err(std::io::Error::other)?;
    Ok(())
}
