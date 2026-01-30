use crate::types::Frame;
use std::time::Duration;

pub struct FrameBuffer {
    frames: Vec<Frame>,
    current_index: usize,
}

impl FrameBuffer {
    pub fn new(frames: Vec<Frame>) -> Self {
        Self {
            frames,
            current_index: 0,
        }
    }

    pub fn next(&mut self) -> &Frame {
        if self.frames.is_empty() {
            panic!("FrameBuffer is empty");
        }
        let frame = &self.frames[self.current_index];
        self.current_index = (self.current_index + 1) % self.frames.len();
        frame
    }

    pub fn override_delay(&mut self, delay: Duration) {
        for frame in &mut self.frames {
            frame.delay = delay;
        }
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}
