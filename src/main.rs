#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cache;
mod decoder;
mod renderer;
mod playback;
mod platform;
mod tui;
mod gui;
pub mod types;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Instant;
use image::{imageops::FilterType, ImageBuffer, Rgba};

use crate::cache::frame_buffer::FrameBuffer;

#[derive(Parser)]
#[command(name = "gif-engine")]
#[command(about = "Gif-Engine Player", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show GIF metadata
    Info {
        /// Path to GIF file
        file: PathBuf,
    },
    /// Play animation on desktop
    Play {
        /// Path to GIF file
        file: PathBuf,
        
        /// Target FPS (overrides GIF delay)
        #[arg(long)]
        fps: Option<u32>,

        /// Scale factor (0.1 to 1.0)
        #[arg(long)]
        scale: Option<f32>,

        /// X Position
        #[arg(long)]
        x: Option<i32>,

        /// Y Position
        #[arg(long)]
        y: Option<i32>,

        /// Enable overlay mode (always-on-top, click-through)
        #[arg(long)]
        overlay: bool,

        /// Alignment (top-left, top-right, bottom-left, bottom-right, center, custom)
        #[arg(long, default_value = "center")]
        align: String,

        /// Monitor Index (0, 1, 2...)
        #[arg(long, default_value_t = 0)]
        monitor: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        None => {
            // Run GUI by default
            if let Err(e) = gui::run_gui() {
                eprintln!("GUI Error: {}", e);
            }
        }
        Some(Commands::Info { file }) => {
            let start = Instant::now();
            println!("Loading {:?}", file);
            
            match decoder::load_animation(file) {
                Ok((info, frames)) => {
                    let duration = start.elapsed();
                    println!("Loaded in {:.2?}", duration);
                    println!("Dimensions: {}x{}", info.width, info.height);
                    println!("Frame count: {}", info.frame_count);
                    println!("Total duration: {:.2?}", info.duration);

                    let fps_decode = info.frame_count as f64 / duration.as_secs_f64();
                    println!("Decode speed: {:.2} fps", fps_decode);

                    let buffer = FrameBuffer::new(frames);
                    println!("FrameBuffer initialized with {} frames", buffer.len());
                }
                Err(e) => {
                    eprintln!("Error loading GIF: {}", e);
                }
            }
        }
        Some(Commands::Play { file, fps, scale, x, y, overlay, align, monitor }) => {
            println!("Playing {:?}", file);
            match decoder::load_animation(file) {
                Ok((mut info, mut frames)) => {
                    if let Some(s) = scale {
                        if *s > 0.0 && (s.abs() - 1.0).abs() > f32::EPSILON {
                            println!("Resizing by factor {}", s);
                            let new_width = (info.width as f32 * s) as u32;
                            let new_height = (info.height as f32 * s) as u32;
                            
                            for frame in &mut frames {
                                let old_buffer = std::mem::take(&mut frame.buffer);
                                if let Some(img) = ImageBuffer::<Rgba<u8>, _>::from_raw(frame.width, frame.height, old_buffer) {
                                    let resized = image::imageops::resize(&img, new_width, new_height, FilterType::Lanczos3);
                                    frame.width = resized.width();
                                    frame.height = resized.height();
                                    frame.buffer = resized.into_raw();
                                }
                            }
                            info.width = new_width as u16;
                            info.height = new_height as u16;
                        }
                    }

                    let mut buffer = FrameBuffer::new(frames);

                    if let Some(target_fps) = fps {
                        let delay = std::time::Duration::from_secs_f64(1.0 / (*target_fps as f64));
                        buffer.override_delay(delay);
                        println!("Overriding FPS to {}", target_fps);
                    }

                    if let Err(e) = playback::player::play(
                        buffer,
                        info.width as u32,
                        info.height as u32,
                        *overlay,
                        x.zip(*y),
                        align.clone(),
                        *monitor,
                    ) {
                        eprintln!("Playback error: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Error loading GIF: {}", e);
                }
            }
        }
    }
}
