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
 *
 * References:
 * - https://docs.rs/winit/0.29.12/winit/event/struct.KeyEvent.html
 *
 * TODO:
 * - check out more leniant svg parsers instead of usvg: svg, lyon_svg, or even nanosvg.
 */
use glutin::config::{ConfigTemplate};
use glutin::context::{ContextApi, ContextAttributesBuilder};
use glutin::display::{DisplayApiPreference};
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};

use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent, ElementState, MouseButton, MouseScrollDelta};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::window::WindowBuilder;
use winit::keyboard::Key;
use winit::event::Modifiers;



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

struct GlState {
    // gl_display: glutin::display::Display,
    // config: glutin::config::Config,
    pub gl: glow::Context,
    pub surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
    pub gl_context: glutin::context::PossiblyCurrentContext,
}

pub fn init_gl(
    event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
    window: &winit::window::Window,
) -> GlState {
    use glutin::prelude::*;

    // Create GL config
    let display_builder = glutin_winit::DisplayBuilder::new()
        .with_window(Some(window.clone()));

    let template = glutin::config::ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_depth_size(24)
        .with_stencil_size(8)
        .build();

    let (window, gl_config) = display_builder
        .build(event_loop, template, |configs| configs[0].clone())
        .unwrap();

    let gl_display = gl_config.display();

    // Create context
    let context_attributes = glutin::context::ContextAttributesBuilder::new()
        .with_context_api(glutin::context::ContextApi::OpenGl(
            glutin::context::Version::new(3, 3),
        ))
        .build(Some(window.raw_window_handle()));

    let not_current_gl_context = unsafe {
        gl_display
            .create_context(&gl_config, &context_attributes)
            .unwrap()
    };

    // Create window surface
    let attrs = glutin::surface::SurfaceAttributesBuilder::<
        glutin::surface::WindowSurface,
    >::new()
    .build(
        window.raw_window_handle(),
        gl_config
            .surface_type()
            .unwrap()
            .bits(),
        window.inner_size().width,
        window.inner_size().height,
    );

    let surface = unsafe {
        gl_display
            .create_window_surface(&gl_config, &attrs)
            .unwrap()
    };

    // Make context current
    let gl_context = not_current_gl_context.make_current(&surface).unwrap();

    // Load glow using the GL function loader
    let gl = unsafe {
        glow::Context::from_loader_function(|s| {
            gl_display.get_proc_address(s) as *const _
        })
    };

    // You can enable GL features here
    unsafe {
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
    }

    GlState {
        gl,
        surface,
        gl_context,
    }
}


fn _init_gl_old(window: &winit::window::Window) -> GlState {
    // --- All your OpenGL / Glutin initialization goes here ---

    // Correct glutin 0.32.3 API usage
    let gl_display = unsafe {
        glutin::display::Display::new(
            window.raw_display_handle().expect("Failed to get raw display handle"),
            glutin::display::DisplayApiPreference::Egl,
        ).unwrap()
    };

    let config_template = glutin::config::ConfigTemplate::default();
    let mut configs = unsafe { gl_display.find_configs(config_template) };
    let config = configs.next().unwrap_or_else(|| panic!("no config found"));	// next() requires a mutable.

    let ctx_attr = glutin::context::ContextAttributesBuilder::new()
        .with_context_api(glutin::context::ContextApi::OpenGl(
            Some(glutin::context::Version::new(3, 3)),
        ))
	.build(Some(window.raw_window_handle().expect("raw window handle")));

    let mut gl_context = unsafe { gl_display.create_context(&config, &ctx_attr).unwrap() };

    let size = window.inner_size();
    let surf_attr = glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new()
        .build(
            window.raw_window_handle().expect("raw window handle sa"),
            std::num::NonZeroU32::new(size.width).unwrap(),
            std::num::NonZeroU32::new(size.height).unwrap(),
        );

    let surface = unsafe {
        gl_display.create_window_surface(&config, &surf_attr).unwrap()
    };

    let gl_context = gl_context.make_current(&surface).unwrap();

    GlState {
        gl_display,
        gl_context,
        surface,
        config,
    }
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // complex signature above to allow using the ? operator.
    let event_loop = EventLoop::new().unwrap();

    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("SVG Viewer")
        .build(&event_loop)
        .unwrap();

    let mut gl_state = Some(init_gl(&event_loop, &window));

    // let mut window: Option<Window> = None;
    // let mut gl_state: Option<GlState> = None;

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

	// usvg recommends to omit the <?xml ...?> node. If used, it must start at offset 0. Why is usvg so strict?
        let svg_content = r###"
<svg width="400" height="300" xmlns="http://www.w3.org/2000/svg">
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

    let _ = event_loop.run(move |event, elwt| {
	// REQUIRED: tell winit the event loop continues
	elwt.set_control_flow(ControlFlow::Wait);	// needed by winit 0.29

	// compiler does not know the order of events. and this allow does not help. Sigh.
        #[allow(unused_assignments)]
	let mut modifiers = Modifiers::default();

	// On Wayland and macOS, OpenGL contexts must be created after Event::Resumed, not before

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput { event, .. }  => {
                    if event.state == ElementState::Pressed
			&& event.logical_key == Key::Character("q".into())
			&& modifiers.state().control_key()
                    {
                        elwt.exit();
                    }
                }
                WindowEvent::ModifiersChanged(new_mods) => {
                    modifiers = new_mods.clone();	// Clone the reference
                }
		WindowEvent::CloseRequested => elwt.exit(),
	        WindowEvent::RedrawRequested(..) => {
                    if let Some(gls) = &gl_state {
                        unsafe {
			    gls.gl.clear_color(0.2, 0.3, 0.3, 1.0);
			    gls.gl.clear(glow::COLOR_BUFFER_BIT);
                            // gl::ClearColor(0.2, 0.3, 0.3, 1.0);
                            // gl::Clear(gl::COLOR_BUFFER_BIT);
                        }

                        gls.surface.swap_buffers(&gls.gl_context).unwrap();
                    }
                }
		WindowEvent::Resized(size) => {
		    if let Some(gls) = &gl_state {
		        gls.surface.resize(
		            &gls.gl_context,
		            size.width.try_into().unwrap(),
		            size.height.try_into().unwrap(),
		        );
		    }
		}
                _ => {}		// empty block returns unit ()
            }
            Event::Resumed => {
                // 1. Create window now (this is required)
                let new_window = WindowBuilder::new()
                    .with_title("SVG Vector Lab")
                    .with_inner_size(LogicalSize::new(800.0, 600.0))
                    .build(&elwt)
                    .unwrap();

                // 2. Call your custom GL init function
                let gl = init_gl(&new_window);

                window = Some(new_window);
                gl_state = Some(gl);
            }
/*
            WindowEvent::RedrawRequested(_) => {
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
		// “keep drawing frames continuously whenever the event loop is otherwise idle.”
		// maybe not a good idea, burns CPU?
                window.request_redraw();
            }
*/
            _ => {}
        }		// END match event
    });			// END event_loop.run
    Ok(())
}

