/*
 * Perplexity prompt:
 * write a complete rust program that
 * uses latest glutin and winit api,
 * loads an SVG file and allows scrolling panning zooming.
 * CTRL-Q quits the program
 *
 * Requires:
 * - cargo add resvg			# resvg is the rendering library that takes the usvg::Tree and rasterizes it
 * - cargo add usvg			# usvg usvg is the SVG parsing and tree management library
 * - cargo add raw_window_handle
 */
use glutin::config::{ConfigSurfaceTypes, ConfigTemplateBuilder};
use glutin::context::{ContextApi, ContextAttributesBuilder};
use glutin::display::{DisplayApiPreference};
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};

use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent, ElementState, MouseButton, MouseScrollDelta};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::window::WindowBuilder;

// This resolves the E0599 error because raw_display_handle() is provided by the
// trait, which must be explicitly imported for the method to be visible on
// Window.
//
// Alternatively, upgrade to a recent winit version (e.g., 0.30+) and replace
// window.raw_display_handle() with window.display_handle(), as suggested by the
// compiler hint—this uses the newer API directly without needing the trait
// import. Note that glutin 0.32.3 pairs best with older winit like 0.29
use winit::raw_window_handle::HasRawDisplayHandle;
use winit::raw_window_handle::HasRawWindowHandle;


use resvg::usvg::{Options};
use resvg::tiny_skia::{Pixmap, Transform};

use usvg::Tree;

use std::num::NonZeroU32;
use std::path::Path;
use std::fs;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // complex signature above to allow using the ? operator.
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("SVG Viewer")
        .with_inner_size(LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .unwrap();

    // Correct glutin 0.32.3 API usage
    let gl_display = unsafe {
        glutin::display::Display::new(
            window.raw_display_handle().expect("Failed to get raw display handle"),
            DisplayApiPreference::Egl,
        ).unwrap()
    };

    let config_template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_surface_type(ConfigSurfaceTypes::WINDOW)
        .build();

    let config = gl_display
        .find_configs(config_template)
        .unwrap()
        .next()		// find_configs() returns an iterator over matching Configs, .next() gets the first (and typically only/best) one
        .unwrap();

    let context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(glutin::context::Version::new(3, 3))))
	.build(Some(window.raw_window_handle().expect("raw window handle")));

    let mut gl_context = unsafe {
        gl_display.create_context(&config, &context_attributes).unwrap()
    };

    let surface_attrs = SurfaceAttributesBuilder::<WindowSurface>::new()
        .build(
            window.raw_window_handle().expect("raw window handle sa"),
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
        Tree::from_data(&fs::read(svg_path)?, &opt).expect("Failed to load SVG")
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
        Tree::from_data(&fs::read("test.svg")?, &opt).expect("Failed to load dummy test.svg file")
    };

    // Variables for pan, zoom
    // View state
    let mut zoom: f32 = 1.0;
    let mut pan = (0.0f32, 0.0f32);
    let mut last_cursor_pos: Option<(f64, f64)> = None;
    let mut dragging = false;
    let mut width = 800u32;
    let mut height = 600u32;

    event_loop.run(move |event, _elwt, control_flow| {

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
                _ => {}
            },
            _ => {}
        }		// END match event
    }).unwrap();	// END event_loop.run
    OK(())
}

