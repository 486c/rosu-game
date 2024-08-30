use std::{fs::File, io::{BufReader, Cursor, Read}, sync::{mpsc::{Receiver, Sender}, Arc}, time::Duration};

use egui::{scroll_area::ScrollBarVisibility, Align, Color32, Label, Margin, Pos2, Rect, RichText, Stroke};
use egui_extras::{Size, StripBuilder};
use image::DynamicImage;
use md5::Digest;
use rand::Rng;
use rodio::{source::UniformSourceIterator, Decoder, Source};
use rosu_map::Beatmap;
use wgpu::{util::DeviceExt, BufferUsages, TextureView};
use winit::{dpi::PhysicalSize, keyboard::KeyCode};

use crate::{buffer_write_or_init, camera::Camera, graphics::Graphics, hit_circle_instance::HitCircleInstance, osu_db::{BeatmapEntry, OsuDatabase}, osu_renderer::QUAD_INDECIES, osu_state::OsuStateEvent, quad_instance::QuadInstance, quad_renderer::QuadRenderer, rgb::Rgb, texture::Texture, vertex::Vertex};


const CARD_INNER_MARGIN: Margin = Margin {
    left: 5.0,
    right: 0.0,
    top: 8.0,
    bottom: 0.0,
};

const ROW_HEIGHT: f32 = 72.0;

// A struct that contains beatmap metadata
// Build only once when loading beatmap because
// calculating all the stuff + reallocating new strings
pub struct BeatmapCardInfoMetadata {
    /// `{} - {} [{}]`
    beatmap_header: String,

    /// `Mapped by {}`
    mapped_by: String,

    /// `Length: {} BPM: {}-{} Objects: {}`
    length_info: String,
    
    /// `Circles: {} Sliders: {} Spinners: {}`
    objects_count: String,

    /// `CS: {} AR: {} OD: {} HP: {} Start: {}`
    difficutly_info: String,
}

impl BeatmapCardInfoMetadata {
    pub fn from_beatmap(b: &mut Beatmap) -> Self {
        let last_hitobject_time = if let Some(obj) = b.hit_objects.last_mut() {
            obj.end_time().clone() as u64
        } else {
            0
        };

        let length = Duration::from_millis(last_hitobject_time);

        let length_str = format!(
            "{:02}:{:02}",
            length.as_secs() / 60,
            length.as_secs() % 60
        );

        let (bpm_max, bpm_min) = {
            let mut max: f64 = f64::MIN;
            let mut min: f64 = f64::MAX;

            for point in &b.control_points.timing_points {
                let bpm = 1.0 / point.beat_len * 1000.0 * 60.0;

                max = max.max(bpm);
                min = max.min(bpm);
            }

            (max, min)
        };

        let length_info = format!(
            "Length: {} BPM: {:.0}-{:.0} Objects: {}",
            length_str, 
            bpm_min, bpm_max,
            b.hit_objects.len() 
        );

        let circles = b.hit_objects.iter().filter(|h| {
            match h.kind {
                rosu_map::section::hit_objects::HitObjectKind::Circle(_) => true,
                _ => false,
            }
        }).count();

        let sliders = b.hit_objects.iter().filter(|h| {
            match h.kind {
                rosu_map::section::hit_objects::HitObjectKind::Slider(_) => true,
                _ => false,
            }
        }).count();

        let spinners = b.hit_objects.iter().filter(|h| {
            match h.kind {
                rosu_map::section::hit_objects::HitObjectKind::Spinner(_) => true,
                _ => false,
            }
        }).count();

        let difficutly_info = format!(
            "CS:{:.2} AR:{:.2} OD:{:.2} HP:{:.2} Stars: TODO",
            b.circle_size, b.approach_rate, b.overall_difficulty, b.hp_drain_rate
        );

        Self {
            beatmap_header: format!("{} - {} [{}]", b.artist, b.title, b.version),
            mapped_by: format!("Mapped by {}", b.creator),
            length_info,
            objects_count: format!("Circles: {} Sliders: {} Spinners: {}", circles, sliders, spinners),
            difficutly_info,
        }
    }
}

// TODO move to some other place
pub struct CurrentBeatmap {
    beatmap: Beatmap,
    metadata: BeatmapCardInfoMetadata,
    beatmap_hash: md5::Digest,
}

pub struct CurrentBackground {
    texture: Texture,
    image_hash: md5::Digest,
}

pub struct CurrentAudio {
    audio_hash: md5::Digest,
}

enum SongSelectionEvents {
    SelectBeatmap(BeatmapEntry),
    LoadedBeatmap{ 
        beatmap: Beatmap, 
        beatmap_md5: Digest,
        image: DynamicImage,
        image_md5: Digest,
        audio_source: Box<dyn Source<Item = f32> + Send + Sync>,
        audio_md5: Digest
    },
    StartBeatmap(BeatmapEntry),
}

pub struct SongSelectionState<'ss> {
    db: OsuDatabase,
    graphics: Arc<Graphics<'ss>>,

    // Min & Max row that we currently need to draw
    min: usize,
    max: usize,

    // Current selected row
    current: usize,

    // Stupid states
    need_scroll_to: Option<usize>,

    current_beatmap: Option<CurrentBeatmap>,
    current_background_image: Option<CurrentBackground>,
    current_audio: Option<CurrentAudio>,

    // SongSelection state senders, used by
    // components inside song selection
    inner_tx: Sender<SongSelectionEvents>,
    inner_rx: Receiver<SongSelectionEvents>,
    
    // Events sender for "god" state
    state_tx: Sender<OsuStateEvent>,

    quad_renderer: QuadRenderer<'ss>,
    quad_test_buffer: wgpu::Buffer,
    quad_test_instance_data: Vec<QuadInstance>,
}

impl<'ss> SongSelectionState<'ss> {
    pub fn new(graphics: Arc<Graphics<'ss>>, state_tx: Sender<OsuStateEvent>) -> Self {
        let (inner_tx, inner_rx) = std::sync::mpsc::channel();

        let quad_renderer = QuadRenderer::new(graphics.clone());

        let quad_test_buffer = quad_renderer.create_instance_buffer();
        let quad_test_instance_data = Vec::new();

        Self {
            db: OsuDatabase::new().unwrap(), // TODO: REMOVE UNRAP
            min: 0,
            max: 0,
            current: 0,
            inner_tx,
            inner_rx,
            current_beatmap: None,
            graphics,
            current_background_image: None,
            state_tx,
            need_scroll_to: None,
            current_audio: None,
            quad_renderer,
            quad_test_buffer,
            quad_test_instance_data,
        }
    }
    
    // Spawns a thread to parse a beatmap
    fn open_beatmap(&self, beatmap: &BeatmapEntry) {
        let tx = self.inner_tx.clone();
        let path = beatmap.path.clone();
        
        // 1. Parse .osu file
        // 2. Load and decode image & apply blur
        // 3. Load and decode audio file
        std::thread::spawn(move || {
            // Beatmap stuff
            let mut beatmap_file = File::open(&path).unwrap();
            let mut beatmap_buffer = Vec::new();
            beatmap_file.read_to_end(&mut beatmap_buffer).unwrap();

            let beatmap_md5 = md5::compute(&beatmap_buffer);

            let parsed_beatmap = Beatmap::from_bytes(&beatmap_buffer).unwrap();

            let bg_filename = parsed_beatmap.background_file.clone();
            let audio_filename = parsed_beatmap.audio_file.clone();

            let bg_path = path.parent()
                .unwrap()
                .join(&bg_filename);

            let audio_path = path.parent()
                .unwrap()
                .join(audio_filename);
        
            // BG image stuff
            let mut bg_file = File::open(bg_path).unwrap();
            let mut bg_buffer = Vec::new();
            bg_file.read_to_end(&mut bg_buffer).unwrap();

            let bg_md5 = md5::compute(&bg_buffer);
        
            let img = image::load_from_memory(&bg_buffer).unwrap();
            let img = img.blur(5.0);
            
            // Audio file stuff
            let mut audio_file = File::open(audio_path).unwrap();
            let mut audio_buffer = Vec::new();
            audio_file.read_to_end(&mut audio_buffer).unwrap();

            let audio_md5 = md5::compute(&audio_buffer);

            let audio_file = Cursor::new(audio_buffer);

            let audio_source = UniformSourceIterator::new(Decoder::new(audio_file).unwrap(), 2, 48000)
                .fade_in(Duration::from_millis(150));

            tx.send(SongSelectionEvents::LoadedBeatmap{
                beatmap: parsed_beatmap,
                beatmap_md5,
                image: img,
                image_md5: bg_md5,
                audio_source: Box::new(audio_source),
                audio_md5
            })
        });
    }


    pub fn on_resize(&mut self, new_size: &PhysicalSize<u32>) {
        self.quad_renderer.resize_camera(new_size);

        if let Some(bg) = &self.current_background_image {
            self.resize_background_vertex(bg.texture.width, bg.texture.height);
        }
        
        // New quad
        self.quad_test_instance_data.clear();
        self.quad_test_instance_data.push(
            QuadInstance::from_xy_pos(
                new_size.width as f32 / 2.0,
                new_size.height as f32 / 2.0,
            )
        );

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.quad_test_buffer,
            &self.quad_test_instance_data,
            QuadInstance
        );
    }

    pub fn on_pressed_down(&mut self, key_code: KeyCode) {
        if key_code == KeyCode::Enter {
            let current_in_cache = self.current - self.min;

            let _ = self.inner_tx.send(
                SongSelectionEvents::StartBeatmap(self.db.cache[current_in_cache].clone())
            );
        }

        if key_code == KeyCode::F2 {
            let mut rng = rand::thread_rng();

            let random_beatmap = rng.gen_range(0..self.db.beatmaps_amount());

            self.need_scroll_to = Some(random_beatmap);
        }

        if key_code == KeyCode::ArrowDown {
            self.need_scroll_to = Some(self.current + 1);
        }

        if key_code == KeyCode::ArrowUp {
            self.need_scroll_to = Some(self.current - 1);
        }
    }
    
    fn resize_background_vertex(&self, width: f32, height: f32) {
        let image_width = width;
        let image_height = height;

        let (graphics_width, graphics_height) = self.graphics.get_surface_size();
        let (graphics_width, graphics_height) = (graphics_width as f32, graphics_height as f32);

        let (mut to_width, mut to_height) = (graphics_width, graphics_height);

        let image_ratio = image_width as f32 / image_height as f32;
        let surface_ratio = graphics_width as f32 / graphics_height as f32;

        let (width, height) = (graphics_height * image_ratio, graphics_width / image_ratio);

        if surface_ratio < image_ratio {
            to_width = width;
        } else {
            to_height = height
        };

        self.quad_renderer.resize_vertex_centered(to_width, to_height);

        tracing::info!("Resized background image vertex, width: {}, height: {}", image_width, image_height);
    }

    fn load_background(&mut self, image: DynamicImage, md5: Digest) {
        // Do not preform any operations if background is the same
        if let Some(current_background) = &self.current_background_image {
            if current_background.image_hash == md5 {
                tracing::info!("Background already cached, doing nothing");
                return;
            }
        }

        self.resize_background_vertex(image.width() as f32, image.height() as f32);

        let texture = Texture::from_image(
            image,
            &self.graphics
        );

        let current_background_image = CurrentBackground {
            texture,
            image_hash: md5,
        };

        self.current_background_image = Some(current_background_image);
    }

    fn load_audio(
        &mut self, 
        audio_source: Box<dyn Source<Item = f32> + Send + Sync>, 
        md5: md5::Digest,
        beatmap: &Beatmap,
    ) {
        // If current audio is the same do nothing
        if let Some(current_audio) = &self.current_audio {
            if current_audio.audio_hash == md5 {
                return;
            }
        };

        let _ = self.state_tx.send(OsuStateEvent::PlaySound(
                beatmap.preview_time,
                audio_source,
        ));

        self.current_audio = Some(CurrentAudio {
            audio_hash: md5,
        })
    }

    pub fn update(&mut self) {
        match self.inner_rx.try_recv() {
            Ok(event) => {
                match event {
                    SongSelectionEvents::SelectBeatmap(entry) => {
                        self.open_beatmap(&entry);
                    },
                    SongSelectionEvents::LoadedBeatmap{ mut beatmap, image, audio_source, image_md5, audio_md5, beatmap_md5 }  => {
                        self.load_background(image, image_md5);
                        self.load_audio(audio_source, audio_md5, &beatmap);

                        let metadata = BeatmapCardInfoMetadata::from_beatmap(&mut beatmap);

                        let current_beatmap = CurrentBeatmap {
                            beatmap,
                            beatmap_hash: beatmap_md5,
                            metadata,
                        };

                        self.current_beatmap = Some(current_beatmap);
                    },
                    SongSelectionEvents::StartBeatmap(entry) => {
                        let _ = self.state_tx.send(OsuStateEvent::StartBeatmap(entry));
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

    pub fn render_background(&self, view: &TextureView) {
        if let Some(current_background) = &self.current_background_image {
            self.quad_renderer.render_on_view(
                &view,
                &current_background.texture.bind_group,
                &self.quad_test_buffer,
                1
            );
        }
    }

    pub fn render_beatmap_card_info(&mut self, ui: &mut egui::Ui) {
        egui::Frame::default()
            .rounding(5.0)
            .outer_margin(10.0)
            .inner_margin(5.0)
            .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 255))
            .show(ui, |ui| {

                ui.set_width(ui.available_rect_before_wrap().width());
                ui.set_height(ui.available_rect_before_wrap().height());
                if let Some(b) = &mut self.current_beatmap {
                    ui.add(Label::new(RichText::new(&b.metadata.beatmap_header).heading()).selectable(false));
                    ui.add(Label::new(&b.metadata.mapped_by).selectable(false));

                    ui.add(Label::new(RichText::new(&b.metadata.length_info).strong()).selectable(false));

                    ui.add(Label::new(&b.metadata.objects_count).selectable(false));
                    ui.add(Label::new(&b.metadata.difficutly_info).selectable(false));
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.spinner();
                    });
                }
            });
    }

    pub fn render(&mut self, input: egui::RawInput, ctx: &egui::Context, view: &TextureView) -> egui::FullOutput {
        self.render_background(view);

        ctx.begin_frame(input);

        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            StripBuilder::new(ui)
                .size(Size::relative(0.6))
                .size(Size::relative(0.4))
                .horizontal(|mut strip| {
                    strip.strip(|builder| {
                        builder
                            .size(Size::relative(0.2))
                            .size(Size::relative(0.8))
                            .vertical(|mut strip| {
                                // INFO ABOUT BEATMAP
                                strip.cell(|ui| {
                                    self.render_beatmap_card_info(ui);
                                });

                                strip.strip(|builder| {
                                    builder
                                        .size(Size::relative(0.9))
                                        .size(Size::relative(0.1))
                                        .vertical(|mut strip| {
                                            strip.cell(|_ui| {});

                                            strip.cell(|ui| {
                                                egui::Frame::default()
                                                    .rounding(5.0)
                                                    .outer_margin(10.0)
                                                    .inner_margin(5.0)
                                                    .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 255))
                                                    .show(ui, |ui| {
                                                        ui.set_width(ui.available_rect_before_wrap().width());
                                                        ui.set_height(ui.available_rect_before_wrap().height());
                                                        ui.centered_and_justified(|ui| {
                                                            let text = format!("Beatmaps: {}", self.db.beatmaps_amount());
                                                            ui.add(Label::new(RichText::new(text).heading())
                                                                .selectable(false));
                                                        })
                                                    });
                                            })
                                        });
                                })
                            });
                    });



                    strip.cell(|ui| {
                        egui::ScrollArea::vertical()
                        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                        .show_viewport(ui, |ui, rect| {
                            let total_height = ROW_HEIGHT * self.db.beatmaps_amount() as f32;
                            ui.set_height(total_height);
                            
                            // Handling custom scrolling event
                            // Cases:
                            //     1. Pressed F2 so we got random beatmap
                            //     2. Pressed ArrowDown/Up so we increment by 1
                            if let Some(need_scroll_to) = self.need_scroll_to.take() {
                                let current_y = self.current as f32 * ROW_HEIGHT;

                                let scroll_y = need_scroll_to as f32 * ROW_HEIGHT;
                                
                                let scroll_y = scroll_y - current_y;
                                self.current = need_scroll_to;
                                
                                ui.scroll_with_delta(
                                    egui::Vec2::new(0.0, -1.0 * scroll_y)
                                );

                                let entry = self.db.get_beatmap_by_index(need_scroll_to);
                                let _ = self.inner_tx.send(
                                    SongSelectionEvents::SelectBeatmap(entry)
                                );
                            }

                            let min_row = (rect.min.y / ROW_HEIGHT).floor() as usize;
                            let max_row = (rect.max.y / ROW_HEIGHT).floor() as usize;


                            let fill_top = (min_row - 0) as f32 * (ROW_HEIGHT);
                            egui::Frame::none()
                                .show(ui, |ui| {
                                    ui.set_height(fill_top);
                                });

                            if max_row != self.max || min_row != self.min {
                                self.db.load_beatmaps_range(min_row, max_row);
                            }

                            let current = min_row;
                            
                            for (i, beatmap) in self.db.cache.iter().enumerate() {
                                let id = current + i;
                                let res = egui::Frame::default()
                                    .inner_margin(CARD_INNER_MARGIN)
                                    .outer_margin(0.0)
                                    .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 250))
                                    .stroke({
                                        if self.current == id {
                                            Stroke::new(1.0, Color32::RED)
                                        } else {
                                            Stroke::new(1.0, Color32::BLACK)
                                        }
                                    })
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_rect_before_wrap().width());
                                        ui.set_height(64.0);
                                        ui.set_max_height(64.0);


                                        ui.add(Label::new(RichText::new(&beatmap.title).heading()).selectable(false));
                                        ui.add(Label::new(format!("{} // {}", &beatmap.artist, &beatmap.creator)).selectable(false));
                                        ui.add(Label::new(&beatmap.version).selectable(false));
                                    });

                                let sense = res.response.interact(egui::Sense::click());

                                if sense.clicked() {
                                    self.current = id;

                                    let _ = 
                                        self.inner_tx.send(
                                            SongSelectionEvents::SelectBeatmap(beatmap.clone())
                                        ); // TODO handle this shit

                                    res.response.scroll_to_me(Some(Align::Center));
                                }

                                if sense.double_clicked() {
                                    self.current = id;
                                    let _ = self.inner_tx.send(
                                        SongSelectionEvents::StartBeatmap(beatmap.clone())
                                    );
                                    res.response.scroll_to_me(Some(Align::Center));
                                }
                            };
                            
                            self.min = min_row;
                            self.max = max_row;
                        });

                    })
                })
        });


        ctx.end_frame()
    }
}
