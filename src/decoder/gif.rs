use crate::types::{AnimationInfo, Frame};
use image::AnimationDecoder;
use std::fs::File;
use std::path::Path;

pub fn load_gif<P: AsRef<Path>>(
    path: P,
) -> Result<(AnimationInfo, Vec<Frame>), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let decoder = image::codecs::gif::GifDecoder::new(file)?;

    // Get info before consuming frames?
    // GifDecoder doesn't expose frame count easily without iterating.

    let frames = decoder.into_frames().collect_frames()?;

    if frames.is_empty() {
        return Err("No frames found in GIF".into());
    }

    let width = frames[0].buffer().width();
    let height = frames[0].buffer().height();

    let mut result_frames = Vec::new();
    let mut total_duration = std::time::Duration::from_secs(0);

    for f in frames {
        let buffer = f.buffer();
        let delay: std::time::Duration = f.delay().into();
        total_duration += delay;

        result_frames.push(Frame {
            buffer: buffer.to_vec(), // Rgba8 buffer
            width: buffer.width(),
            height: buffer.height(),
            delay,
        });
    }

    let info = AnimationInfo {
        width: width as u16,
        height: height as u16,
        frame_count: result_frames.len(),
        duration: total_duration,
    };

    Ok((info, result_frames))
}
