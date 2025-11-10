use std::env;
use std::path::PathBuf;
use winit::{
    dpi::LogicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder},
};
use femtovg::{renderer::OpenGl, Canvas, Color, Path, Paint, Transform2D};
use glutin::{ContextBuilder, event::{Event, WindowEvent}, event_loop::ControlFlow};
use rfd::FileDialog;


fn main() {
    // Accept initial SVG file if provided
    let svg_file = env::args().nth(1).map(|s| PathBuf::from(s));
    let mut current_svg = svg_file;

    let event_loop = glutin::event_loop::EventLoop::new();
    let window_builder = glutin::window::WindowBuilder::new()
        .with_title("VectorLab")
        .with_inner_size(glutin::dpi::LogicalSize::new(1200.0, 800.0));
    let windowed_context = ContextBuilder::new()
        .build_windowed(window_builder, &event_loop)
        .unwrap();
    let windowed_context = unsafe { windowed_context.make_current().unwrap() };
    let gl = unsafe { glow::Context::from_loader_function(|s| windowed_context.get_proc_address(s)) };
    let renderer = OpenGl::new(gl).unwrap();
    let mut canvas = Canvas::new(renderer).unwrap();
    canvas.set_size(800, 600, 1.0);

    // Load and parse SVG file into femtovg paths and paints
    let svg_drawables = load_svg_as_paths_and_paints(&svg_file);

    // Pan, zoom, and mouse state
    let mut transform = Transform2D::identity();
    // TODO: maintain a stack or matrix for pan/zoom state

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::RedrawRequested(_) => {
                canvas.clear_rect(0, 0, 1200, 800, Color::rgbf(1.0, 1.0, 1.0));
                canvas.save();
                canvas.transform(transform);
                for (path, paint) in &svg_drawables {
                    // To detect hover, check mouse position vs. path hit-test here
                    canvas.fill_path(path, paint);
                }
                canvas.restore();

                // Draw UI: menu, zoom buttons (+, -, 100%), right pane checkboxes
                draw_ui_overlays(&mut canvas);

                canvas.flush();
                windowed_context.swap_buffers().unwrap();
            }
            Event::WindowEvent { event, .. } => match event {
                /* ... mouse + keyboard ... */
                WindowEvent::KeyboardInput { input, .. } => {
                    // Handle zoom with +/-, reset with '0', menu shortcuts
                    // Example: Ctrl-O triggers File > Open (can hook to menu, too)
                    if let Some(virtual_keycode) = input.virtual_keycode {
                        if input.state == ElementState::Pressed
                            && virtual_keycode == VirtualKeyCode::O
                            && input.modifiers.ctrl()
                        {
                            if let Some(path) = FileDialog::new().add_filter("SVG", &["svg"]).pick_file() {
                                current_svg = Some(path);
                                // Load and parse new SVG here!
                            }
                        }
                    }
                }
                WindowEvent::MouseInput {button, state, .. } => {
                    // Handle panning with shift + drag logic
                }
                WindowEvent::CursorMoved { position, .. } => {
                    // Track for hover on paths
                }
                WindowEvent::MouseWheel {delta, .. } => {
                    // Apply zoom logic
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            }
            Event::MainEventsCleared => {
                windowed_context.window().request_redraw();
            }
            _ => (),
        }
    });
}


fn load_svg_as_paths_and_paints(svg_file: &PathBuf) -> Vec<(Path, Paint)> {
    // Use resvg, usvg, or similar, or a custom parser
    // Each SVG <path> becomes a femtovg::Path and corresponding Paint
    vec![]
}

fn draw_ui_overlays(canvas: &mut Canvas<OpenGl>) {
    // Draw menu bar rectangles/text, e.g., File (Load, Save, Quit), Help, About
    // Draw zoom control (+/-/100% buttons)
    // Draw right pane checkboxes (future: add interactivity)
}

