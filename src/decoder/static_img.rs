use crate::types::{AnimationInfo, Frame};
use std::path::Path;
use std::time::Duration;

pub fn load_static<P: AsRef<Path>>(path: P) -> Result<(AnimationInfo, Vec<Frame>), Box<dyn std::error::Error>> {
    let img = image::io::Reader::open(path)?
        .with_guessed_format()?
        .decode()?;
    
    let rgba = img.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    
    let frame = Frame {
        buffer: rgba.into_raw(),
        width,
        height,
        delay: Duration::from_secs(1),
    };
    
    let info = AnimationInfo {
        width: width as u16,
        height: height as u16,
        frame_count: 1,
        duration: Duration::from_secs(1),
    };
    
    Ok((info, vec![frame]))
}
