use std::fs::File;
use std::path::Path;
use crate::types::{Frame, AnimationInfo};
use std::time::Duration;
use png::{DisposeOp, BlendOp};

pub fn load_apng<P: AsRef<Path>>(path: P) -> Result<(AnimationInfo, Vec<Frame>), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info()?;
    let info = reader.info().clone();
    
    // Check if it's an APNG
    if info.animation_control.is_none() {
        return Err("Not an APNG file".into());
    }

    let width = info.width;
    let height = info.height;
    
    let mut frames = Vec::new();
    let mut total_duration = Duration::from_secs(0);
    
    // Full canvas (RGBA)
    let mut canvas = vec![0u8; (width * height * 4) as usize];
    // Previous canvas for DisposeOp::Previous
    let mut previous_canvas = canvas.clone();
    
    // Track previous frame control for disposal
    let mut prev_fc: Option<png::FrameControl> = None;
    // Track region of previous frame for disposal
    let mut prev_rect: Option<(u32, u32, u32, u32)> = None;

    // Buffer for reading current frame chunk
    let mut buf = vec![0; reader.output_buffer_size()];
    
    // Iterate over frames
    while let Ok(frame_info) = reader.next_frame(&mut buf) {
        let buffer = buf[..frame_info.buffer_size()].to_vec();
        
        // Get current frame control
        let fc = reader.info().frame_control.unwrap(); // Should exist for APNG
        
        // 1. Dispose previous frame
        if let Some(p_fc) = prev_fc {
            match p_fc.dispose_op {
                DisposeOp::None => {
                    // Do nothing, keep canvas as is
                },
                DisposeOp::Background => {
                    // Clear the previous frame's region to transparent
                    if let Some((x, y, w, h)) = prev_rect {
                        clear_region(&mut canvas, width, x, y, w, h);
                    }
                },
                DisposeOp::Previous => {
                    // Restore to what it was before the previous frame
                    canvas.copy_from_slice(&previous_canvas);
                }
            }
        }

        // Save current canvas state if next frame needs to restore it
        if fc.dispose_op == DisposeOp::Previous {
            previous_canvas.copy_from_slice(&canvas);
        }

        // 2. Render current frame onto canvas
        let frame_data = convert_to_rgba(&buffer, fc.width, fc.height, info.color_type, info.bit_depth)?;
        blend_region(&mut canvas, width, &frame_data, fc.x_offset, fc.y_offset, fc.width, fc.height, fc.blend_op);

        // Store frame
        let delay_num = fc.delay_num;
        let delay_den = if fc.delay_den == 0 { 100 } else { fc.delay_den };
        let delay_secs = delay_num as f64 / delay_den as f64;
        let delay = Duration::from_secs_f64(delay_secs);
        total_duration += delay;

        frames.push(Frame {
            buffer: canvas.clone(),
            width,
            height,
            delay,
        });

        // Update tracking
        prev_fc = Some(fc);
        prev_rect = Some((fc.x_offset, fc.y_offset, fc.width, fc.height));

        if frames.len() >= info.animation_control.unwrap().num_frames as usize {
            break;
        }
    }

    let anim_info = AnimationInfo {
        width: width as u16,
        height: height as u16,
        frame_count: frames.len(),
        duration: total_duration,
    };

    Ok((anim_info, frames))
}

fn clear_region(canvas: &mut [u8], stride: u32, x: u32, y: u32, w: u32, h: u32) {
    for row in 0..h {
        let start = ((y + row) * stride + x) as usize * 4;
        let end = start + (w as usize * 4);
        // Zero out (transparent black)
        if start < canvas.len() && end <= canvas.len() {
             canvas[start..end].fill(0);
        }
    }
}

fn blend_region(canvas: &mut [u8], stride: u32, src: &[u8], x: u32, y: u32, w: u32, h: u32, blend: BlendOp) {
    for row in 0..h {
        let canvas_idx = ((y + row) * stride + x) as usize * 4;
        let src_idx = (row * w) as usize * 4;
        
        if canvas_idx + (w as usize * 4) > canvas.len() { continue; }
        
        for col in 0..w as usize {
            let c_ptr = canvas_idx + col * 4;
            let s_ptr = src_idx + col * 4;
            
            let sr = src[s_ptr];
            let sg = src[s_ptr + 1];
            let sb = src[s_ptr + 2];
            let sa = src[s_ptr + 3];

            match blend {
                BlendOp::Source => {
                    // Copy src to canvas (overwrite)
                    canvas[c_ptr] = sr;
                    canvas[c_ptr + 1] = sg;
                    canvas[c_ptr + 2] = sb;
                    canvas[c_ptr + 3] = sa;
                },
                BlendOp::Over => {
                    // Alpha blend: src OVER dst
                    // Result = Src * alpha + Dst * (1 - alpha)
                    let da = canvas[c_ptr + 3];
                    
                    // Optimization: if source is fully transparent, do nothing
                    if sa == 0 { continue; }
                    // If source is fully opaque, just copy
                    if sa == 255 {
                        canvas[c_ptr] = sr;
                        canvas[c_ptr + 1] = sg;
                        canvas[c_ptr + 2] = sb;
                        canvas[c_ptr + 3] = sa;
                        continue;
                    }
                    
                    // Simple integer alpha blending
                    let sa_f = sa as f32 / 255.0;
                    let da_f = da as f32 / 255.0;
                    let out_a = sa_f + da_f * (1.0 - sa_f);
                    
                    if out_a > 0.0 {
                        let dr = canvas[c_ptr];
                        let dg = canvas[c_ptr + 1];
                        let db = canvas[c_ptr + 2];
                        
                        let out_r = (sr as f32 * sa_f + dr as f32 * da_f * (1.0 - sa_f)) / out_a;
                        let out_g = (sg as f32 * sa_f + dg as f32 * da_f * (1.0 - sa_f)) / out_a;
                        let out_b = (sb as f32 * sa_f + db as f32 * da_f * (1.0 - sa_f)) / out_a;
                        
                        canvas[c_ptr] = out_r as u8;
                        canvas[c_ptr + 1] = out_g as u8;
                        canvas[c_ptr + 2] = out_b as u8;
                        canvas[c_ptr + 3] = (out_a * 255.0) as u8;
                    }
                }
            }
        }
    }
}

fn convert_to_rgba(buffer: &[u8], width: u32, height: u32, color_type: png::ColorType, bit_depth: png::BitDepth) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // This is a simplified conversion. 
    // Ideally use `image` crate to handle format conversions if possible, 
    // but we are working with raw buffers from `png` crate here.
    
    match (color_type, bit_depth) {
        (png::ColorType::Rgba, png::BitDepth::Eight) => Ok(buffer.to_vec()),
        (png::ColorType::Rgb, png::BitDepth::Eight) => {
            let mut rgba = Vec::with_capacity((width * height * 4) as usize);
            for chunk in buffer.chunks(3) {
                rgba.extend_from_slice(chunk);
                rgba.push(255);
            }
            Ok(rgba)
        },
        _ => Err(format!("Unsupported color type: {:?} {:?}", color_type, bit_depth).into()),
    }
}
