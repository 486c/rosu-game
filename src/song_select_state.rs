use std::{fs::File, io::{Cursor, Read}, path::PathBuf, sync::{mpsc::{Receiver, Sender}, Arc, RwLock}, time::Duration};

use image::{DynamicImage, ImageReader};
use md5::Digest;
use rand::Rng;
use rosu_map::Beatmap;
use soloud::{audio, AudioExt, LoadExt};
use wgpu::TextureView;
use winit::{dpi::PhysicalSize, keyboard::KeyCode};

use crate::{config::Config, graphics::Graphics, osu_db::{DbBeatmapEntry, OsuDatabase, DEFAULT_DB_PATH}, osu_state::OsuStateEvent, screen::{settings::SettingsScreen, song_select::{BeatmapCardInfoMetadata, CurrentAudio, CurrentBeatmap, SongSelectScreen}}, skin_manager::SkinManager};

pub struct SongsImportJob {
    pub path: PathBuf,
    pub stop_rx: oneshot::Receiver<()>,
}

pub enum SongSelectionEvents {
    /// Request to select beatmap from song select screen
    SelectBeatmap(Arc<DbBeatmapEntry>),
    /// When beatmap loading thread is successfully returned a beatmap
    LoadedBeatmap{ 
        beatmap: Beatmap, 
        //beatmap_md5: Digest,
        image: DynamicImage,
        image_md5: Digest,
        audio_source: audio::Wav,
        audio_md5: Digest
    },
    /// Request to start the beatmap
    StartBeatmap(Arc<DbBeatmapEntry>),
    ImportSongsDirectory(SongsImportJob),
    ToggleSettings,
    CloseSettings,
}

pub struct SongSelectionState<'ss> {
    db: Arc<OsuDatabase>,

    // SongSelection state senders, used by
    // components inside song selection
    inner_tx: Sender<SongSelectionEvents>,
    inner_rx: Receiver<SongSelectionEvents>,

    current_audio: Option<CurrentAudio>,
    
    // Events sender for "god" state
    state_tx: Sender<OsuStateEvent>,

    settings: SettingsScreen,
    song_select_screen: SongSelectScreen<'ss>,

    worker_tx: Sender<DbBeatmapEntry>,
}

impl<'ss> SongSelectionState<'ss> {
    pub fn new(
        graphics: Arc<Graphics<'ss>>, 
        state_tx: Sender<OsuStateEvent>,
        config: Arc<RwLock<Config>>,
        skin_manager: Arc<RwLock<SkinManager>>,
    ) -> Self {
        let (inner_tx, inner_rx) = std::sync::mpsc::channel();
        let (worker_tx, worker_rx) = std::sync::mpsc::channel::<DbBeatmapEntry>();

        let db: Arc<OsuDatabase> = OsuDatabase::new_from_path(DEFAULT_DB_PATH)
            .unwrap()
            .into(); // TODO: REMOVE UNRAP

        spawn_beatmap_opener_worker(worker_rx, inner_tx.clone());

        Self {
            db: db.clone(),
            inner_tx: inner_tx.clone(),
            inner_rx,
            state_tx: state_tx.clone(),
            settings: SettingsScreen::new(config.clone(), skin_manager.clone(), state_tx.clone()),
            song_select_screen: SongSelectScreen::new(db.clone(), graphics.clone(), inner_tx.clone()),
            current_audio: None,
            worker_tx,
        }
    }
    
    // Spawns a thread to parse a beatmap
    fn open_beatmap(&self, beatmap: &DbBeatmapEntry) {
        let _span = tracy_client::span!("osu_song_select_state::open_beatmap");

        let _ = self.worker_tx.send(beatmap.clone());
    }

    pub fn on_pressed_down(
        &mut self,
        key_code: KeyCode,
        is_cntrl_pressed: bool
    ) {
        let _span = tracy_client::span!("osu_song_select_state::on_pressed_down");

        if key_code == KeyCode::Enter {
            let current_in_cache = self.song_select_screen.current_in_cache();

            self.inner_tx.send(
                SongSelectionEvents::StartBeatmap(self.db.get_from_cache(current_in_cache).unwrap())
            ).expect(
                "Failed to send StartBeatmap event to the SongSelectState"
            );
        }

        if key_code == KeyCode::F2 {
            let mut rng = rand::thread_rng();

            let random_beatmap = rng.gen_range(0..self.db.beatmaps_amount());

            self.song_select_screen.set_scroll_to(random_beatmap);
        }

        if key_code == KeyCode::ArrowDown || key_code == KeyCode::ArrowRight {
            self.song_select_screen.increment_beatmap();
        }

        if key_code == KeyCode::ArrowUp || key_code == KeyCode::ArrowLeft {
            self.song_select_screen.decrement_beatmap();
        }

        if key_code == KeyCode::KeyO && is_cntrl_pressed {
            let _ = self.inner_tx.send(SongSelectionEvents::ToggleSettings);
        }

        if key_code == KeyCode::Escape && self.settings.is_open() {
            let _ = self.inner_tx.send(SongSelectionEvents::CloseSettings);
        }
    }
    

    
    #[inline]
    fn load_background(&mut self, image: DynamicImage, md5: Digest) {
        let _span = tracy_client::span!("osu_song_select_state::load_background");

        self.song_select_screen.set_background(image, md5);

    }
    
    #[inline]
    fn load_audio(
        &mut self, 
        audio_source: audio::Wav,
        md5: md5::Digest,
        beatmap: &Beatmap,
    ) {
        let _span = tracy_client::span!("osu_song_select_state::load_audio");

        // If current audio is the same do nothing
        if let Some(current_audio) = &self.current_audio {
            if current_audio.audio_hash == md5 {
                return;
            }
        };

        self.state_tx.send(OsuStateEvent::PlaySound(
                beatmap.preview_time,
                audio_source,
        )).expect(
            "Failed to send PlaySound event to the OsuState"
        );

        self.current_audio = Some(CurrentAudio {
            audio_hash: md5,
        });
    }

    pub fn update(&mut self) {
        let _span = tracy_client::span!("osu_song_select_state::update");
        match self.inner_rx.try_recv() {
            Ok(event) => {
                match event {
                    SongSelectionEvents::SelectBeatmap(entry) => {
                        let _span = tracy_client::span!("osu_song_select_state::update::event::select_beatmap");
                        self.open_beatmap(&entry);
                    },
                    SongSelectionEvents::LoadedBeatmap{ mut beatmap, image, audio_source, image_md5, audio_md5, .. }  => {
                        let _span = tracy_client::span!("osu_song_select_state::update::event::loaded_beatmap");
                        self.load_background(image, image_md5);
                        self.load_audio(audio_source, audio_md5, &beatmap);

                        let metadata = BeatmapCardInfoMetadata::from_beatmap(&mut beatmap);

                        let current_beatmap = CurrentBeatmap {
                            metadata,
                        };

                        self.song_select_screen.set_current_beatmap(Some(current_beatmap));
                    },
                    SongSelectionEvents::ToggleSettings => {
                        self.settings.toggle();
                    },
                    SongSelectionEvents::CloseSettings => {
                        self.settings.close();
                    },
                    SongSelectionEvents::StartBeatmap(entry) => {
                        let _span = tracy_client::span!("osu_song_select_state::update::event::start_beatmap");
                        self.settings.close();
                        self.state_tx.send(OsuStateEvent::StartBeatmap(entry))
                            .expect("Failed to send StartBeatmap event to the OsuState");
                    },
                    SongSelectionEvents::ImportSongsDirectory(job) => {
                        let _span = tracy_client::span!("osu_song_select_state::update::event::import_songs_directory");
                        self.db.scan_beatmaps(job.path, job.stop_rx);
                    },
                }
            },
            Err(e) => match e {
                std::sync::mpsc::TryRecvError::Empty => {},
                std::sync::mpsc::TryRecvError::Disconnected => {
                    tracing::error!("Channel is closed!")
                },
            },
        }
    }

    pub fn on_resize(&mut self, new_size: &PhysicalSize<u32>) {
        self.song_select_screen.on_resize(new_size);
    }

    pub fn render(
        &mut self, 
        input: egui::RawInput, 
        ctx: &egui::Context, 
        view: &TextureView,
    ) -> egui::FullOutput {
        ctx.begin_pass(input);

        self.settings.render(ctx);
        self.song_select_screen.render(ctx, view);

        ctx.end_pass()
    }
}

/// Worker for opening requested beatmaps
fn spawn_beatmap_opener_worker(
    worker_rx: Receiver<DbBeatmapEntry>, 
    song_select_tx: Sender<SongSelectionEvents>
) {
    std::thread::spawn(move || {
        loop {
            let res = worker_rx.try_recv();

            match res {
                Ok(job) => {
                    let _span = tracy_client::span!("osu_song_select_state::open_beatmap_thread");
                    let path = job.path;

                    tracing::info!("Starting opening beatmap for path {}", path.display());

                    // Beatmap stuff
                    let mut beatmap_file = File::open(&path).unwrap();
                    let mut beatmap_buffer = Vec::new();
                    beatmap_file.read_to_end(&mut beatmap_buffer).unwrap();

                    let _beatmap_md5 = md5::compute(&beatmap_buffer);

                    let parsed_beatmap = Beatmap::from_bytes(&beatmap_buffer).unwrap();

                    let bg_filename = parsed_beatmap.background_file.clone();
                    let audio_filename = parsed_beatmap.audio_file.clone();

                    let bg_path = path.parent()
                        .unwrap()
                        .join(&bg_filename);

                    let audio_path = path.parent()
                        .unwrap()
                        .join(audio_filename);

                    if bg_path.is_dir() {
                        tracing::error!(
                            "Trying to read dir as background: {} for map {}",
                            bg_path.display(),
                            path.display()
                        );

                        continue;
                    }

                    // BG image stuff
                    let mut bg_file = File::open(bg_path).unwrap();
                    let mut bg_buffer = Vec::new();
                    bg_file.read_to_end(&mut bg_buffer).unwrap();
                    let bg_md5 = md5::compute(&bg_buffer);

                    let bg_buffer = Cursor::new(bg_buffer);

                    let img_reader = ImageReader::new(bg_buffer)
                        .with_guessed_format().unwrap();

                    let img = img_reader.decode().unwrap();
                    let img = img.blur(5.0);

                    // Audio file stuff
                    let mut audio_file = File::open(audio_path).unwrap();
                    let mut audio_buffer = Vec::new();
                    audio_file.read_to_end(&mut audio_buffer).unwrap();

                    let audio_md5 = md5::compute(&audio_buffer);

                    let mut wav = audio::Wav::default();
                    wav.load_mem(&audio_buffer).unwrap(); // TODO: Handle error

                    let _ = song_select_tx.send(SongSelectionEvents::LoadedBeatmap{
                        beatmap: parsed_beatmap,
                        image: img,
                        image_md5: bg_md5,
                        audio_source: wav,
                        audio_md5
                    });
                },
                Err(e) => match e {
                    std::sync::mpsc::TryRecvError::Empty => continue,
                    std::sync::mpsc::TryRecvError::Disconnected => {
                        panic!("disconnected");
                    },
                },
            }
        }
    });
}
