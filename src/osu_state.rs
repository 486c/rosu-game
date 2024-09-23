use std::{fs::File, io::BufReader, path::{Path, PathBuf}, sync::{mpsc::{channel, Receiver, Sender, TryRecvError}, Arc}, time::{Duration, Instant}};

use cgmath::Vector2;
use egui::{RawInput, Slider};
use rodio::{source::UniformSourceIterator, Decoder, Sink, Source};
use rosu_map::Beatmap;
use wgpu::TextureView;
use winit::{dpi::{PhysicalPosition, PhysicalSize}, keyboard::KeyCode, window::Window};

use crate::{
    config::Config, egui_state::EguiState, frameless_source::FramelessSource, graphics::Graphics, hit_objects::{HitResult, Object, ObjectKind}, math::{calc_playfield, get_hitcircle_diameter}, osu_cursor_renderer::CursorRenderer, osu_db::BeatmapEntry, osu_input::{OsuInput, OsuInputState}, osu_renderer::OsuRenderer, skin_manager::SkinManager, song_select_state::SongSelectionState, timer::Timer, ui::settings::SettingsView
};


pub enum OsuStates {
    Playing,
    SongSelection,
}

pub enum OsuStateEvent {
    ToSongSelection,
    ChangeSkin(PathBuf),
    StartBeatmap(BeatmapEntry),
    PlaySound(i32, Box<dyn Source<Item = f32> + Send + Sync>),
}

/// Return preempt and fadein based on AR
fn calculate_preempt_fadein(ar: f32) -> (f32, f32) {
    if ar > 5.0 {
        (
            1200.0 - 750.0 * (ar - 5.0) / 5.0,
            800.0 - 500.0 * (ar - 5.0) / 5.0,
        )
    } else if ar < 5.0 {
        (
            1200.0 + 600.0 * (5.0 - ar) / 5.0,
            800.0 + 400.0 * (5.0 - ar) / 5.0,
        )
    } else {
        (1200.0, 800.0)
    }
}

pub struct HitWindow {
    pub x300: f64,
    pub x100: f64,
    pub x50: f64,
}

impl Default for HitWindow {
    fn default() -> Self {
        Self {
            x300: 0.0,
            x100: 0.0,
            x50: 0.0,
        }
    }
}

fn calculate_hit_window(od: f32) -> HitWindow {
    HitWindow {
        x300: 80.0 - 6.0 * (od as f64),
        x100: 140.0 - 8.0 * (od as f64),
        x50: 200.0 - 10.0 * (od as f64),
    }
}

pub struct OsuState<'s> {
    pub window: Arc<Window>,
    pub egui: EguiState,
    pub event_receiver: Receiver<OsuStateEvent>,
    pub event_sender: Sender<OsuStateEvent>,

    pub sink: Sink,

    pub current_state: OsuStates,
    pub song_select: SongSelectionState<'s>,

    skin_manager: SkinManager,
    config: Config,
    settings_view: SettingsView,

    osu_renderer: OsuRenderer<'s>,

    current_beatmap: Option<Beatmap>,
    preempt: f32,
    fadein: f32,
    current_hit_window: HitWindow,

    hit_objects: Vec<Object>,

    objects_render_queue: Vec<usize>,
    objects_judgments_render_queue: Vec<usize>,

    osu_clock: Timer,
    
    cursor_renderer: CursorRenderer<'s>,

    current_input_state: OsuInputState,
    input_buffer: Vec<OsuInput>,

    current_screen_size: Vector2<f32>,
    current_hit_circle_diameter: f32,
}

impl<'s> OsuState<'s> {
    pub fn new(window: Arc<Window>, graphics: Graphics<'s>, sink: Sink) -> Self {
        let egui = EguiState::new(&graphics, &window);
        let skin_manager = SkinManager::from_path("skin", &graphics);
        let config = Config::default();
        let graphics = Arc::new(graphics);

        let osu_renderer = OsuRenderer::new(graphics.clone(), &config);

        let (event_sender, event_receiver) = channel::<OsuStateEvent>();

        let song_select = SongSelectionState::new(graphics.clone(), event_sender.clone());

        window.set_cursor_visible(false);

        Self {
            cursor_renderer: CursorRenderer::new(graphics.clone()),
            event_receiver,
            preempt: 0.0,
            fadein: 0.0,
            osu_renderer,
            window,
            current_beatmap: None,
            egui,
            sink,
            osu_clock: Timer::new(),
            objects_render_queue: Vec::with_capacity(20),
            hit_objects: Vec::new(),
            skin_manager,
            config,
            settings_view: SettingsView::new(event_sender.clone()),
            current_state: OsuStates::SongSelection,
            song_select,
            event_sender,
            input_buffer: Vec::new(),
            current_hit_window: Default::default(),
            current_input_state: OsuInputState::default(),
            current_screen_size: Vector2::new(1.0, 1.0),
            current_hit_circle_diameter: 1.0,
            objects_judgments_render_queue: Vec::new(),
        }
    }

    pub fn open_skin(&mut self, path: impl AsRef<Path>) {
        let skin_manager = SkinManager::from_path(path, &self.osu_renderer.get_graphics());
        self.skin_manager = skin_manager;
    }

    pub fn open_beatmap(&mut self, path: impl AsRef<Path>) {
        self.osu_clock.reset_time();
        self.osu_clock.unpause();

        let map = match Beatmap::from_path(path.as_ref()) {
            Ok(m) => m,
            Err(e) => {
                println!("Failed to parse beatmap");
                println!("{}", e);
                return;
            }
        };

        self.sink.clear();

        let beatmap_dir = path.as_ref().parent().expect("failed to get beatmap dir");
        let audio_file = beatmap_dir.join(&map.audio_file);
        
        // We have to acknowlage the fact that there might be beatmaps
        // without any audio files
        if audio_file.is_file() {
            let file = BufReader::new(File::open(audio_file).unwrap());
            let source = FramelessSource::new(Decoder::new(file).expect("Failed to load audio file source"));
            let source = UniformSourceIterator::new(source, 2, 44100);
            self.set_audio(source);
            println!("open_beatmap: Initialized a new audio file!");
        }

        let (preempt, fadein) = calculate_preempt_fadein(map.approach_rate);
        let hit_window = calculate_hit_window(map.overall_difficulty);

        self.preempt = preempt;
        self.fadein = fadein;
        self.current_hit_window = hit_window;

        // Convert rosu_map object to our objects
        //let mut out_objects = Vec::with_capacity(map.hit_objects.len());
        let out_objects = Object::from_rosu(&map.hit_objects);

        self.hit_objects = out_objects;

        self.current_beatmap = Some(map);
        self.apply_beatmap_transformations();

        self.sink.play();
    }

    pub fn set_audio<I>(&self, audio: I) 
    where 
    I: Source<Item = f32> + Send + Sync + 'static {
        self.sink.append(audio);
    }

    pub fn apply_beatmap_transformations(&mut self) {
        let cs = match &self.current_beatmap {
            Some(beatmap) => beatmap.circle_size,
            None => 4.0,
        };

        self.osu_renderer.on_cs_change(cs);
        self.current_hit_circle_diameter = get_hitcircle_diameter(cs);
    }

    pub fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        self.current_screen_size.x = new_size.width as f32;
        self.current_screen_size.y = new_size.height as f32;

        self.cursor_renderer.on_resize(new_size);
        self.osu_renderer.on_resize(new_size);
        self.song_select.on_resize(new_size);
    }

    pub fn on_pressed_down(&mut self, key_code: KeyCode) {
        match self.current_state {
            OsuStates::Playing => {
                if key_code == KeyCode::Escape {
                    let _ = self.event_sender.send(OsuStateEvent::ToSongSelection);
                }
                
                let ts = self.osu_clock.since_start();

                if key_code == KeyCode::KeyZ {
                    self.input_buffer.push(OsuInput::key(ts, true, false, false, false));
                }

                if key_code == KeyCode::KeyX {
                    self.input_buffer.push(OsuInput::key(ts, false, true, false, false));
                }
            },
            OsuStates::SongSelection => {
                self.song_select.on_pressed_down(key_code);
            },
        }
    }

    pub fn on_pressed_release(&mut self, key_code: KeyCode) {
        match self.current_state {
            OsuStates::Playing => {

                let ts = self.osu_clock.since_start();
                if key_code == KeyCode::KeyZ {
                    self.input_buffer.push(OsuInput::key(ts, false, false, false, false));
                }

                if key_code == KeyCode::KeyX {
                    self.input_buffer.push(OsuInput::key(ts, false, false, false, false));
                }
            }
            _ => {}
        };
    }

    pub fn on_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.cursor_renderer.on_cursor_moved(position);

        match self.current_state {
            OsuStates::Playing => {
                let ts = self.osu_clock.since_start();
                let mut recv_pos = Vector2::new(position.x as f32, position.y as f32);
                let (scale, offsets) = calc_playfield(self.current_screen_size.x, self.current_screen_size.y);

                recv_pos -= offsets;
                recv_pos /= scale;
                
                let pos = Vector2::new(recv_pos.x as f64, recv_pos.y as f64);
                self.input_buffer.push(OsuInput::moved(ts, pos))
            },
            _ => {},
        }
    }

    pub fn update_egui(&mut self, input: RawInput) {
        let _span = tracy_client::span!("osu_state update egui");

        self.egui.state.egui_ctx().begin_frame(input);

        self.settings_view.window(
            &self.egui.state.egui_ctx(),
            &self.skin_manager,
            &mut self.config,
        );

        egui::Window::new("Debug Gameplay Window")
            .resizable(false)
            .show(&self.egui.state.egui_ctx(), |ui| {

            if let Some(beatmap) = &self.current_beatmap {
                ui.add(egui::Label::new(format!("{}", self.osu_clock.get_time())));

                if ui.add(
                    Slider::new(
                        &mut self.osu_clock.last_time,
                        1.0..=(beatmap.hit_objects.last().unwrap().start_time),
                    )
                    .step_by(1.0),
                ).changed() {
                    self.osu_clock.pause();
                    self.sink.try_seek(Duration::from_millis(self.osu_clock.get_time().round() as u64)).unwrap();
                    self.osu_clock.unpause();
                };

                if !self.osu_clock.is_paused() {
                    if ui.add(egui::Button::new("pause")).clicked() {
                        self.osu_clock.pause();
                        self.sink.pause();
                    }
                } else {
                    if ui.add(egui::Button::new("unpause")).clicked() {
                        self.sink.try_seek(Duration::from_millis(self.osu_clock.get_time().round() as u64)).unwrap();
                        self.osu_clock.unpause();
                        self.sink.play();
                    }
                }
            }
        });

        let output = self.egui.state.egui_ctx().end_frame();

        self.egui.state.handle_platform_output(
            &self.window,
            output.platform_output.to_owned(),
        );

        self.egui.output = Some(output);
    }

    pub fn process_inputs(&mut self, process_time: f64) {
        self.input_buffer.iter().for_each(|i| {
            assert!(i.ts <= process_time);
        });

        'input_loop: for input in &self.input_buffer {
            self.current_input_state.update(input);
            if !self.current_input_state.is_key_hit() {
                continue 'input_loop;
            }

            let input_time = input.ts;

            'obj_loop: for obj in self.hit_objects.iter_mut() {
                match &mut obj.kind {
                    ObjectKind::Circle(circle) => {
                        // If holding dont even try to process input for circle
                        if self.current_input_state.is_holding() {
                            continue 'obj_loop;
                        }

                        if circle.hit_result.is_none() {
                            let result = circle.is_hittable(
                                input_time, 
                                &self.current_hit_window, 
                                self.current_input_state.cursor.into(),
                                self.current_hit_circle_diameter
                            );

                            if let Some(result) = result {
                                tracing::info!("hit res: {:?} div: {} | c: {:.2} i: {:.2}", 
                                    result, obj.start_time - input_time, circle.start_time, input_time
                                );
                                circle.hit_result = Some(HitResult::Hit {
                                    pos: self.current_input_state.cursor.into(),
                                    at: input_time,
                                    result,
                                });

                                self.current_input_state.set_no_hit();

                                break 'obj_loop;
                            }
                        }
                    },
                    ObjectKind::Slider(_) => {},
                }
            }
        }

        self.input_buffer.clear();
    }

    // Going through every object on beatmap and preparing it to
    // assigned buffers
    pub fn prepare_objects_for_renderer(&mut self, time: f64) {
        let _span = tracy_client::span!("osu_state prepare objects");

        for (i, obj) in self.hit_objects.iter_mut().enumerate().rev() {
            if obj.is_judgements_visible(time, self.preempt) {
                self.objects_judgments_render_queue.push(i);
            };

            if !obj.is_visible(time, self.preempt, &self.current_hit_window) {
                continue;
            }

            match &mut obj.kind {
                ObjectKind::Slider(slider) => {
                    self.osu_renderer.prepare_and_render_slider_texture(slider, &self.skin_manager, &self.config);
                }
                _ => {},
            }

            self.objects_render_queue.push(i);
        }

        self.osu_renderer.prepare_judgements(time, &self.objects_judgments_render_queue, &self.hit_objects);

        self.osu_renderer.prepare_objects(
            time, self.preempt, self.fadein,
            &self.objects_render_queue, &self.hit_objects,
            &self.skin_manager
        );

        // Syncing osu state settings with the osu renderer
        self.osu_renderer.prepare(
            &self.config
        );

        // When we are done preparing all objects for rendering
        // we should not forget to upload all that to gpu
        self.osu_renderer.write_buffers();
    }
    
    pub fn update(&mut self) {
        let _span = tracy_client::span!("osu_state update");
        self.cursor_renderer.update();

        //tracing::info!("{:.?}", self.sink.get_pos().as_secs_f64() * 1000.0 - self.osu_clock.get_time());
        
        // Recv all events
        let event = self.event_receiver.try_recv();
        match event {
            Ok(event) => {
                match event {
                    OsuStateEvent::ChangeSkin(path) => {
                        self.open_skin(path)
                    },
                    OsuStateEvent::StartBeatmap(entry) => {
                        tracing::info!("Request to enter beatmap");
                        self.open_beatmap(entry.path);
                        self.current_state = OsuStates::Playing;
                    },
                    OsuStateEvent::ToSongSelection => {
                        self.osu_clock.reset_time();
                        self.input_buffer.clear();
                        self.current_input_state.clear();
                        self.current_state = OsuStates::SongSelection;
                    },
                    OsuStateEvent::PlaySound(start_at, audio_source) => {
                        self.sink.clear();
                        self.sink.append(audio_source);
                        self.sink.try_seek(Duration::from_millis(start_at.try_into().unwrap_or(0))).unwrap();
                        self.sink.play();
                    },
                }
            },
            Err(TryRecvError::Empty) => {},
            _ => panic!("sender disconnected"),
        }

        // egui inputs
        //let input = self.egui.state.take_egui_input(&self.window);

        match self.current_state {
            OsuStates::Playing => {},
            OsuStates::SongSelection => {
                self.song_select.update();
            },
        }

    }

    pub fn render_egui(&mut self, view: &TextureView) -> Result<(), wgpu::SurfaceError> {
        let graphics = self.osu_renderer.get_graphics();
        let mut encoder = graphics
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.egui.render(&graphics, &mut encoder, &view)?;

        graphics.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {

        //println!("diff: {}", self.osu_clock.get_time() as u128 - self.sink.get_pos().as_millis());

        //let graphics = self.osu_renderer.get_graphics();
        let output = self.osu_renderer.get_graphics().get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let egui_input = self.egui.state.take_egui_input(&self.window);

        match self.current_state {
            OsuStates::Playing => {

                self.prepare_objects_for_renderer(self.osu_clock.get_time());

                // TODO THIS SHOULN'T BE HERE, fix when dicided what to
                // do with egui_input thing
                self.update_egui(egui_input);

                self.osu_renderer.render_objects(
                    &view,
                    &self.objects_render_queue, &self.hit_objects,
                    &self.skin_manager,
                )?;

                // Clearing objects queue only after they successfully rendered
                self.objects_render_queue.clear();
                self.objects_judgments_render_queue.clear();
                self.render_egui(&view)?;

                //self.render_playing(&view);

                self.osu_clock.update();
                self.process_inputs(self.osu_clock.get_time());
            },
            OsuStates::SongSelection => {
                let egui_output = self.song_select.render(
                    egui_input, 
                    self.egui.state.egui_ctx(),
                    &view
                );
                self.render_egui(&view)?;
                self.egui.output = Some(egui_output)
            },
        }

        self.cursor_renderer.render_on_view(
            &view,
            &self.skin_manager
        );

        output.present();

        Ok(())
    }
}
