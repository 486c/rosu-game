use std::{
    ops::Range,
    path::{Path, PathBuf},
    sync::{
        mpsc::{Receiver, Sender},
        Arc, RwLock,
    },
};

use cgmath::{Vector2, Vector3};
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

use crate::{
    analyze_cursor_renderer::{AnalyzeCursorRenderer, PointsInstance},
    click_detector::ClickResult,
    lines_vertex::LinesVertex,
    playfield::PlayfieldRenderer,
    replay_log::{ReplayFrame, ReplayLog},
    tools::{SelectionChange, ToolBox, ToolOutcome},
    ui::analyze::AnalyzeUi,
};

pub enum ReplayViewerEvents {
    OpenReplay(PathBuf),
    ScanBeatmaps(PathBuf),
    ResetModal,
    SaveReplay(PathBuf),
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
    analyze_ui: AnalyzeUi,

    toolbox: ToolBox,
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
            toolbox: ToolBox::new(),
            tx,
            rx,
            circle_diameter: 4.0,
            selected_frame: None,
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

    /// Rebuilds and re-uploads the whole cursor data (points + lines).
    /// Only needed when something affects all frames (colors, key
    /// conversion, opening a replay). For single-frame edits use
    /// `sync_cursor_range` instead.
    pub fn sync_cursor(&mut self) {
        let _span = tracy_client::span!("state::sync_cursor");
        let Some(replay) = &self.replay else {
            return;
        };

        let points: Vec<PointsInstance> = replay
            .frames
            .iter()
            .map(|f| Self::choose_frame_colors(f, &self.settings))
            .collect();

        let lines: Vec<LinesVertex> = points
            .iter()
            .map(|p| LinesVertex {
                pos: Vector3::new(p.pos[0], p.pos[1], 1.0),
                alpha: 1.0,
            })
            .collect();

        self.cursor_renderer.set_data(points, lines);
    }

    /// Recomputes and re-uploads cursor data for a range of frames only.
    /// Used by tools and key edits so a one-frame change does not trigger
    /// a full-replay GPU upload.
    fn sync_cursor_range(&mut self, range: Range<usize>) {
        if range.is_empty() {
            return;
        }

        let Some(replay) = &self.replay else {
            return;
        };

        let mut points = Vec::with_capacity(range.len());
        let mut lines = Vec::with_capacity(range.len());

        for i in range.clone() {
            let p = Self::choose_frame_colors(&replay.frames[i], &self.settings);
            lines.push(LinesVertex {
                pos: Vector3::new(p.pos[0], p.pos[1], 1.0),
                alpha: 1.0,
            });
            points.push(p);
        }

        self.cursor_renderer.update_range(range.start, &points, &lines);
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

        // Playback: advance time and re-derive the visible frame window
        // BEFORE recording anything, so rendering uses up-to-date indices.
        if !self.time.is_paused() {
            self.slider_time = self.time.update();
            self.update_replay_position_by_time();
        }

        // Visible window + selection live in a tiny uniform now, so no
        // per-frame buffer uploads happen while scrubbing or playing.
        self.cursor_renderer.set_window(
            self.replay_frame_start_idx,
            self.replay_frame_end_idx,
            self.selected_frame,
        );

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
            render_pass.set_bind_group(1, &self.cursor_renderer.window_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.cursor_renderer.lines_vertex_buffer.slice(..));

            render_pass.draw(
                self.replay_frame_start_idx as u32..self.replay_frame_end_idx as u32,
                0..1,
            );

            // Points
            render_pass.set_pipeline(&self.cursor_renderer.points_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.cursor_renderer.window_bind_group, &[]);

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

    /// Calculates and updates the visible frame window based on current time.
    /// Binary search instead of a linear reverse scan: replays can hold
    /// tens of thousands of frames and this runs every rendered frame.
    fn update_replay_position_by_time(&mut self) {
        let _span = tracy_client::span!("state::update_replay_position");
        let Some(replay) = &self.replay else {
            return;
        };

        // Two values below used only for rendering.
        // NOTE: this intentionally collapses frames sharing a timestamp
        // (osu writes 0-delta frames on key events). Frame-by-frame
        // stepping goes through `step_frame`, which never re-derives the
        // index from time — otherwise it would get stuck inside a
        // duplicate-timestamp group.
        let time = self.time.get_time();
        let end = replay.frames.partition_point(|frame| frame.ts <= time);

        self.replay_frame_end_idx = end.saturating_sub(1);
        self.replay_frame_start_idx = self
            .replay_frame_end_idx
            .saturating_sub(self.settings.frames_to_show);
    }

    /// Moves the current frame by `delta` steps. The frame index is the
    /// source of truth here: we set the time from the frame, but never
    /// re-derive the index from the time. Re-deriving would collapse
    /// duplicate timestamps (osu key frames share one millisecond) and
    /// make A/D stepping stuck.
    pub fn step_frame(&mut self, delta: i64) {
        let Some(replay) = &self.replay else {
            return;
        };

        if replay.frames.is_empty() {
            return;
        }

        let new_end = (self.replay_frame_end_idx as i64 + delta)
            .clamp(0, replay.frames.len() as i64 - 1) as usize;

        if new_end == self.replay_frame_end_idx {
            return;
        }

        self.replay_frame_end_idx = new_end;
        self.replay_frame_start_idx = new_end.saturating_sub(self.settings.frames_to_show);

        self.set_time(replay.frames[new_end].ts);
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
                        .add(egui::Slider::new(&mut self.slider_time, min..=max).show_value(false));

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
            self.toolbox.render_ui(ui);

            if ui.button("Convert m1/m2 to k1/k2").clicked() {
                // TODO maybe move this somewhere?
                let changed = if let Some(replay) = &mut self.replay {
                    replay.frames.iter_mut().for_each(|frame| {
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
            self.step_frame(1);
        }

        if key_code == KeyCode::ArrowLeft || key_code == KeyCode::KeyA {
            self.step_frame(-1);
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
        if let Some(selected_frame) = self.selected_frame {
            let changed = self.replay.as_mut().map_or(false, |replay| match key_code {
                KeyCode::KeyZ => replay.toggle_k1(selected_frame),
                KeyCode::KeyX => replay.toggle_k2(selected_frame),
                KeyCode::KeyC => replay.toggle_m1(selected_frame),
                KeyCode::KeyV => replay.toggle_m2(selected_frame),
                _ => false,
            });

            if changed {
                // Only the edited frame's color changed — upload just it.
                self.sync_cursor_range(selected_frame..selected_frame + 1);
            }
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

        self.toolbox.on_mouse_moved(position);

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
            if self.replay.is_some() {
                let start = self.replay_frame_start_idx;
                let end = self.replay_frame_end_idx;

                let outcome = {
                    let replay = self.replay.as_mut().unwrap();
                    self.toolbox.on_left_mouse_holding(
                        &mut replay.frames[start..end],
                        start,
                        &self.camera,
                    )
                };

                self.apply_tool_outcome(outcome, start, end);
            }
        }

        self.mouse_pos = Vector2::new(position.x as f32, position.y as f32)
    }

    /// Applies what a tool did: updates selection and, when frames were
    /// edited, re-uploads only the edited window (instead of round-tripping
    /// through the event channel, which used to process one event per frame).
    fn apply_tool_outcome(&mut self, outcome: ToolOutcome, start: usize, end: usize) {
        if let Some(selection) = outcome.selection {
            match selection {
                SelectionChange::Selected(idx) => self.selected_frame = Some(idx),
                SelectionChange::Deselected => self.selected_frame = None,
            }
        }

        if outcome.changed {
            self.sync_cursor_range(start..end);
        }
    }

    // Quick click
    pub fn on_click(&mut self) {
        if self.replay.is_some() {
            let start = self.replay_frame_start_idx;
            let end = self.replay_frame_end_idx;

            let outcome = {
                let replay = self.replay.as_mut().unwrap();
                self.toolbox
                    .on_click(&mut replay.frames[start..end], start, &self.camera)
            };

            self.apply_tool_outcome(outcome, start, end);
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
        // Drain ALL pending events: previously only one event was consumed
        // per rendered frame, which made queued tool events lag behind.
        while let Ok(event) = self.rx.try_recv() {
            match event {
                ReplayViewerEvents::OpenReplay(path_buf) => {
                    self.open_replay(&path_buf);
                }
                ReplayViewerEvents::ResetModal => self.modal_text = None,
                ReplayViewerEvents::ScanBeatmaps(path_buf) => {
                    let (_tx, rx) = oneshot::channel();
                    self.db.scan_beatmaps(path_buf, rx);
                }
                ReplayViewerEvents::SaveReplay(path_buf) => {
                    if let Some(replay) = &self.replay {
                        replay.save_replay(&path_buf);
                    }
                }
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
        // Mix all pressed keys' colors together. (The fallback used to use
        // k1's color for other keys — fixed to use each key's own color.)
        let mut mixed: Option<Rgb> = None;

        macro_rules! mix_in {
            ($color:expr) => {
                mixed = Some(match mixed {
                    Some(prev) => mix_colors_linear(&prev, &Rgb::from($color), 0.5),
                    None => Rgb::from($color),
                });
            };
        }

        if f.keys.k1 {
            mix_in!(settings.k1_color);
        }

        if f.keys.k2 {
            mix_in!(settings.k2_color);
        }

        if f.keys.m1 {
            mix_in!(settings.m1_color);
        }

        if f.keys.m2 {
            mix_in!(settings.m2_color);
        }

        PointsInstance {
            pos: [f.pos.0 as f32, f.pos.1 as f32, 1.0],
            color: mixed
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
