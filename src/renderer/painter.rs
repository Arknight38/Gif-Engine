use crate::types::Frame;
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::window::Window;

pub struct Painter {
    surface: Surface<Rc<Window>, Rc<Window>>,
}

impl Painter {
    pub fn new(
        context: &Context<Rc<Window>>,
        window: Rc<Window>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let surface = Surface::new(context, window)?;
        Ok(Self { surface })
    }

    pub fn paint(&mut self, frame: &Frame) -> Result<(), Box<dyn std::error::Error>> {
        let width = frame.width;
        let height = frame.height;

        if let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) {
            self.surface.resize(w, h)?;
        }

        let mut buffer = self.surface.buffer_mut()?;

        for (i, pixel) in buffer.iter_mut().enumerate() {
            let offset = i * 4;
            if offset + 3 < frame.buffer.len() {
                let r = frame.buffer[offset] as u32;
                let g = frame.buffer[offset + 1] as u32;
                let b = frame.buffer[offset + 2] as u32;
                let a = frame.buffer[offset + 3] as u32;

                // Premultiply alpha for correct blending with DWM
                // r_pre = (r * a) / 255
                let r_pre = (r * a) / 255;
                let g_pre = (g * a) / 255;
                let b_pre = (b * a) / 255;

                // 0xAARRGGBB
                *pixel = (a << 24) | (r_pre << 16) | (g_pre << 8) | b_pre;
            }
        }

        buffer.present()?;
        Ok(())
    }
}
