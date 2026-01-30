use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub fn create_window(
    event_loop: &EventLoop<()>,
    width: u32,
    height: u32,
    position: Option<(i32, i32)>,
) -> Result<Window, winit::error::OsError> {
    let mut builder = WindowBuilder::new()
        .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
        .with_title("GifEngine")
        .with_decorations(false) // Borderless
        .with_transparent(true) // Transparent
        .with_visible(true);

    if let Some((x, y)) = position {
        builder = builder.with_position(winit::dpi::PhysicalPosition::new(x, y));
    }

    builder.build(event_loop)
}
