use std::{fs::File, io::BufReader, path::{Path, PathBuf}, sync::{mpsc::{channel, Receiver, Sender, TryRecvError}, Arc, RwLock}, time::Duration};

use cgmath::Vector2;
use egui::{RawInput, Slider};
use rosu_map::Beatmap;
use soloud::{audio, AudioExt, Handle, LoadExt, Soloud, Wav};
use wgpu::TextureView;
use winit::{dpi::{PhysicalPosition, PhysicalSize}, keyboard::KeyCode, window::Window};

use crate::{
    config::Config, egui_state::EguiState, graphics::Graphics, hit_objects::{hit_window::HitWindow, Object, ObjectKind}, math::{calc_playfield, calculate_preempt_fadein, calc_hitcircle_diameter}, renderer::cursor::CursorRenderer, osu_db::DbBeatmapEntry, osu_input::KeyboardState, osu_renderer::OsuRenderer, skin_manager::SkinManager, song_select_state::SongSelectionState, timer::Timer
};
use crate::processor::OsuProcessor;

pub enum OsuStates {
    Playing,
    SongSelection,
}

pub enum OsuStateEvent {
    ToSongSelection,
    SetCursorSize(f32),
    ChangeSkin(PathBuf),
    StartBeatmap(Arc<DbBeatmapEntry>),
    PlaySound(i32, audio::Wav),
}


pub struct OsuState<'s> {
    pub window: Arc<Window>,
    pub egui: EguiState,
    pub event_receiver: Receiver<OsuStateEvent>,
    pub event_sender: Sender<OsuStateEvent>,

    pub sl: Soloud,

    pub current_state: OsuStates,
    current_beatmap: Option<Beatmap>,
    current_hit_window: HitWindow,
    current_screen_size: Vector2<f32>,
    current_hit_circle_diameter: f32,
    current_audio: Option<Wav>,
    current_playing_audio: Option<Handle>,

    pub song_select: SongSelectionState<'s>,

    skin_manager: Arc<RwLock<SkinManager>>,

    osu_renderer: OsuRenderer<'s>,

    preempt: f32,
    fadein: f32,

    hit_objects: Vec<Object>,

    objects_render_queue: Vec<usize>,
    objects_judgments_render_queue: Vec<usize>,

    osu_clock: Timer,
    
    cursor_renderer: CursorRenderer<'s>,

    input_processor: OsuProcessor,
}

impl<'s> OsuState<'s> {
    pub fn new(window: Arc<Window>, graphics: Graphics<'s>, sl: Soloud) -> Self {
        let egui = EguiState::new(&graphics, &window);
        let skin_manager = Arc::new(RwLock::new(
            SkinManager::from_path("skin", &graphics)
        ));

        let config = Arc::new(RwLock::new(Config::default()));
        let graphics = Arc::new(graphics);

        let osu_renderer = OsuRenderer::new(graphics.clone(), config.clone(), skin_manager.clone());

        let (event_sender, event_receiver) = channel::<OsuStateEvent>();

        let song_select = SongSelectionState::new(
            graphics.clone(), 
            event_sender.clone(),
            config.clone(),
            skin_manager.clone()
        );

        window.set_cursor_visible(false);

        Self {
            cursor_renderer: CursorRenderer::new(graphics.clone(), skin_manager.clone()),
            event_receiver,
            preempt: 0.0,
            fadein: 0.0,
            osu_renderer,
            window,
            current_beatmap: None,
            egui,
            sl,
            osu_clock: Timer::new(),
            objects_render_queue: Vec::with_capacity(20),
            hit_objects: Vec::new(),
            skin_manager,
            current_state: OsuStates::SongSelection,
            song_select,
            event_sender,
            input_processor: OsuProcessor::default(),
            current_hit_window: Default::default(),
            current_screen_size: Vector2::new(1.0, 1.0),
            current_hit_circle_diameter: 1.0,
            objects_judgments_render_queue: Vec::new(),
            current_audio: None,
            current_playing_audio: None,
        }
    }

    pub fn open_skin(&mut self, path: impl AsRef<Path>) {
        let _span = tracy_client::span!("osu_state::open_skin");
        let mut lock = self.skin_manager.write().expect("failed to acquire lock");

        let skin = SkinManager::from_path(path, &self.osu_renderer.get_graphics());

        *lock = skin;
    }

    pub fn open_beatmap(&mut self, path: impl AsRef<Path>) {
        let _span = tracy_client::span!("osu_state::open_beatmap");
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

        if let Some(audio_handle) = self.current_playing_audio.take() {
            self.sl.pause(audio_handle);
        }

        let beatmap_dir = path.as_ref().parent().expect("failed to get beatmap dir");
        let audio_file = beatmap_dir.join(&map.audio_file);
        
        // We have to acknowlage the fact that there might be beatmaps
        // without any audio files
        if audio_file.is_file() {
            let mut wav = audio::Wav::default();
            wav.load(audio_file).unwrap(); // TODO handle error
            self.set_audio(wav);
            tracing::info!("Initialized a new audio file!");
        }

        let (preempt, fadein) = calculate_preempt_fadein(map.approach_rate);
        let hit_window = HitWindow::from_od(map.overall_difficulty);

        self.preempt = preempt;
        self.fadein = fadein;
        self.current_hit_window = hit_window;

        // Convert rosu_map to our objects
        let out_objects = Object::from_rosu(&map);

        self.hit_objects = out_objects;

        self.current_beatmap = Some(map);
        self.apply_beatmap_transformations();

        if let Some(audio) = &self.current_audio {
            self.current_playing_audio = Some(self.sl.play(audio));
        }
    }

    pub fn set_audio(&mut self, audio: Wav) {
        let _span = tracy_client::span!("osu_state::set_audio");
        self.current_audio = Some(audio);
    }

    pub fn apply_beatmap_transformations(&mut self) {
        let _span = tracy_client::span!("osu_state::apply_beatmap_transformations");
        let cs = match &self.current_beatmap {
            Some(beatmap) => beatmap.circle_size,
            None => 4.0,
        };

        self.osu_renderer.on_cs_change(cs);
        self.current_hit_circle_diameter = calc_hitcircle_diameter(cs);
    }

    pub fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        let _span = tracy_client::span!("osu_state::resize");
        self.current_screen_size.x = new_size.width as f32;
        self.current_screen_size.y = new_size.height as f32;

        self.cursor_renderer.on_resize(new_size);
        self.osu_renderer.on_resize(new_size);
        self.song_select.on_resize(new_size);
    }

    pub fn on_pressed_down(
        &mut self, 
        key_code: KeyCode, 
        is_cntrl_pressed: bool
    ) {
        let _span = tracy_client::span!("osu_state::on_pressed_down");
        match self.current_state {
            OsuStates::Playing => {
                if key_code == KeyCode::Escape {
                    self.event_sender.send(OsuStateEvent::ToSongSelection)
                        .expect("Failed to send ToSongSelection event to the OsuState");
                }
                
                let ts = self.osu_clock.since_start();

                if key_code == KeyCode::KeyZ {
                    let state = KeyboardState {
                        k1: true,
                        k2: false,
                    };

                    self.input_processor.store_keyboard_pressed(ts, state);
                }

                if key_code == KeyCode::KeyX {
                    let state = KeyboardState {
                        k1: false,
                        k2: true,
                    };

                    self.input_processor.store_keyboard_pressed(ts, state);
                }
            },
            OsuStates::SongSelection => {
                self.song_select.on_pressed_down(key_code, is_cntrl_pressed);
            },
        }
    }

    pub fn on_pressed_release(&mut self, key_code: KeyCode) {
        let _span = tracy_client::span!("osu_state::on_pressed_release");
        match self.current_state {
            OsuStates::Playing => {

                let ts = self.osu_clock.since_start();
                if key_code == KeyCode::KeyZ {
                    let state = KeyboardState {
                        k1: true,
                        k2: false,
                    };

                    self.input_processor.store_keyboard_released(ts, state);
                }

                if key_code == KeyCode::KeyX {
                    let state = KeyboardState {
                        k1: false,
                        k2: true,
                    };

                    self.input_processor.store_keyboard_released(ts, state);
                }
            }
            _ => {}
        };
    }

    pub fn on_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let _span = tracy_client::span!("osu_state::on_cursor_moved");
        self.cursor_renderer.on_cursor_moved(position);

        match self.current_state {
            OsuStates::Playing => {
                let ts = self.osu_clock.since_start();

                let mut recv_pos = Vector2::new(position.x as f32, position.y as f32);
                let (scale, offsets) = calc_playfield(self.current_screen_size.x, self.current_screen_size.y);

                recv_pos -= offsets;
                recv_pos /= scale;
                
                let pos = Vector2::new(recv_pos.x as f64, recv_pos.y as f64);

                self.input_processor.store_cursor_moved(ts, pos);
            },
            _ => {},
        }
    }

    pub fn update_egui(&mut self, input: RawInput) {
        let _span = tracy_client::span!("osu_state::update_egui");

        self.egui.state.egui_ctx().begin_pass(input);

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
                    //self.sink.try_seek(Duration::from_millis(self.osu_clock.get_time().round() as u64)).unwrap();
                    self.osu_clock.unpause();
                };

                if !self.osu_clock.is_paused() {
                    if ui.add(egui::Button::new("pause")).clicked() {
                        self.osu_clock.pause();
                        //self.sink.pause();
                    }
                } else {
                    if ui.add(egui::Button::new("unpause")).clicked() {
                        //self.sink.try_seek(Duration::from_millis(self.osu_clock.get_time().round() as u64)).unwrap();
                        self.osu_clock.unpause();
                        //self.sink.play();
                    }
                }
            }
        });

        let output = self.egui.state.egui_ctx().end_pass();

        self.egui.state.handle_platform_output(
            &self.window,
            output.platform_output.to_owned(),
        );

        self.egui.output = Some(output);
    }

    pub fn process_inputs(&mut self, _process_time: f64) {
        let _span = tracy_client::span!("osu_state::process_inputs");
    }

    // Going through every object on beatmap and preparing it to
    // assigned buffers
    pub fn prepare_objects_for_renderer(&mut self, time: f64) {
        let _span = tracy_client::span!("osu_state::prepare_objects_for_renderer");

        for (i, obj) in self.hit_objects.iter_mut().enumerate().rev() {
            self.objects_judgments_render_queue.push(i);

            if !obj.is_visible(time, self.preempt, &self.current_hit_window) {
                continue;
            }

            match &mut obj.kind {
                ObjectKind::Slider(slider) => {
                    self.osu_renderer.prepare_and_render_slider_texture(slider);
                }
                _ => {},
            }

            self.objects_render_queue.push(i);
        }
        
        self.osu_renderer.prepare_judgements(
            time, 
            &self.objects_judgments_render_queue, 
            &self.hit_objects,
        );

        self.osu_renderer.prepare_objects(
            time, self.preempt, self.fadein,
            &self.objects_render_queue, 
            &self.hit_objects,
            &self.current_hit_window
        );

        // Syncing osu state settings with the osu renderer
        self.osu_renderer.prepare();

        // When we are done preparing all objects for rendering
        // we should not forget to upload all that to gpu
        self.osu_renderer.write_buffers();
    }
    
    pub fn update(&mut self) {
        let _span = tracy_client::span!("osu_state::update");
        self.cursor_renderer.update();

        // Recv all events
        let event = self.event_receiver.try_recv();

        match event {
            Ok(event) => {
                match event {
                    OsuStateEvent::SetCursorSize(new_size) => {
                        self.cursor_renderer.set_size(new_size);
                    },
                    OsuStateEvent::ChangeSkin(path) => {
                        let _span = tracy_client::span!("osu_state::update::event::change_skin");
                        self.open_skin(path)
                    },
                    OsuStateEvent::StartBeatmap(entry) => {
                        let _span = tracy_client::span!("osu_state::update::event::start_beatmap");
                        self.open_beatmap(&entry.path);
                        self.current_state = OsuStates::Playing;
                    },
                    OsuStateEvent::ToSongSelection => {
                        let _span = tracy_client::span!("osu_state::update::event::to_song_selection");
                        self.osu_clock.reset_time();
                        self.current_state = OsuStates::SongSelection;
                    },
                    OsuStateEvent::PlaySound(start_at, audio_source) => {
                        if let Some(audio_handle) = self.current_playing_audio.take() {
                            self.sl.stop(audio_handle);
                        };

                        let handle = self.sl.play(&audio_source);
                        self.sl.set_pause(handle, true);
                        let seek_to = (start_at as f64 / 1000.0).max(0.0);
                        self.sl.seek(handle, seek_to).unwrap(); // TODO: Handle
                        self.sl.set_pause(handle, false);

                        self.current_playing_audio = Some(handle);
                        self.current_audio = Some(audio_source);
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
        let _span = tracy_client::span!("osu_state::render_egui");

        let graphics = self.osu_renderer.get_graphics();

        self.egui.render(&graphics, &view)?;


        Ok(())
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let _span = tracy_client::span!("osu_state::render");

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
                //self.update_egui(egui_input);

                self.osu_renderer.render_objects(
                    &view,
                    &self.objects_render_queue, 
                    &self.hit_objects
                )?;

                // Clearing objects queue only after they successfully rendered
                self.objects_render_queue.clear();
                self.objects_judgments_render_queue.clear();
                //self.render_egui(&view)?;

                //self.render_playing(&view);

                let time = self.osu_clock.update() / 1000.0;
                if let Some(audio_handle) = self.current_playing_audio {
                    let pos = self.sl.stream_position(audio_handle);

                    let diff = pos - time;
                    //tracing::info!("Audio vs Time pos: {diff}");
                    let diff_abs = diff.abs();
                    
                    // Applying simple time correction if 
                    // 9ms threeshold is hit
                    if diff_abs * 1000.0 >= 9.0 {
                        if diff > 0.0 {
                            self.osu_clock.last_time += diff_abs;
                        } else if diff < 0.0 {
                            self.osu_clock.last_time -= diff_abs;
                        }
                    }


                }

                self.input_processor.process_all(
                    &mut self.hit_objects,
                    &self.current_hit_window,
                    self.current_hit_circle_diameter
                );
            },
            OsuStates::SongSelection => {
                let egui_output = self.song_select.render(
                    egui_input, 
                    self.egui.state.egui_ctx(),
                    &view,
                );
                self.render_egui(&view)?;
                self.egui.output = Some(egui_output)
            },
        }

        self.cursor_renderer.render_on_view(
            &view
        );

        output.present();

        Ok(())
    }
}
