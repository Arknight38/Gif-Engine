use crate::types::{Frame, AnimationInfo};
use eframe::egui;

pub struct PreviewState {
    pub frames: Vec<Frame>,
    pub info: AnimationInfo,
    pub current_frame: usize,
    pub last_update: std::time::Instant,
    pub texture: Option<egui::TextureHandle>,
    pub override_delay: Option<std::time::Duration>,
}

impl PreviewState {
    pub fn new(frames: Vec<Frame>, info: AnimationInfo) -> Self {
        Self {
            frames,
            info,
            current_frame: 0,
            last_update: std::time::Instant::now(),
            texture: None,
            override_delay: None,
        }
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        if self.frames.is_empty() {
            return;
        }

        let delay = self.override_delay.unwrap_or(self.frames[self.current_frame].delay);

        if self.last_update.elapsed() >= delay {
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            
            // Accumulate time to prevent drift
            self.last_update += delay;
            
            // If we've drifted too far (e.g. system sleep or heavy lag), reset
            if self.last_update.elapsed() > delay {
                self.last_update = std::time::Instant::now();
            }
            
            // Update texture
            let frame = &self.frames[self.current_frame];
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [frame.width as usize, frame.height as usize],
                &frame.buffer,
            );
            
            if let Some(texture) = &mut self.texture {
                texture.set(image, egui::TextureOptions::LINEAR);
            } else {
                self.texture = Some(ctx.load_texture(
                    "preview_tex",
                    image,
                    egui::TextureOptions::LINEAR
                ));
            }
            ctx.request_repaint(); // Ensure continuous animation
        } else {
            // Request repaint for smooth timing even if we didn't update frame yet
            // but use delay to avoid busy loop
            let remaining = delay.saturating_sub(self.last_update.elapsed());
            ctx.request_repaint_after(remaining);
        }
    }
}

