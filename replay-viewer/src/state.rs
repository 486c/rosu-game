// Inner state of the replay viewer
// Responsible for
// 1. Rendering everything
// 2. Handling replay opening
// etc

use std::{env, path::{Path, PathBuf}, sync::{mpsc::{Receiver, Sender}, Arc}};

use cgmath::Vector2;
use egui::Modal;
use egui_file::FileDialog;
use osu_replay_parser::replay::Replay;
use rosu::{camera::Camera, config::Config, graphics::Graphics, hit_objects::{hit_window::HitWindow, Object, ObjectKind}, math::{calc_playfield, calculate_preempt_fadein}, osu_db::{OsuDatabase, DEFAULT_DB_PATH}, osu_renderer::{OsuRenderer, QUAD_INDECIES}, rgb::{mix_colors_linear, Rgb}, skin_manager::SkinManager, timer::Timer, vertex::Vertex};
use rosu_map::Beatmap;
use wgpu::{util::DeviceExt, BindGroup, BufferUsages, TextureView};
use winit::{dpi::{PhysicalPosition, PhysicalSize}, event::MouseButton, keyboard::KeyCode};

use crate::{analyze_cursor_renderer::{AnalyzeCursorRenderer, PointsInstance}, replay_log::ReplayLog};

enum ReplayViewerEvents {
    OpenReplay(PathBuf),
    ScanBeatmaps(PathBuf),
    ResetModal,
}

pub struct ReplayViewerSettings {
    /// Amount of frames to show before current position
    frames_to_show: usize,
    frame_point_size: usize,

    k1_color: [u8; 3],
    k2_color: [u8; 3],
    m1_color: [u8; 3],
    m2_color: [u8; 3],
}

pub struct ReplayViewerState<'rvs> {
    db: OsuDatabase,

    graphics: Arc<Graphics<'rvs>>,
    replay: Option<ReplayLog>,
    cursor_renderer: AnalyzeCursorRenderer<'rvs>,
    
    playing: bool,
    slider_time: f64,
    time: Timer,

    settings: ReplayViewerSettings,
    gameplay_config: Config,
    skin_manager: SkinManager,

    osu_renderer: OsuRenderer<'rvs>,

    /// TODO
    camera: Camera,
    camera_bind_group: BindGroup,
    camera_buffer: wgpu::Buffer,

    quad_vertex_buffer: wgpu::Buffer,
    quad_index_buffer: wgpu::Buffer,

    replay_frame_end_idx: usize,
    replay_frame_start_idx: usize,

    // Needed for rendering
    preempt: f32,
    fadein: f32,
    hit_window: HitWindow,
    objects: Option<Vec<Object>>,
    objects_render_queue: Vec<usize>,

    zoom: f32,
    offsets: Vector2<f32>,

    mouse_pos: Vector2<f32>,
    left_mouse_holding: bool,

    // Stupid egui handling lmao
    modal_text: Option<String>,

    // Events
    tx: Sender<ReplayViewerEvents>,
    rx: Receiver<ReplayViewerEvents>
}

impl<'rvs> ReplayViewerState<'rvs> {
    pub fn new(graphics: Arc<Graphics<'rvs>>) -> Self {
        let (graphics_width, graphics_height) = graphics.get_surface_size();
        let camera = Camera::new(
            &graphics,
            graphics_width as f32, graphics_height as f32, 1.0
        );
        let config = Config::default();
        let skin_manager = SkinManager::from_path("./assets", &graphics);

        let quad_verticies = Vertex::quad_centered(5.0, 5.0);

        let quad_vertex_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&quad_verticies),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });

        let quad_index_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_index_buffer"),
                    contents: bytemuck::cast_slice(QUAD_INDECIES),
                    usage: BufferUsages::INDEX,
                });

        let camera_bind_group_layout =
            graphics
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("camera_bind_group_layout"),
                });
        
        let camera_buffer = graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("uniform_buffer"),
                contents: bytemuck::bytes_of(&camera.gpu),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

        let camera_bind_group = graphics
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
                label: Some("camera_bind_group"),
            });


        let (tx, rx) = std::sync::mpsc::channel();

        Self {
            time: Timer::new(),
            replay: None,
            camera,
            cursor_renderer: AnalyzeCursorRenderer::new(graphics.clone()),
            osu_renderer: OsuRenderer::new(graphics.clone(), &Config::default()),
            graphics,
            camera_bind_group,
            camera_buffer,
            quad_vertex_buffer,
            quad_index_buffer,
            settings: ReplayViewerSettings {
                frames_to_show: 100,
                frame_point_size: 10,
                k1_color: [12, 12, 255],
                k2_color: [252, 12, 12],
                m1_color: [51, 255, 255],
                m2_color: [255, 51, 255],
            },
            replay_frame_end_idx: 0,
            replay_frame_start_idx: 0,
            preempt: 0.0,
            fadein: 0.0,
            hit_window: HitWindow::default(),
            objects: None,
            gameplay_config: config,
            skin_manager,
            objects_render_queue: Vec::with_capacity(10),
            slider_time: 0.0,
            playing: false,
            zoom: 1.0,
            offsets: Vector2::new(1.0, 1.0),
            mouse_pos: Vector2::new(0.0, 0.0),
            left_mouse_holding: false,
            db: OsuDatabase::new_from_path(DEFAULT_DB_PATH).unwrap(),
            modal_text: None,
            tx,
            rx,
        }
    }

    fn open_beatmap(&mut self, beatmap_path: PathBuf) {
        let _span = tracy_client::span!("state::open_beatmap");
        let map = match Beatmap::from_path(&beatmap_path) {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("Failed to parse beatmap: {e}");
                self.modal_text = Some("Can't open beatmap".to_owned());
                return;
            }
        };

        let cs = map.circle_size;

        self.osu_renderer.on_cs_change(cs);

        let (preempt, fadein) = calculate_preempt_fadein(map.approach_rate);
        let hit_window = HitWindow::from_od(map.overall_difficulty);
        let out_objects = Object::from_rosu(&map);

        self.preempt = preempt;
        self.fadein = fadein;
        self.hit_window = hit_window;
        self.objects = Some(out_objects);
    }

    pub fn open_replay(&mut self, replay_path: impl AsRef<Path>) {
        let Ok(replay) = Replay::open(&replay_path.as_ref()) else {
            self.modal_text = Some("Can't open replay file".to_owned());
            return;
        };

        let Some(beatmap_entry) = self.db.get_beatmap_by_hash(&replay.map_hash) else {
            self.modal_text = Some("Can't find a beatmap for that replay".to_owned());
            return;
        };

        self.open_beatmap(beatmap_entry.path);

        self.replay = Some(replay.into());

        self.time.reset_time();

        self.sync_cursor();
        self.update_replay_position_by_time();
        self.playing = false;
    }

    pub fn sync_cursor(&mut self) {
        let _span = tracy_client::span!("state::sync_cursor");
        let Some(replay) = &self.replay else {
            return;
        };

        let iter = replay.frames.iter()
            .map(|f| {

                let mut initial_color = None;

                if f.keys.k1 {
                    initial_color = match initial_color {
                        Some(c) => Some(mix_colors_linear(&c, &Rgb::from(self.settings.k1_color), 0.5)),
                        None => Some(Rgb::from(self.settings.k1_color)),
                    };
                }

                if f.keys.k2 {
                    initial_color = match initial_color {
                        Some(c) => Some(mix_colors_linear(&c, &Rgb::from(self.settings.k2_color), 0.5)),
                        None => Some(Rgb::from(self.settings.k1_color)),
                    };
                }

                if f.keys.m1 {
                    initial_color = match initial_color {
                        Some(c) => Some(mix_colors_linear(&c, &Rgb::from(self.settings.m1_color), 0.5)),
                        None => Some(Rgb::from(self.settings.k1_color)),
                    };
                }

                if f.keys.m2 {
                    initial_color = match initial_color {
                        Some(c) => Some(mix_colors_linear(&c, &Rgb::from(self.settings.m2_color), 0.5)),
                        None => Some(Rgb::from(self.settings.k1_color)),
                    };
                }

                PointsInstance {
                    pos: [f.pos.0 as f32, f.pos.1 as f32, 0.0],
                    color: initial_color.unwrap_or(Rgb::new(91, 92, 97)).to_gpu_values(),
                    alpha: 1.0,
                    scale: 1.0,
                }
            });
        
        self.cursor_renderer.clear_cursor_data();
        self.cursor_renderer.append_cursor_from_slice(iter);
    }

    pub fn on_resize(&mut self, new_size: &PhysicalSize<u32>) {
        let _span = tracy_client::span!("state::on_resize");
        let (scale, offsets) = calc_playfield(new_size.width as f32, new_size.height as f32);

        self.offsets = offsets;

        self.camera.resize(new_size);
        self.camera.transform(scale, offsets);

        self.osu_renderer.on_resize(&new_size);

        self.graphics
            .queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&self.camera.gpu)); // TODO
    }

    pub fn render(&mut self, view: &TextureView) {
        let _span = tracy_client::span!("state::render");

        self.handle_events();

        if self.objects.is_none() || self.replay.is_none() {
            return
        }

        self.render_gameplay_objects(view);

        let mut encoder =
            self.graphics
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: None,
            });

        {
            let _span = tracy_client::span!("state::render::record_render_pass");
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Lines
            render_pass.set_pipeline(&self.cursor_renderer.lines_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.cursor_renderer.lines_vertex_buffer.slice(..));

            render_pass.draw(
                self.replay_frame_start_idx as u32..self.replay_frame_end_idx as u32,
                0..1
            );
            
            // Points
            render_pass.set_pipeline(&self.cursor_renderer.points_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.cursor_renderer.points_instance_buffer.slice(..));

            render_pass.set_index_buffer(
                self.quad_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );

            render_pass.draw_indexed(
                0..QUAD_INDECIES.len() as u32,
                0,
                self.replay_frame_start_idx as u32..self.replay_frame_end_idx as u32,
            );
        }

        let span = tracy_client::span!("state::render::queue_submit");
        self.graphics.queue.submit([encoder.finish()]);
        drop(span);

        self.slider_time = self.time.update();
        self.update_replay_position_by_time();
    }

    fn update_frame_point_size(&mut self) {
        let _span = tracy_client::span!("state::update_frame_point_size");
        let quad_verticies = Vertex::quad_centered(
            self.settings.frame_point_size as f32, 
            self.settings.frame_point_size as f32, 
        );

        self.quad_vertex_buffer =
            self.graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("hit_circle_buffer"),
                contents: bytemuck::cast_slice(&quad_verticies),
                usage: BufferUsages::VERTEX,
            });
    }

    fn update_analyze_cursor_buffers(&mut self) {
        let total = self.replay_frame_end_idx.saturating_sub(self.replay_frame_start_idx);

        if total == 0 {
            return
        };

        let mut alpha = 0.0;
        let step = 1.0 / total as f32;

        for i in self.replay_frame_start_idx..=self.replay_frame_end_idx {
            self.cursor_renderer.points_data_mut()[i].alpha = alpha;
            self.cursor_renderer.lines_vertex_data_mut()[i].alpha = alpha;
            alpha += step;
        }

        self.cursor_renderer.write_buffers();
    }

    /// Calculates and updates data and gpu buffers based
    /// on current frame indexes
    fn update_replay_posititon_by_frame_idx(&mut self) {
        let Some(replay) = &self.replay else {
            return;
        };

        let time = replay.frames[self.replay_frame_end_idx].ts;

        self.time.set_time(time);
        self.update_analyze_cursor_buffers();
    }
    
    /// Calculates and updates data and gpu buffers based
    /// on current time
    fn update_replay_position_by_time(&mut self) {
        let _span = tracy_client::span!("state::update_replay_position");
        let Some(replay) = &self.replay else {
            return;
        };
        
        // Two values below used only for rendering
        self.replay_frame_end_idx = replay.frames
            .iter()
            .enumerate()
            .rev()
            .find(|(_i, frame)| frame.ts <= self.time.get_time() )
            .map(|(i, _frame)| i)
            .unwrap();

        self.replay_frame_start_idx = self.replay_frame_end_idx.saturating_sub(self.settings.frames_to_show);

        self.update_analyze_cursor_buffers();
    }
    
    fn render_gameplay_objects(&mut self,  view: &TextureView) {
        let _span = tracy_client::span!("state::render_gameplay_objects");

        // 1. Prepare all objects
        let Some(objects) = &mut self.objects else {
            return;
        };

        for (i, obj) in objects.iter_mut().enumerate().rev() {
            if !obj.is_visible(self.time.get_time(), self.preempt, &self.hit_window) {
                continue;
            }

            match &mut obj.kind {
                ObjectKind::Slider(slider) => {
                    self.osu_renderer.prepare_and_render_slider_texture(
                        slider, 
                        &self.skin_manager, 
                        &self.gameplay_config
                    );
                }
                _ => {},
            }

            self.objects_render_queue.push(i);
        }

        self.osu_renderer.prepare_objects(
            self.time.get_time(), self.preempt, self.fadein,
            &self.objects_render_queue, &objects,
            &self.skin_manager
        );

        self.osu_renderer.prepare(
            &self.gameplay_config
        );

        self.osu_renderer.write_buffers();

        // 2. Rendering
        self.osu_renderer.render_objects(
            &view,
            &self.objects_render_queue, &objects,
            &self.skin_manager,
        ).unwrap();

        self.objects_render_queue.clear();
    }

    pub fn render_ui(&mut self, ctx: &egui::Context) {
        let _span = tracy_client::span!("state::render_ui");

        if let Some(modal_text) = &self.modal_text {

            Modal::new(egui::Id::new("Modal")).show(ctx, |ui| {
                ui.label(modal_text);

                if ui.button("Ok").clicked() {
                    let _ = self.tx.send(ReplayViewerEvents::ResetModal);
                }
            });
        }

        egui::TopBottomPanel::bottom("bottom")
            .resizable(false)
            .show(ctx, |ui| {
                let slider_width = ui.available_width();
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(&format!("Time: {:.2} |", self.time.get_time()));

                        if let Some(replay) = &self.replay {
                            let idx = self.replay_frame_end_idx;
                            ui.label(&format!("Frame ms: {} |", replay.frames[idx].ts));
                            ui.label(&format!("Frame index: {}/{}", idx, replay.frames.len()));
                        };
                    });

                    ui.spacing_mut().slider_width = slider_width;

                    let (min, max) = if let Some(replay) = &self.replay {
                        let min = replay.frames.first().unwrap().ts;
                        let max = replay.frames.last().unwrap().ts;

                        (min, max)
                    } else {
                        let min = 0.0;
                        let max = 100.0;

                        (min, max)
                    };

                    let response = ui.add(
                        egui::Slider::new(
                            &mut self.slider_time, min..=max
                        )
                        .show_value(false)
                    );

                    if response.changed() {
                        self.time.set_time(self.slider_time);
                        self.update_replay_position_by_time()
                    }
                });
            });

        egui::SidePanel::left("left").show(ctx, |ui| {
            if ui.button("Select replay").clicked() {
                self.spawn_replay_chooser();
            }

            if ui.button("Export beatmaps").clicked() {
                self.spawn_beatmaps_directory_chooser();
            }

            ui.heading("Settings");
            let resp = ui.add(
                egui::Slider::new(
                    &mut self.settings.frames_to_show, 0..=1000
                ).step_by(1.0).text("Show frames")
            );

            if resp.changed() {
                self.update_replay_position_by_time()
            }

            let resp = ui.add(
                egui::Slider::new(
                    &mut self.settings.frame_point_size, 1..=50
                ).step_by(1.0).text("Frame point size")
            );

            if resp.changed() {
                self.update_frame_point_size()
            }

            ui.horizontal(|ui| {
                ui.label("K1 Color");
                if ui.color_edit_button_srgb(&mut self.settings.k1_color).changed() {
                    self.sync_cursor()
                };
            });

            ui.horizontal(|ui| {
                ui.label("K2 Color");
                if ui.color_edit_button_srgb(&mut self.settings.k2_color).changed() {
                    self.sync_cursor()
                };
            });

            ui.horizontal(|ui| {
                ui.label("M1 Color");
                if ui.color_edit_button_srgb(&mut self.settings.m1_color).changed() {
                    self.sync_cursor()
                };
            });

            ui.horizontal(|ui| {
                ui.label("M2 Color");
                if ui.color_edit_button_srgb(&mut self.settings.m2_color).changed() {
                    self.sync_cursor()
                };
            });

            ui.collapsing("Gameplay Visuals", |ui| {
                if ui.add(
                    egui::Slider::new(
                        &mut self.gameplay_config.slider.border_feather, 
                        0.01..=2.0
                    ).step_by(0.01).text("Border feather")
                ).changed() {
                    if let Some(objects) = &mut self.objects {
                        self.osu_renderer.clear_cached_slider_textures(
                            objects
                        );
                    }
                };

                if ui.add(
                    egui::Slider::new(
                        &mut self.gameplay_config.slider.border_size_multiplier, 
                        0.01..=2.0
                    ).step_by(0.01).text("Border size multiplier")
                ).changed() {
                    if let Some(objects) = &mut self.objects {
                        self.osu_renderer.clear_cached_slider_textures(
                            objects
                        );
                    }
                };

                if ui.add(
                    egui::Slider::new(
                        &mut self.gameplay_config.slider.body_color_saturation, 
                        0.01..=2.0
                    ).step_by(0.01).text("Body color saturation")
                ).changed() {
                    if let Some(objects) = &mut self.objects {
                        self.osu_renderer.clear_cached_slider_textures(
                            objects
                        );
                    }
                };

                if ui.add(
                    egui::Slider::new(
                        &mut self.gameplay_config.slider.body_alpha_multiplier, 
                        0.01..=2.0
                    ).step_by(0.01).text("Body alpha multiplier")
                ).changed() {
                    if let Some(objects) = &mut self.objects {
                        self.osu_renderer.clear_cached_slider_textures(
                            objects
                        );
                    }
                };
            })
        });
    }

    pub fn on_pressed_down(&mut self, key_code: KeyCode) {
        let _span = tracy_client::span!("state::on_pressed_down");
        if key_code == KeyCode::ArrowRight {
            if let Some(replay) = &self.replay {
                self.replay_frame_end_idx = 
                    (self.replay_frame_end_idx + 1).min(replay.frames.len() - 1);

                self.replay_frame_start_idx = 
                    (self.replay_frame_start_idx + 1).min(replay.frames.len() - 1);

                self.update_replay_posititon_by_frame_idx();
            };
        }

        if key_code == KeyCode::ArrowLeft {
            if self.replay.is_some() {
                self.replay_frame_end_idx = 
                    self.replay_frame_end_idx.saturating_sub(1);

                self.replay_frame_start_idx = 
                    self.replay_frame_start_idx.saturating_sub(1);

                self.update_replay_posititon_by_frame_idx();
            };
        }

        if key_code == KeyCode::Space {
            if self.playing {
                self.time.pause();
                self.playing = false;
            } else {
                self.time.unpause();
                self.playing = true;
            }
        }
    }

    pub fn on_mouse_pressed(&mut self, button: MouseButton) {
        let _span = tracy_client::span!("state::on_mouse_pressed");
        if button == MouseButton::Left {
            self.left_mouse_holding = true;
        }
    }

    pub fn on_mouse_released(&mut self, button: MouseButton) {
        let _span = tracy_client::span!("state::on_mouse_released");
        if button == MouseButton::Left {
            self.left_mouse_holding = false;
        }
    }

    pub fn on_mouse_moved(&mut self, position: &PhysicalPosition<f64>) {
        let _span = tracy_client::span!("state::on_mouse_moved");
        if self.left_mouse_holding {
            let delta = self.mouse_pos - Vector2::new(position.x as f32, position.y as f32);
            self.camera.move_camera(delta);
            self.osu_renderer.move_camera(delta);

            self.graphics
                .queue
                .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&self.camera.gpu)); // TODO

            self.osu_renderer.write_camera_buffers();
        }

        self.mouse_pos = Vector2::new(position.x as f32, position.y as f32)
    }

    pub fn zoom_in(&mut self) {
        let _span = tracy_client::span!("state::zoom_in");
        self.zoom += 0.1;

        self.camera.zoom(0.1, self.mouse_pos);

        self.osu_renderer.zoom_camera(0.1, self.mouse_pos);
        self.osu_renderer.write_camera_buffers();

        self.graphics
            .queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&self.camera.gpu)); // TODO
    }

    pub fn zoom_out(&mut self) {
        let _span = tracy_client::span!("state::zoom_out");
        self.zoom -= 0.1;

        self.camera.zoom(-0.1, self.mouse_pos);

        self.osu_renderer.zoom_camera(-0.1, self.mouse_pos);
        self.osu_renderer.write_camera_buffers();

        self.graphics
            .queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&self.camera.gpu)); // TODO
    }

    pub fn handle_events(&mut self) {
        match self.rx.try_recv() {
            Ok(event) => match event {
                ReplayViewerEvents::OpenReplay(path_buf) => {
                    self.open_replay(&path_buf);
                },
                ReplayViewerEvents::ResetModal => self.modal_text = None,
                ReplayViewerEvents::ScanBeatmaps(path_buf) => {
                    let (_tx, rx) = oneshot::channel();
                    self.db.scan_beatmaps(path_buf, rx);
                },
            },
            Err(e) => {
                // TODO
                //println!("e")
            },
        }
    }

    fn spawn_replay_chooser(&self) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let file = rfd::FileDialog::new()
                .add_filter("osu", &["osr"])
                .pick_file();

            if let Some(file) = file {
                let _ = tx.send(ReplayViewerEvents::OpenReplay(file.into()));
            }
        });
    }

    fn spawn_beatmaps_directory_chooser(&self) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let directory = rfd::FileDialog::new()
                .pick_folder();

            if let Some(directory) = directory {
                let _ = tx.send(ReplayViewerEvents::ScanBeatmaps(directory.into()));
            }
        });
    }
}
