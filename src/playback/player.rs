use crate::cache::frame_buffer::FrameBuffer;
use crate::renderer::painter::Painter;
use crate::renderer::window::create_window;
use crate::app::process::ProcessStore;
use softbuffer::Context;
use std::process::Command;
use std::env;
use std::rc::Rc;
use std::time::{Duration, Instant};
use winit::event::{Event, WindowEvent, ElementState, MouseButton};
use winit::event_loop::{ControlFlow, EventLoop};

use crate::platform;
use rand::Rng;

pub fn play(
    mut frames: FrameBuffer,
    width: u32,
    height: u32,
    overlay: bool,
    mut click_through: bool,
    position: Option<(i32, i32)>,
    align: String,
    monitor_id: usize,
    anchor: Option<String>,
    behavior: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    
    // Calculate position based on monitor and alignment
    let monitor = event_loop.available_monitors().nth(monitor_id)
        .or_else(|| event_loop.primary_monitor());
        
    let monitor_rect = if let Some(m) = &monitor {
        let p = m.position();
        let s = m.size();
        (p.x, p.y, s.width as i32, s.height as i32)
    } else {
        (0, 0, 1920, 1080)
    };

    let final_position = if let Some(monitor) = &monitor {
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
    let mut paused = false;

    // State for interaction
    let mut last_mouse_pos = (0.0, 0.0);
    let mut interactive_was_active = false;
    
    // Anchoring State
    // If anchored, position is treated as offset from anchor top-left.
    // If no position provided, default to (0,0) offset.
    let mut anchor_offset = if let Some((x, y)) = position { (x, y) } else { (0, 0) };
    let mut last_anchor_pos = None;
    let mut anchor_hwnd: Option<isize> = None;
    let mut current_owner_hwnd: Option<isize> = None;

    // Behavior State
    let mut velocity: (f32, f32) = match behavior.as_str() {
        "bounce" => (3.0, 3.0),
        "wander" => (0.0, 0.0), // Start still, accelerate
        _ => (0.0, 0.0),
    };
    let mut last_behavior_update = Instant::now();

    println!("Starting event loop...");
    event_loop.run(move |event, elwt| {
        // Default to waiting until next frame deadline (much lower CPU than Poll).
        // If click-through is enabled we still periodically wake to poll key states.
        let now = Instant::now();
        let next_frame_deadline = last_frame_time + current_delay;
        
        // Use faster polling if anchored to prevent lag/stutter when moving parent window
        let poll_interval = if anchor.is_some() {
            Duration::from_millis(16) // ~60 FPS
        } else {
            Duration::from_millis(50) // ~20 FPS for just hotkey polling
        };
        
        let next_wakeup = if paused {
             now + poll_interval
        } else if click_through || anchor.is_some() {
            std::cmp::min(next_frame_deadline, now + poll_interval)
        } else {
            next_frame_deadline
        };
        elwt.set_control_flow(ControlFlow::WaitUntil(next_wakeup));

        // Poll for Anchoring
        if let Some(target_title) = &anchor {
             let mut rect = None;
             
             // Try cached HWND first
             if let Some(hwnd) = anchor_hwnd {
                 rect = platform::get_window_rect_by_hwnd(hwnd);
                 if rect.is_none() {
                     // Window might have closed, invalidate cache
                     anchor_hwnd = None;
                 }
             }
             
             // If no rect yet, try to find by title
             if rect.is_none() {
                 if let Some(hwnd) = platform::get_hwnd_by_title(target_title) {
                     anchor_hwnd = Some(hwnd);
                     rect = platform::get_window_rect_by_hwnd(hwnd);
                 }
             }

             if let Some((ax, ay, aw, ah)) = rect {
                 let current_anchor_pos = (ax, ay, aw, ah);
                 
                 // Apply ownership if changed
                 if let Some(hwnd) = anchor_hwnd {
                     if current_owner_hwnd != Some(hwnd) {
                         platform::set_window_owner(&window, hwnd);
                         // Disable global topmost when anchored so we stay relative to owner
                         platform::set_topmost(&window, false);
                         current_owner_hwnd = Some(hwnd);
                     }
                 }

                 // If target moved, update our position
                 if last_anchor_pos != Some(current_anchor_pos) {
                     let new_x = ax + anchor_offset.0;
                     let new_y = ay + anchor_offset.1;
                     window.set_outer_position(winit::dpi::PhysicalPosition::new(new_x, new_y));
                     last_anchor_pos = Some(current_anchor_pos);
                 }
             } else {
                 // Lost anchor (window closed)
                 if current_owner_hwnd.is_some() {
                     platform::set_window_owner(&window, 0);
                     current_owner_hwnd = None;
                     // Restore topmost if overlay mode was enabled
                     if overlay {
                         platform::set_topmost(&window, true);
                     }
                 }
             }
        }

        // Poll for Ctrl or Alt key state
        // Ctrl: Interactive mode (click, drag)
        // Alt: Drag mode (and interactive)
        if click_through {
            let interactive = platform::is_ctrl_pressed() || platform::is_alt_pressed();
            
            // Only toggle click-through when state changes to avoid flickering
            if interactive != interactive_was_active {
                platform::set_click_through(&window, !interactive);
                interactive_was_active = interactive;
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
                    WindowEvent::Moved(new_pos) => {
                        // If we moved (e.g. dragged), update the anchor offset
                        if let Some(target_title) = &anchor {
                            if let Some((ax, ay, _, _)) = platform::get_window_rect_by_title(target_title) {
                                anchor_offset = (new_pos.x - ax, new_pos.y - ay);
                            }
                        }
                    },
                    WindowEvent::MouseInput { state, button, .. } => {
                        // Allow interaction if click-through is disabled OR if we are in interactive mode (Ctrl/Alt held)
                        if !click_through || interactive_was_active {
                            match (button, state) {
                                (MouseButton::Left, ElementState::Pressed) => {
                                    // Use winit's built-in drag which handles the OS specific window dragging
                                    if let Err(e) = window.drag_window() {
                                        eprintln!("Failed to start drag: {}", e);
                                    }
                                },
                                (MouseButton::Right, ElementState::Released) => {
                                    // Show context menu
                                    if let Ok(window_pos) = window.outer_position() {
                                        let screen_x = window_pos.x + last_mouse_pos.0 as i32;
                                        let screen_y = window_pos.y + last_mouse_pos.1 as i32;
                                        
                                        if let Some(cmd_id) = platform::show_context_menu(&window, screen_x, screen_y, paused, click_through) {
                                            match cmd_id {
                                                platform::ID_PAUSE => {
                                                    paused = !paused;
                                                    // Reset frame time to avoid skipping when resuming
                                                    last_frame_time = Instant::now();
                                                },
                                                platform::ID_STOP => {
                                                    let mut ps = ProcessStore::load();
                                                    ps.remove_self();
                                                    elwt.exit();
                                                },
                                                platform::ID_TOGGLE_CLICKTHROUGH => {
                                                    click_through = !click_through;
                                                    platform::set_click_through(&window, click_through);
                                                    // Force a repaint or style update if needed
                                                    // If enabling click-through, we might need to reset interactive state
                                                    if click_through {
                                                        interactive_was_active = false;
                                                    }
                                                },
                                                platform::ID_SETTINGS => {
                                                    println!("Open Settings requested");
                                                    if !platform::focus_window("Gif-Engine Manager") {
                                                        if let Ok(exe_path) = env::current_exe() {
                                                            // Launch the main manager (no args)
                                                            if let Err(e) = Command::new(exe_path).spawn() {
                                                                eprintln!("Failed to launch manager: {}", e);
                                                            }
                                                        }
                                                    }
                                                },
                                                _ => {}
                                            }
                                        }
                                    }
                                },
                                _ => {}
                            }
                        }
                    },
                    WindowEvent::CursorMoved { position, .. } => {
                        last_mouse_pos = (position.x, position.y);
                    },
                    _ => ()
                }
            }
            Event::AboutToWait => {
                let now = Instant::now();
                
                // Behavior Update
                if behavior != "none" && !paused && anchor.is_none() {
                     // Update at ~60Hz
                     if now.duration_since(last_behavior_update).as_millis() >= 16 {
                         if let Ok(pos) = window.outer_position() {
                             let mut x = pos.x as i32;
                             let mut y = pos.y as i32;
                             let w = width as i32;
                             let h = height as i32;
                             let (mx, my, mw, mh) = monitor_rect;
                             
                             if behavior == "bounce" {
                                 x += velocity.0 as i32;
                                 y += velocity.1 as i32;
                                 
                                 // Bounce checks
                                 if x <= mx { velocity.0 = velocity.0.abs(); x = mx; }
                                 if x + w >= mx + mw { velocity.0 = -velocity.0.abs(); x = mx + mw - w; }
                                 if y <= my { velocity.1 = velocity.1.abs(); y = my; }
                                 if y + h >= my + mh { velocity.1 = -velocity.1.abs(); y = my + mh - h; }
                                 
                             } else if behavior == "wander" {
                                 // Randomly change velocity occasionally
                                 if rand::thread_rng().gen_bool(0.05) {
                                     velocity.0 += rand::thread_rng().gen_range(-1.0..=1.0);
                                     velocity.1 += rand::thread_rng().gen_range(-1.0..=1.0);
                                     // Cap velocity
                                     velocity.0 = velocity.0.clamp(-4.0, 4.0);
                                     velocity.1 = velocity.1.clamp(-4.0, 4.0);
                                 }
                                 
                                 x += velocity.0 as i32;
                                 y += velocity.1 as i32;
                                 
                                 // Keep in bounds
                                 if x < mx { velocity.0 = velocity.0.abs(); }
                                 if x + w > mx + mw { velocity.0 = -velocity.0.abs(); }
                                 if y < my { velocity.1 = velocity.1.abs(); }
                                 if y + h > my + mh { velocity.1 = -velocity.1.abs(); }
                             }
                             
                             window.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
                         }
                         last_behavior_update = now;
                     }
                }

                if !paused && now.duration_since(last_frame_time) >= current_delay {
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
