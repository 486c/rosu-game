use std::{
    path::{Path, PathBuf},
    sync::{
        mpsc::{Receiver, Sender},
        Arc, RwLock,
    },
};

use cgmath::Vector2;
use egui::Modal;
use rosu::{
    camera::Camera,
    config::Config,
    graphics::Graphics,
    hit_objects::{hit_window::HitWindow, Object, ObjectKind},
    math::{calc_hitcircle_diameter, calc_playfield, calculate_preempt_fadein},
    osu_db::{OsuDatabase, DEFAULT_DB_PATH},
    osu_renderer::{OsuRenderer, QUAD_INDECIES},
    rgb::{mix_colors_linear, Rgb},
    skin_manager::SkinManager,
    timer::Timer,
    vertex::Vertex,
};
use rosu_map::Beatmap;
use wgpu::{util::DeviceExt, BindGroup, BufferUsages, TextureView};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::MouseButton,
    keyboard::KeyCode,
};

const CLICKABLE_SELECTION_RADIUS: f64 = 4.0;

use crate::{
    analyze_cursor_renderer::{AnalyzeCursorRenderer, PointsInstance},
    click_detector::ClickResult,
    playfield::PlayfieldRenderer,
    replay_log::{ReplayFrame, ReplayLog}, ui::analyze::AnalyzeUi,
};

enum ReplayViewerEvents {
    OpenReplay(PathBuf),
    ScanBeatmaps(PathBuf),
    ResetModal,
    SaveReplay(PathBuf),
    UpdateReplayPositionByTime(f64),
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
    //judgements_list: Option<Vec<JudgementPoint>>,
    cursor_renderer: AnalyzeCursorRenderer<'rvs>,
    playfield_renderer: PlayfieldRenderer,

    playing: bool,
    slider_time: f64,
    time: Timer,

    settings: ReplayViewerSettings,
    gameplay_config: Arc<RwLock<Config>>,
    skin_manager: Arc<RwLock<SkinManager>>,

    osu_renderer: OsuRenderer<'rvs>,

    /// TODO
    camera: Camera,
    camera_bind_group: BindGroup,
    camera_buffer: wgpu::Buffer,

    quad_vertex_buffer: wgpu::Buffer,
    quad_index_buffer: wgpu::Buffer,

    selected_frame: Option<usize>,
    prev_selected_frame: Option<usize>,
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
    right_mouse_holding: bool,

    // Stupid egui handling lmao
    modal_text: Option<String>,

    // Events
    tx: Sender<ReplayViewerEvents>,
    rx: Receiver<ReplayViewerEvents>,

    circle_diameter: f32,

    // Other Ui elements
    analyze_ui: AnalyzeUi

}

impl<'rvs> ReplayViewerState<'rvs> {
    pub fn new(graphics: Arc<Graphics<'rvs>>) -> Self {
        let (graphics_width, graphics_height) = graphics.get_surface_size();
        let camera = Camera::new(
            &graphics,
            graphics_width as f32,
            graphics_height as f32,
            1.0,
        );
        let config = Arc::new(RwLock::new(Config::default()));
        let skin_manager = Arc::new(RwLock::new(SkinManager::from_path("./assets", &graphics)));

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
            playfield_renderer: PlayfieldRenderer::new(graphics.clone()),
            osu_renderer: OsuRenderer::new(graphics.clone(), config.clone(), skin_manager.clone()),
            graphics,
            camera_bind_group,
            camera_buffer,
            quad_vertex_buffer,
            quad_index_buffer,
            settings: ReplayViewerSettings {
                frames_to_show: 20,
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
            right_mouse_holding: false,
            db: OsuDatabase::new_from_path(DEFAULT_DB_PATH).unwrap(),
            modal_text: None,
            tx,
            rx,
            circle_diameter: 4.0,
            selected_frame: None,
            prev_selected_frame: None,
            analyze_ui: AnalyzeUi::new(),
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
        self.circle_diameter = calc_hitcircle_diameter(cs);
        self.objects = Some(out_objects);
    }

    pub fn open_replay(&mut self, replay_path: impl AsRef<Path>) {
        let Ok(replay) = rosu_replay::Replay::from_path(&replay_path) else {
            self.modal_text = Some("Can't open replay file".to_owned());
            return;
        };

        //let mut processor: OsuProcessor = Replay::open(&replay_path.as_ref()).unwrap().into();

        let Some(beatmap_entry) = self.db.get_beatmap_by_hash(&replay.beatmap_hash) else {
            self.modal_text = Some("Can't find a beatmap for that replay".to_owned());
            return;
        };

        self.open_beatmap(beatmap_entry.path);
        
        let replay: ReplayLog = replay.into();
        self.analyze_ui.update_replay(&replay);
        self.replay = Some(replay);

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

        let iter = replay
            .frames
            .iter()
            .map(|f| Self::choose_frame_colors(&f, &self.settings));

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

        self.graphics.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&self.camera.gpu),
        ); // TODO
    }

    pub fn render(&mut self, view: &TextureView) {
        let _span = tracy_client::span!("state::render");

        self.handle_events();

        if self.objects.is_none() || self.replay.is_none() {
            return;
        }

        self.render_gameplay_objects(view);

        let mut encoder = self
            .graphics
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

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

            // Playfield
            render_pass.set_pipeline(&self.playfield_renderer.pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.playfield_renderer.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.playfield_renderer.instance_buffer.slice(..));

            render_pass
                .set_index_buffer(self.quad_index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            render_pass.draw_indexed(0..QUAD_INDECIES.len() as u32, 0, 0..1);

            // Lines
            render_pass.set_pipeline(&self.cursor_renderer.lines_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.cursor_renderer.lines_vertex_buffer.slice(..));

            render_pass.draw(
                self.replay_frame_start_idx as u32..self.replay_frame_end_idx as u32,
                0..1,
            );

            // Points
            render_pass.set_pipeline(&self.cursor_renderer.points_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.cursor_renderer.points_instance_buffer.slice(..));

            render_pass
                .set_index_buffer(self.quad_index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            render_pass.draw_indexed(
                0..QUAD_INDECIES.len() as u32,
                0,
                self.replay_frame_start_idx as u32..self.replay_frame_end_idx as u32,
            );
        }

        let span = tracy_client::span!("state::render::queue_submit");
        self.graphics.queue.submit([encoder.finish()]);
        drop(span);

        if !self.time.is_paused() {
            self.slider_time = self.time.update();
            self.update_replay_position_by_time();
        }
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
        let Some(replay) = &self.replay else {
            return;
        };

        let total = self
            .replay_frame_end_idx
            .saturating_sub(self.replay_frame_start_idx);

        if total == 0 {
            return;
        };

        let mut alpha = 0.0;
        let step = 1.0 / total as f32;

        if let Some(idx) = self.prev_selected_frame {
            let new_instance = Self::choose_frame_colors(&replay.frames[idx], &self.settings);
            self.cursor_renderer.points_data_mut()[idx].color = new_instance.color;
        }

        if let Some(idx) = self.selected_frame {
            self.cursor_renderer.points_data_mut()[idx].color = [1.0, 0.0, 0.0];
        }

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

        self.set_time(time);
        self.update_replay_position_by_time();
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
        self.replay_frame_end_idx = replay
            .frames
            .iter()
            .enumerate()
            .rev()
            .find(|(_i, frame)| frame.ts <= self.time.get_time())
            .map(|(i, _frame)| i)
            .unwrap_or(0);

        self.replay_frame_start_idx = self
            .replay_frame_end_idx
            .saturating_sub(self.settings.frames_to_show);

        self.update_analyze_cursor_buffers();
    }

    fn render_gameplay_objects(&mut self, view: &TextureView) {
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
                    self.osu_renderer.prepare_and_render_slider_texture(slider);
                }
                _ => {}
            }

            self.objects_render_queue.push(i);
        }

        self.osu_renderer.prepare_objects(
            self.time.get_time(),
            self.preempt,
            self.fadein,
            &self.objects_render_queue,
            &objects,
            &self.hit_window,
        );

        self.osu_renderer.prepare();

        self.osu_renderer.write_buffers();

        // 2. Rendering
        self.osu_renderer
            .render_objects(&view, &self.objects_render_queue, &objects)
            .unwrap();

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
                        ui.label("Time:");
                        let response = ui.add(egui::DragValue::new(&mut self.slider_time));

                        if response.changed() {
                            self.set_time(self.slider_time);
                            self.update_replay_position_by_time()
                        }

                        if let Some(replay) = &self.replay {
                            let idx = self.replay_frame_end_idx;
                            ui.label(&format!("Frame ms: {}", replay.frames[idx].ts));
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

                    let response = ui
                        .add(
                            egui::Slider::new(&mut self.slider_time, min..=max)
                                .show_value(false)
                        );

                    if response.changed() {
                        self.set_time(self.slider_time);
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
            
            if self.replay.is_some() {
                if ui.button("Save replay").clicked() {
                    self.spawn_save_replay();
                }
            }
            
            ui.separator();
            ui.heading("Settings");
            let resp = ui.add(
                egui::Slider::new(&mut self.settings.frames_to_show, 0..=100)
                    .step_by(1.0)
                    .text("Show frames"),
            );

            if resp.changed() {
                self.update_replay_position_by_time()
            }

            let resp = ui.add(
                egui::Slider::new(&mut self.settings.frame_point_size, 1..=50)
                    .step_by(1.0)
                    .text("Frame point size"),
            );

            if resp.changed() {
                self.update_frame_point_size()
            }

            ui.horizontal(|ui| {
                ui.label("K1 Color");
                if ui
                    .color_edit_button_srgb(&mut self.settings.k1_color)
                    .changed()
                {
                    self.sync_cursor()
                };
            });

            ui.horizontal(|ui| {
                ui.label("K2 Color");
                if ui
                    .color_edit_button_srgb(&mut self.settings.k2_color)
                    .changed()
                {
                    self.sync_cursor()
                };
            });

            ui.horizontal(|ui| {
                ui.label("M1 Color");
                if ui
                    .color_edit_button_srgb(&mut self.settings.m1_color)
                    .changed()
                {
                    self.sync_cursor()
                };
            });

            ui.horizontal(|ui| {
                ui.label("M2 Color");
                if ui
                    .color_edit_button_srgb(&mut self.settings.m2_color)
                    .changed()
                {
                    self.sync_cursor()
                };
            });

            if let Some(replay) = &self.replay {
                ui.collapsing("Current Frame Info", |ui| {
                    let frame = &replay.frames[self.replay_frame_end_idx];

                    ui.label(format!("Index: {}", self.replay_frame_end_idx));
                    ui.label(format!("Frame Time: {}", frame.ts));
                    ui.label(format!(
                        "Position: ({:.2}, {:.2})",
                        frame.pos.0, frame.pos.1
                    ));
                    ui.label(format!("Is K1 Pressed: {}", frame.keys.k1));
                    ui.label(format!("Is K2 Pressed: {}", frame.keys.k2));
                    ui.label(format!("Is M1 Pressed: {}", frame.keys.m1));
                    ui.label(format!("Is M2 Pressed: {}", frame.keys.m2));
                });

                if let Some(selected_frame) = self.selected_frame {
                    ui.collapsing("Currently Selected Frame Info", |ui| {
                        let frame = &replay.frames[selected_frame];

                        ui.label(format!("Index: {}", selected_frame));
                        ui.label(format!("Frame Time: {}", frame.ts));
                        ui.label(format!(
                            "Position: ({:.2}, {:.2})",
                            frame.pos.0, frame.pos.1
                        ));
                        ui.label(format!("Is K1 Pressed: {}", frame.keys.k1));
                        ui.label(format!("Is K2 Pressed: {}", frame.keys.k2));
                        ui.label(format!("Is M1 Pressed: {}", frame.keys.m1));
                        ui.label(format!("Is M2 Pressed: {}", frame.keys.m2));
                    });
                }
            };

            ui.separator();
            ui.heading("Analyze");
            self.analyze_ui.render_ui(ui);

            ui.separator();
            ui.heading("Tools");
            if ui.button("Convert m1/m2 to k1/k2").clicked() {
                // TODO maybe move this somewhere?
                let changed = if let Some(replay) = &mut self.replay {
                    replay.frames
                        .iter_mut()
                        .for_each(|frame| {
                            if frame.keys.m1 && !frame.keys.k1 {
                                frame.keys.k1 = true;
                            };

                            if frame.keys.m2 && !frame.keys.k2 {
                                frame.keys.k2 = true;
                            };
                        });

                    true
                } else {
                    false
                };

                if changed {
                    self.sync_cursor();
                    if let Some(replay) = &self.replay {
                        self.analyze_ui.update_replay(replay);
                    }
                }
            };
        });

    }

    pub fn on_pressed_down(&mut self, key_code: KeyCode) {
        let _span = tracy_client::span!("state::on_pressed_down");
        if key_code == KeyCode::ArrowRight || key_code == KeyCode::KeyD {
            if let Some(replay) = &self.replay {
                self.replay_frame_end_idx =
                    (self.replay_frame_end_idx + 1).min(replay.frames.len() - 1);

                self.replay_frame_start_idx =
                    (self.replay_frame_start_idx + 1).min(replay.frames.len() - 1);

                self.update_replay_posititon_by_frame_idx();
            };
        }

        if key_code == KeyCode::ArrowLeft || key_code == KeyCode::KeyA {
            if self.replay.is_some() {
                self.replay_frame_end_idx = self.replay_frame_end_idx.saturating_sub(1);
                self.replay_frame_start_idx = self.replay_frame_start_idx.saturating_sub(1);

                self.update_replay_posititon_by_frame_idx();
            };
        }

        if key_code == KeyCode::KeyZ && self.selected_frame.is_none() {
            if let Some(replay) = &self.replay {
                self.time.pause();
                self.playing = false;
                let frame = replay.frames.first().unwrap();
                self.set_time(frame.ts);
                self.update_replay_position_by_time();
            }
        };

        if key_code == KeyCode::KeyV && self.selected_frame.is_none() {
            if let Some(replay) = &self.replay {
                self.time.pause();
                self.playing = false;
                let frame = replay.frames.last().unwrap();
                self.set_time(frame.ts);
                self.update_replay_position_by_time();
            }
        };

        // Key editing options
        if let Some(selected_frame) = &self.selected_frame {
            if let Some(replay) = &mut self.replay {
                let changed = match key_code {
                    KeyCode::KeyZ => replay.toggle_k1(*selected_frame),
                    KeyCode::KeyX => replay.toggle_k2(*selected_frame),
                    KeyCode::KeyC => replay.toggle_m1(*selected_frame),
                    KeyCode::KeyV => replay.toggle_m2(*selected_frame),
                    _ => false,
                };

                if changed {
                    self.sync_cursor();
                    self.update_analyze_cursor_buffers();
                }
            }
        };

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

    pub fn on_mouse_state(&mut self, button: MouseButton, result: ClickResult) {
        match result {
            ClickResult::None => {}
            ClickResult::Click => {
                if button == MouseButton::Left {
                    self.on_click();
                }
            }
            ClickResult::HoldStart => match button {
                MouseButton::Left => self.left_mouse_holding = true,
                MouseButton::Right => self.right_mouse_holding = true,
                _ => {}
            },
            ClickResult::HoldEnd => match button {
                MouseButton::Left => self.left_mouse_holding = false,
                MouseButton::Right => self.right_mouse_holding = false,
                _ => {}
            },
        }
    }

    pub fn on_mouse_moved(&mut self, position: &PhysicalPosition<f64>) {
        let _span = tracy_client::span!("state::on_mouse_moved");

        if self.right_mouse_holding {
            let delta = self.mouse_pos - Vector2::new(position.x as f32, position.y as f32);
            self.camera.move_camera(delta);
            self.osu_renderer.move_camera(delta);

            self.graphics.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::bytes_of(&self.camera.gpu),
            ); // TODO

            self.osu_renderer.write_camera_buffers();
        }

        if self.left_mouse_holding {
            if let Some(selected_frame) = self.selected_frame {
                if let Some(replay) = &mut self.replay {
                    let world_pos = self
                        .camera
                        .screen_to_world(Vector2::new(position.x as f32, position.y as f32));

                    let (px, py) = (world_pos.x as f64, world_pos.y as f64);
                    let (cx, cy) = (
                        replay.frames[selected_frame].pos.0,
                        replay.frames[selected_frame].pos.1,
                    );

                    let distance = ((px - cx).powf(2.0) + (py - cy).powf(2.0)).sqrt();

                    let is_in_radius = distance <= (CLICKABLE_SELECTION_RADIUS * 2.0 / 2.0) as f64;

                    if is_in_radius {
                        replay.frames[selected_frame].pos =
                            (world_pos.x as f64, world_pos.y as f64);
                        self.sync_cursor();
                        self.update_analyze_cursor_buffers();
                    }
                }
            }
        }

        self.mouse_pos = Vector2::new(position.x as f32, position.y as f32)
    }

    // Quick click
    pub fn on_click(&mut self) {
        let s = self.camera.screen_to_world(self.mouse_pos);

        if let Some(replay) = &self.replay {
            let slice = replay
                .frames
                .get(self.replay_frame_start_idx..=self.replay_frame_end_idx);

            if let Some(slice) = slice {
                for (i, frame) in slice.iter().enumerate() {
                    let (px, py) = (s.x as f64, s.y as f64);
                    let (cx, cy) = (frame.pos.0, frame.pos.1);

                    let distance = ((px - cx).powf(2.0) + (py - cy).powf(2.0)).sqrt();

                    if !(distance <= (CLICKABLE_SELECTION_RADIUS / 2.0) as f64) {
                        continue;
                    };

                    let selected_index = i + self.replay_frame_start_idx;

                    if let Some(prev_selected) = self.selected_frame {
                        self.prev_selected_frame = Some(prev_selected)
                    };

                    self.selected_frame = Some(selected_index);
                    self.update_analyze_cursor_buffers();

                    return;
                }

                self.prev_selected_frame = self.selected_frame.take();
                self.update_analyze_cursor_buffers();
            };
        }
    }

    pub fn zoom_in(&mut self) {
        let _span = tracy_client::span!("state::zoom_in");
        self.zoom += 0.1;

        self.camera.zoom(0.1, self.mouse_pos);

        self.osu_renderer.zoom_camera(0.1, self.mouse_pos);
        self.osu_renderer.write_camera_buffers();

        self.graphics.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&self.camera.gpu),
        ); // TODO
    }

    pub fn zoom_out(&mut self) {
        let _span = tracy_client::span!("state::zoom_out");
        self.zoom -= 0.1;

        self.camera.zoom(-0.1, self.mouse_pos);

        self.osu_renderer.zoom_camera(-0.1, self.mouse_pos);
        self.osu_renderer.write_camera_buffers();

        self.graphics.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&self.camera.gpu),
        ); // TODO
    }

    pub fn handle_events(&mut self) {
        match self.rx.try_recv() {
            Ok(event) => match event {
                ReplayViewerEvents::OpenReplay(path_buf) => {
                    self.open_replay(&path_buf);
                }
                ReplayViewerEvents::ResetModal => self.modal_text = None,
                ReplayViewerEvents::ScanBeatmaps(path_buf) => {
                    let (_tx, rx) = oneshot::channel();
                    self.db.scan_beatmaps(path_buf, rx);
                }
                ReplayViewerEvents::UpdateReplayPositionByTime(ts) => {
                    self.set_time(ts);
                    self.update_replay_position_by_time();
                }
                ReplayViewerEvents::SaveReplay(path_buf) => {
                    if let Some(replay) = &self.replay {
                        replay.save_replay(&path_buf);
                    }
                }
            },
            Err(_) => {
                // TODO
                //println!("e")
            }
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
            let directory = rfd::FileDialog::new().pick_folder();

            if let Some(directory) = directory {
                let _ = tx.send(ReplayViewerEvents::ScanBeatmaps(directory.into()));
            }
        });
    }

    fn spawn_save_replay(&self) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let file = rfd::FileDialog::new().save_file();

            if let Some(file) = file {
                let _ = tx.send(ReplayViewerEvents::SaveReplay(file));
            }
        });
    }

    fn choose_frame_colors(f: &ReplayFrame, settings: &ReplayViewerSettings) -> PointsInstance {
        let mut initial_color = None;

        if f.keys.k1 {
            initial_color = match initial_color {
                Some(c) => Some(mix_colors_linear(&c, &Rgb::from(settings.k1_color), 0.5)),
                None => Some(Rgb::from(settings.k1_color)),
            };
        }

        if f.keys.k2 {
            initial_color = match initial_color {
                Some(c) => Some(mix_colors_linear(&c, &Rgb::from(settings.k2_color), 0.5)),
                None => Some(Rgb::from(settings.k1_color)),
            };
        }

        if f.keys.m1 {
            initial_color = match initial_color {
                Some(c) => Some(mix_colors_linear(&c, &Rgb::from(settings.m1_color), 0.5)),
                None => Some(Rgb::from(settings.k1_color)),
            };
        }

        if f.keys.m2 {
            initial_color = match initial_color {
                Some(c) => Some(mix_colors_linear(&c, &Rgb::from(settings.m2_color), 0.5)),
                None => Some(Rgb::from(settings.k1_color)),
            };
        }

        PointsInstance {
            pos: [f.pos.0 as f32, f.pos.1 as f32, 0.0],
            color: initial_color
                .unwrap_or(Rgb::new(91, 92, 97))
                .to_gpu_values(),
            alpha: 1.0,
            scale: 1.0,
        }
    }

    fn set_time(&mut self, time: f64) {
        self.time.set_time(time);
        self.slider_time = time;
    }
}
