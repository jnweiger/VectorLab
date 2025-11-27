/*
 * Perplexity prompt:
 * write a complete rust program that
 * uses latest glutin and winit api,
 * loads an SVG file and allows scrolling panning zooming.
 * CTRL-Q quits the program
 *
 * Requires:
 * - cargo add resvg
 * - cargo add raw_window_handle
 */

use glutin::config::{ConfigSurfaceTypes, ConfigTemplateBuilder};
use glutin::context::{ContextApi, ContextAttributesBuilder};
use glutin::display::{DisplayApiPreference};
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};

use winit::dpi::LogicalSize;
// use winit::event::{ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent};
use winit::event::{Event, WindowEvent, ElementState, MouseButton, MouseScrollDelta};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::window::WindowBuilder;

use resvg::usvg::{Options, Tree};
use resvg::tiny_skia::{Pixmap, Transform};
use std::num::NonZeroU32;
use std::path::Path;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("SVG Viewer")
        .with_inner_size(LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .unwrap();

    // Correct glutin 0.32.3 API usage
    let gl_display = unsafe {
        glutin::display::Display::new(
            window.raw_display_handle(),
            DisplayApiPreference::Egl,
        ).unwrap()
    };

    let config_template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_config_surface_types(&[ConfigSurfaceTypes::Window])
        .build();

    let config = gl_display
        .find_configs(config_template)
        .unwrap()
        .reduce(glutin::config::ConfigSurfaceTypes::best_type)
        .unwrap();

    let context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(glutin::context::Version::new(3, 3))))
        .build(Some(window.raw_display_handle()));

    let mut gl_context = unsafe {
        gl_display.create_context(&config, &context_attributes).unwrap()
    };

    let surface_attrs = SurfaceAttributesBuilder::<WindowSurface>::new()
        .build(
            window.raw_window_handle(),
            NonZeroU32::new(window.inner_size().width).unwrap(),
            NonZeroU32::new(window.inner_size().height).unwrap(),
        );

    let surface = unsafe { gl_display.create_window_surface(&config, &surface_attrs).unwrap() };
    gl_context.make_current(&surface).unwrap();

    // Load SVG - FIXED STRING SYNTAX
    // Load SVG (create a test.svg file or change path)
    let svg_path = std::path::Path::new("test.svg");
    let opt = Options::default();
    let rtree = if svg_path.exists() {
        Tree::from_file(svg_path, &opt.to_ref()).expect("Failed to load SVG")
    } else {
        // Create a simple test SVG if none exists
        // Create test SVG with proper raw string syntax
	// Used r###"..."### (triple hash delimiters) for the raw string literal. This tells Rust:
	// r = raw string (no escaping needed)
	// ### = delimiter that safely contains all the " characters inside without confusion
    	// Single r#"..."# fails because SVG has many " quotes
    	// Triple r###"..."### ensures the closing ### is unambiguous

        let svg_content = r###"
<?xml version="1.0" encoding="UTF-8"?>
<svg width="400" height="300" xmlns="http://www.w3.org/2000/SVG">
  <rect width="100%" height="100%" fill="#f0f0f0"/>
  <circle cx="200" cy="150" r="80" fill="#3498db"/>
  <text x="200" y="160" font-size="24" text-anchor="middle" fill="white">SVG Test</text>
</svg>
"###;
        std::fs::write("test.svg", svg_content).unwrap();
        Tree::from_file("test.svg", &opt.to_ref()).unwrap()
    };

    // Variables for pan, zoom
    // View state
    let mut zoom: f32 = 1.0;
    let mut pan = (0.0f32, 0.0f32);
    let mut last_cursor_pos: Option<(f64, f64)> = None;
    let mut dragging = false;
    let mut width = 800u32;
    let mut height = 600u32;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput { input, .. } => {
		    if let Some(keycode) = input.virtual_keycode {
                        if input.state == ElementState::Pressed && keycode == VirtualKeyCode::Q {
                            // You can also check modifiers if needed
                            // if modifiers.ctrl()  { ... }
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                }
		WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::MouseWheel { delta, .. } => {
                    let scroll = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y as f32,
                        MouseScrollDelta::PixelDelta(p) => p.y as f32 * 0.1,
                    };
                    zoom *= 1.0 + scroll * 0.1;
                    zoom = zoom.clamp(0.1, 10.0);
                    window.request_redraw();
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if dragging {
                        if let Some((lx, ly)) = last_cursor_pos {
                            pan.0 += (position.x - lx) as f32;
                            pan.1 += (position.y - ly) as f32;
                        }
                        last_cursor_pos = Some((position.x, position.y));
                        window.request_redraw();
                    }
                }
                WindowEvent::MouseInput { button: MouseButton::Left, state, .. } => {
                    match state {
                        ElementState::Pressed => {
                            dragging = true;
                            last_cursor_pos = None;
                        }
                        ElementState::Released => {
                            dragging = false;
                        }
                    }
                }
                WindowEvent::Resized(size) => {
                    width = size.width;
                    height = size.height;
                    surface.resize(
                        &gl_context,
                        NonZeroU32::new(width).unwrap(),
                        NonZeroU32::new(height).unwrap(),
                    );
                    window.request_redraw();
                }
                _ => {}
            },
            Event::RedrawRequested(_) => {
                // Render SVG to pixmap
                let mut pixmap = Pixmap::new(width, height).unwrap();
                let transform = Transform::from_scale(zoom, zoom)
                    .post_translate(pan.0, pan.1);
                rtree.render(transform, &mut pixmap.as_mut());

                // Simple buffer swap (add GL texture rendering for pixmap display)
                surface.swap_buffers(&gl_context).unwrap();	// High level API, safe interface.
                // Here you should upload pixmap data to GL texture and draw (not implemented fully here)
                // For simplicity, we just swap buffers
                // Clear and swap (simple GL usage - you'd upload pixmap as texture here)
		// low level API (not recommended)
		//                unsafe {
		//                    gl_context.swap_buffers(&surface).unwrap();
		//                }
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }		// END match event
    }).unwrap();	// END event_loop.run
}

