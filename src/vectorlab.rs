use std::fs;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{Event, WindowEvent, StartCause},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowAttributes},
};
use glutin_winit::DisplayBuilder;
use glutin::{
    config::ConfigSurfaceTypes, context::ContextAttributesBuilder,
    display::GetGlDisplay,
    prelude::*,
    surface::{Surface, WindowSurface, SurfaceAttributesBuilder},
};
use glow::HasContext;
use egui_winit::State as EguiWinitState;
use egui::{ClippedPrimitive, Context as EguiContext, TexturesDelta};
use egui_glow::Painter;
use resvg::tiny_skia::PathBuilder;
use resvg::usvg::{self, TreeParsing};

struct VectorLabApp {
    egui_ctx: EguiContext,
    egui_winit: EguiWinitState,
    painter: Painter,
    paint_jobs: Vec<ClippedPrimitive>,
    textures: TexturesDelta,
    window_size: (usize, usize),
    gl: glow::Context,
    surface: Surface<WindowSurface>,
    gl_context: glutin::context::PossiblyCurrentContext<glutin_winit::Api>,
    window: Window,
    paths: Vec<Vec<[f32; 2]>>,
    scale: f32,
    offset: [f32; 2],
    file_dialog_open: bool,
    current_file: Option<String>,
}

impl VectorLabApp {
    fn new(window: &Window, gl_display: &glutin::display::Display<glutin_winit::Api>) -> Result<Self, Box<dyn std::error::Error>> {
        let gl_config = gl_display
            .find_configs(ConfigSurfaceTypes::default())
            .expect("No GL config")
            .next()
            .ok_or("No suitable GL config")?;

        let surface_attrs = SurfaceAttributesBuilder::new()
            .build(window.raw_window_handle(), None)?;

        let surface = unsafe {
            gl_display.create_window_surface(&gl_config, &surface_attrs)?
        };

        let context_attrs = ContextAttributesBuilder::new().build(Some(window.raw_window_handle()))?;
        let gl_context = unsafe {
            gl_display.create_context(&gl_config, &context_attrs)?
                .make_current(&surface)?
        };

        let gl = unsafe { glow::Context::from_loader_function(|s| gl_context.get_proc_address(s) as *const _) };

        let egui_ctx = EguiContext::default();
        let mut egui_winit = EguiWinitState::new(&egui_ctx, window, None);
        let painter = Painter::new(&mut gl, None);

        Ok(Self {
            egui_ctx,
            egui_winit,
            painter,
            paint_jobs: vec![],
            textures: Default::default(),
            window_size: (1200, 800),
            gl,
            surface,
            gl_context,
            window: window.clone(),
            paths: vec![],
            scale: 1.0,
            offset: [0.0, 0.0],
            file_dialog_open: false,
            current_file: None,
        })
    }

    fn load_svg(&mut self, path: &str) {
        match fs::read_to_string(path) {
            Ok(svg_string) => {
                let opts = usvg::Options::default();
                if let Ok(tree) = usvg::Tree::from_str(&svg_string, &opts) {
                    self.paths.clear();
                    let svg_size = tree.size().to_int_size();

                    for node in tree.root().descendants() {
                        if let usvg::NodeKind::Path(path_node) = node.borrow() {
                            let mut path_builder = PathBuilder::new();
                            for segment in &path_node.data.segments {
                                match &segment {
                                    &usvg::path::PathSegment::MoveTo { x, y } =>
                                        path_builder.move_to(x as f32, y as f32),
                                    &usvg::path::PathSegment::LineTo { x, y } =>
                                        path_builder.line_to(x as f32, y as f32),
                                    usvg::path::PathSegment::CurveTo { x1, y1, x2, y2, x, y } =>
                                        path_builder.cubic_to(x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32),
                                    _ => {}
                                }
                            }

                            if let Some(tsvg_path) = path_builder.finish() {
                                let mut points = vec![];
                                for point in tsvg_path.iter() {
                                    points.push([point.x as f32, point.y as f32]);
                                }
                                if !points.is_empty() {
                                    self.paths.push(points);
                                }
                            }
                        }
                    }

                    self.scale = 400.0 / svg_size.width().max(1.0) as f32;
                    self.offset = [600.0, 400.0];
                    self.current_file = Some(path.to_string());
                }
            }
            Err(e) => eprintln!("Failed to load SVG {}: {}", path, e),
        }
    }

    fn render(&mut self) -> Result<(), winit::error::EventLoopError> {
        unsafe {
            self.gl.clear_color(0.1, 0.1, 0.1, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }

        let raw_input = self.egui_winit.take_egui_input(self.window_size);
        let output = self.egui_ctx.run(raw_input, |egui_ctx| {
            egui::TopBottomPanel::top("menu_bar").show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("📁 Open").clicked() {
                        self.file_dialog_open = true;
                    }
                    ui.separator();
                    ui.label(self.current_file.as_deref().unwrap_or("No file"));
                });
            });

            egui::CentralPanel::default().show(egui_ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                ui.painter().rect_filled(rect, 0.0, egui::Color32::from_black_alpha(20));

                if self.file_dialog_open {
                    ui.centered_and_justified(|ui| {
                        ui.heading("Load SVG file");
                        if ui.button("Load /home/jw/test.svg").clicked() {
                            self.load_svg("/home/jw/test.svg");
                            self.file_dialog_open = false;
                        }
                    });
                } else if !self.paths.is_empty() {
                    ui.heading(format!("{} paths loaded", self.paths.len()));
                    egui::ScrollArea::both().show(ui, |ui| {
                        let rect = ui.available_rect_before_wrap();
                        ui.painter().rect(rect, 0.0, egui::Color32::BLACK, egui::Stroke::new(1.0, egui::Color32::WHITE));

                        for path in &self.paths {
                            let points: Vec<egui::Pos2> = path.iter()
                                .map(|p| egui::Pos2::new(
                                    rect.left() + (p[0] * self.scale + self.offset[0]) * rect.width() / 1200.0,
                                    rect.top() + (p[1] * self.scale + self.offset[1]) * rect.height() / 800.0,
                                ))
                                .collect();
                            if points.len() > 1 {
                                ui.painter().add(egui::Shape::line(points.iter().cloned().collect::<Vec<_>>(), egui::Stroke::new(2.0, egui::Color32::GREEN)));
                            }
                        }
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.heading("VectorLab");
                        ui.label("Press 📁 Open or 'O' to load SVG");
                    });
                }
            });
        });

        self.textures.append(output.textures_delta);
        self.egui_winit.handle_platform_output(&self.window, output.platform_output);
        self.paint_jobs = self.egui_ctx.tessellate(output.shapes, egui_ctx.tessellation_config());

        self.painter.paint_and_update_textures(
            &[self.window_size.0 as f32, self.window_size.1 as f32],
            &mut self.gl,
            &mut self.textures,
            &self.paint_jobs,
            &self.egui_ctx.tessellation_config(),
        )?;

        self.surface.swap_buffers(&self.gl_context)?;

        Ok(())
    }
}

impl ApplicationHandler for VectorLabApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: winit::window::WindowId, event: WindowEvent) {
        self.egui_winit.on_window_event(&self.egui_ctx, &event);
        if self.egui_winit.egui_ctx().wants_pointer_input() || self.egui_winit.egui_ctx().wants_keyboard_input() {
            return;
        }

        match event {
            WindowEvent::Resized(size) => {
                self.window_size = (size.width as _, size.height as _);
                self.gl.viewport(0, 0, size.width as i32, size.height as i32);
            }
            WindowEvent::RedrawRequested => {
                let _ = self.render();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event: keyboard_input, .. } => {
                if keyboard_input.state.is_pressed() && !keyboard_input.repeat {
                    match keyboard_input.logical_key {
                        Key::Named(NamedKey::KeyO) => {
                            self.file_dialog_open = true;
                            self.window.request_redraw();
                        }
                        Key::Named(NamedKey::Escape) => event_loop.exit(),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _start_cause: StartCause) {
        self.textures.remove();
        self.window.request_redraw();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let event_loop = EventLoop::new()?;

    let window_attrs = WindowAttributes::default()
        .with_title("VectorLab - SVG Viewer")
        .with_inner_size(LogicalSize::new(1200.0, 800.0));

    let window = event_loop.create_window(window_attrs)?;

    let gl_display = glutin_winit::DisplayBuilder::new(event_loop.instance())
        .with_window_attributes(window.window_attributes_dpi())
        .build(&event_loop, glutin_winit::DisplayRequestTemplate::default(), |configs| configs.next().unwrap())?;

    let mut app = VectorLabApp::new(&window, &gl_display.0)?;

    event_loop.run_app(&mut app)?;
    Ok(())
}
