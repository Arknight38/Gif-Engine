pub mod gif;
pub mod apng;
pub mod static_img;

use std::path::Path;
use crate::types::{Frame, AnimationInfo};

pub fn load_thumbnail<P: AsRef<Path>>(path: P) -> Result<(usize, usize, Vec<u8>), Box<dyn std::error::Error>> {
    // Open the file and attempt to guess format from bytes
    let img = image::io::Reader::open(path)?
        .with_guessed_format()?
        .decode()?;
    
    // Resize to max 200x200 for thumbnail, preserving aspect ratio
    let resized = img.resize(200, 200, image::imageops::FilterType::Lanczos3);
    
    let width = resized.width() as usize;
    let height = resized.height() as usize;
    let rgba = resized.to_rgba8().into_raw();
    
    Ok((width, height, rgba))
}

pub fn load_animation<P: AsRef<Path>>(path: P) -> Result<(AnimationInfo, Vec<Frame>), Box<dyn std::error::Error>> {
    let path = path.as_ref();
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    match ext.as_str() {
        "gif" => gif::load_gif(path),
        "png" | "apng" => {
            // Try loading as APNG first
            match apng::load_apng(path) {
                Ok(res) => Ok(res),
                // If it fails (e.g. not an APNG), try loading as static image
                Err(_) => static_img::load_static(path),
            }
        },
        _ => static_img::load_static(path),
    }
}
