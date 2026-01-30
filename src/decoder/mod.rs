pub mod gif;
pub mod apng;

use std::path::Path;
use crate::types::{Frame, AnimationInfo};

pub fn load_animation<P: AsRef<Path>>(path: P) -> Result<(AnimationInfo, Vec<Frame>), Box<dyn std::error::Error>> {
    let path = path.as_ref();
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    match ext.as_str() {
        "gif" => gif::load_gif(path),
        "png" | "apng" => apng::load_apng(path),
        _ => Err("Unsupported file format".into()),
    }
}
