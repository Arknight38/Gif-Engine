use crate::cache::frame_buffer::FrameBuffer;
use crate::renderer::painter::Painter;
use crate::renderer::window::create_window;
use crate::app::process::ProcessStore;
use softbuffer::Context;
use std::rc::Rc;
use std::time::{Duration, Instant};
use winit::event::{Event, WindowEvent, ElementState, MouseButton};
use winit::event_loop::{ControlFlow, EventLoop};

use crate::platform;

pub fn play(
    mut frames: FrameBuffer,
    width: u32,
    height: u32,
    overlay: bool,
    click_through: bool,
    position: Option<(i32, i32)>,
    align: String,
    monitor_id: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    
    // Calculate position based on monitor and alignment
    let monitor = event_loop.available_monitors().nth(monitor_id)
        .or_else(|| event_loop.primary_monitor());

    let final_position = if let Some(monitor) = monitor {
        let m_pos = monitor.position();
        let m_size = monitor.size();
        
        // Ensure window fits or at least starts correctly
        // We use i32 for calculations
        let w = width as i32;
        let h = height as i32;
        let mw = m_size.width as i32;
        let mh = m_size.height as i32;
        
        let x = match align.as_str() {
            "top-left" | "bottom-left" => m_pos.x,
            "top-right" | "bottom-right" => m_pos.x + (mw - w),
            "center" => m_pos.x + (mw - w) / 2,
            "custom" => m_pos.x + position.map(|p| p.0).unwrap_or(0),
            _ => m_pos.x,
        };
        
        let y = match align.as_str() {
            "top-left" | "top-right" => m_pos.y,
            "bottom-left" | "bottom-right" => m_pos.y + (mh - h),
            "center" => m_pos.y + (mh - h) / 2,
            "custom" => m_pos.y + position.map(|p| p.1).unwrap_or(0),
            _ => m_pos.y,
        };
        
        Some((x, y))
    } else {
        position
    };

    let window = Rc::new(create_window(&event_loop, width, height, final_position)?);

    if overlay {
        platform::set_overlay(&window, click_through);
    }

    let context = Context::new(window.clone())?;
    let mut painter = Painter::new(&context, window.clone())?;

    let mut last_frame_time = Instant::now();
    let mut current_delay = Duration::from_millis(0);

    // State for manual dragging to avoid blocking the loop
    let mut is_dragging = false;
    let mut drag_start_mouse = (0.0, 0.0);
    let mut ctrl_was_pressed = false;

    println!("Starting event loop...");
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        // When click-through is enabled, poll for Ctrl key state globally in every frame
        // This allows us to temporarily disable click-through when Ctrl is held
        if click_through {
            let ctrl_pressed = platform::is_ctrl_pressed();
            
            // Only toggle click-through when Ctrl state changes to avoid flickering
            if ctrl_pressed != ctrl_was_pressed {
                platform::set_click_through(&window, !ctrl_pressed);
                ctrl_was_pressed = ctrl_pressed;
            }
        }

        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => {
                        println!("Close requested, exiting...");
                        let mut ps = ProcessStore::load();
                        ps.remove_self();
                        elwt.exit();
                    },
                    WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                        // If click-through is enabled, we only get mouse events when Ctrl is held
                        // (because we disable click-through when Ctrl is pressed)
                        // If click-through is disabled, always allow interaction
                        if !click_through || ctrl_was_pressed {
                            match state {
                                ElementState::Pressed => {
                                    is_dragging = true;
                                },
                                ElementState::Released => {
                                    is_dragging = false;
                                },
                            }
                        }
                    },
                    WindowEvent::CursorMoved { position, .. } => {
                        if is_dragging {
                            if let Ok(current_pos) = window.outer_position() {
                                let delta_x = position.x - drag_start_mouse.0;
                                let delta_y = position.y - drag_start_mouse.1;
                                
                                let new_pos = winit::dpi::PhysicalPosition::new(
                                    current_pos.x + delta_x as i32,
                                    current_pos.y + delta_y as i32,
                                );
                                window.set_outer_position(new_pos);
                            }
                        } else {
                            drag_start_mouse = (position.x, position.y);
                        }
                    },
                    _ => ()
                }
            }
            Event::AboutToWait => {
                if last_frame_time.elapsed() >= current_delay {
                    let frame = frames.next();
                    current_delay = frame.delay;
                    last_frame_time = Instant::now();

                    if let Err(e) = painter.paint(frame) {
                        eprintln!("Paint error: {}", e);
                    }
                }
            }
            _ => (),
        }
    })?;

    Ok(())
}
