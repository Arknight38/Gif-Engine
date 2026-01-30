use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, Rgba};
use std::fs::File;
use std::time::Duration;

fn main() {
    let width = 100;
    let height = 100;

    let mut frames = Vec::new();

    for i in 0..10 {
        let mut buffer = image::ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 0]));

        // Draw a moving box
        for x in 0..20 {
            for y in 0..20 {
                let px = (i * 10 + x) % width;
                let py = (i * 10 + y) % height;
                buffer.put_pixel(px, py, Rgba([255, 0, 0, 255]));
            }
        }

        frames.push(Frame::from_parts(
            buffer,
            0,
            0,
            Delay::from_saturating_duration(Duration::from_millis(100)),
        ));
    }

    // Ensure directory exists
    std::fs::create_dir_all("test_assets").unwrap();
    let file = File::create("test_assets/sample.gif").unwrap();
    let mut encoder = GifEncoder::new(file);
    encoder.set_repeat(Repeat::Infinite).unwrap();
    encoder.encode_frames(frames.into_iter()).unwrap();

    println!("Generated test_assets/sample.gif");
}
