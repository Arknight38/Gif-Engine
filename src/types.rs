use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Frame {
    pub buffer: Vec<u8>, // RGBA buffer
    pub width: u32,
    pub height: u32,
    pub delay: Duration,
}

#[derive(Debug)]
pub struct AnimationInfo {
    pub width: u16,
    pub height: u16,
    pub frame_count: usize,
    pub duration: Duration,
}
