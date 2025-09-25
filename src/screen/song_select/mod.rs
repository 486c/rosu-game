use std::{sync::Arc, time::Duration};
use std::sync::mpsc::Sender;


use egui::Direction;
use image::DynamicImage;
use md5::Digest;
use wgpu::{util::DeviceExt, BufferUsages, TextureView};
use egui::{scroll_area::ScrollBarVisibility, Align, Color32, Label, Margin, RichText, Stroke};
use egui_extras::{Size, StripBuilder};
use rosu_map::Beatmap;
use winit::dpi::PhysicalSize;

use crate::texture::Texture;
use crate::{graphics::Graphics, osu_db::OsuDatabase, quad_instance::QuadInstance, quad_renderer::QuadRenderer, song_select_state::SongSelectionEvents};

const CARD_INNER_MARGIN: Margin = Margin {
    left: 5,
    right: 0,
    top: 8,
    bottom: 0,
};

const ROW_HEIGHT: f32 = 72.0;

// A struct that contains beatmap metadata
// Build only once when loading beatmap because
// calculating all the stuff + reallocating new strings
pub struct BeatmapCardInfoMetadata {
    // `{} - {} [{}]`
    beatmap_header: String,

    // `Mapped by {}`
    mapped_by: String,

    // `Length: {} BPM: {}-{} Objects: {}`
    length_info: String,
    
    // `Circles: {} Sliders: {} Spinners: {}`
    objects_count: String,

    // `CS: {} AR: {} OD: {} HP: {} Start: {}`
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

pub struct CurrentBeatmap {
    //beatmap: Beatmap,
    pub metadata: BeatmapCardInfoMetadata,
    //beatmap_hash: md5::Digest,
}

pub struct CurrentBackground {
    pub texture: Texture,
    pub image_hash: md5::Digest,
}

pub struct CurrentAudio {
    pub audio_hash: md5::Digest,
}

pub struct SongSelectScreen<'sss> {
    db: Arc<OsuDatabase>,
    graphics: Arc<Graphics<'sss>>,

    // Min & Max row that we currently need to draw
    min: usize,
    max: usize,

    // Currently selected row
    current: usize,
    
    // Just to keep state
    need_scroll_to: Option<usize>,

    song_select_tx: Sender<SongSelectionEvents>,

    quad_renderer: QuadRenderer<'sss>,
    quad_test_buffer: wgpu::Buffer,
    quad_test_instance_data: Vec<QuadInstance>,

    current_beatmap: Option<CurrentBeatmap>,
    current_background_image: Option<CurrentBackground>,
}

impl<'sss> SongSelectScreen<'sss> {
    pub fn new(
        db: Arc<OsuDatabase>,
        graphics: Arc<Graphics<'sss>>, 
        song_select_tx: Sender<SongSelectionEvents>,
    ) -> Self {
        let quad_renderer = QuadRenderer::new(graphics.clone(), false);

        let quad_test_buffer = quad_renderer.create_instance_buffer();
        let quad_test_instance_data = Vec::new();

        Self {
            db,
            graphics,
            min: 0,
            max: 0,
            current: 0,
            need_scroll_to: None,
            song_select_tx,
            quad_renderer,
            quad_test_buffer,
            quad_test_instance_data,
            current_beatmap: None,
            current_background_image: None,
        }
    }

    pub fn current_in_cache(&self) -> usize {
        self.current - self.min
    }

    pub fn set_scroll_to(&mut self, to: usize) {
        self.need_scroll_to = Some(to);
    }

    pub fn increment_beatmap(&mut self) {
        self.set_scroll_to(self.current + 1);
    }

    pub fn decrement_beatmap(&mut self) {
        self.set_scroll_to(self.current.saturating_sub(1));
    }

    pub fn set_background(&mut self, image: DynamicImage, md5: Digest) {
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

    pub fn set_current_beatmap(&mut self, beatmap: Option<CurrentBeatmap>) {
        self.current_beatmap = beatmap;
    }

    fn render_background(&self, view: &TextureView) {
        let _span = tracy_client::span!("osu_song_select_state::render_background");
        if let Some(current_background) = &self.current_background_image {
            self.quad_renderer.render_on_view_instanced(
                &view,
                &current_background.texture.bind_group,
                &self.quad_test_buffer,
                0..1
            );
        }
    }

    fn resize_background_vertex(&self, width: f32, height: f32) {
        let _span = tracy_client::span!("osu_song_select_state::resize_background_vertex");

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

    pub fn on_resize(&mut self, new_size: &PhysicalSize<u32>) {
        let _span = tracy_client::span!("osu_song_select_state::on_resize");
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
        
        // TODO move into quad_renderer itself since we are operating on
        // QuadInstance only
        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.quad_test_buffer,
            &self.quad_test_instance_data,
            QuadInstance
        );
    }

    pub fn render(
        &mut self, 
        ctx: &egui::Context, 
        view: &TextureView,
    ) {
        self.render_background(view);

        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            StripBuilder::new(ui)
                .size(Size::relative(0.6))
                .size(Size::relative(0.4))
                .horizontal(|mut strip| {
                    strip.strip(|builder| {
                        builder
                            .size(Size::relative(0.2))
                            .size(Size::relative(0.8))
                            .vertical(|mut strip| {
                                strip.cell(|ui| {
                                    self.render_beatmap_card_info(ui);
                                });

                                strip.strip(|builder| {
                                    builder
                                        .size(Size::relative(0.9))
                                        .size(Size::relative(0.1))
                                        .vertical(|mut strip| {
                                            strip.cell(|_ui| ());

                                            strip.cell(|ui| {
                                                egui::Frame::default()
                                                    .corner_radius(5.0)
                                                    .outer_margin(10.0)
                                                    .inner_margin(5.0)
                                                    .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 255))
                                                    .show(ui, |ui| {
                                                        ui.set_width(ui.available_rect_before_wrap().width());
                                                        ui.set_height(ui.available_rect_before_wrap().height());
                                                        self.render_beatmap_footer(ui);
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
                                let entry = self.db.get_beatmap_by_index(need_scroll_to);

                                if let Some(entry) = entry {
                                    let current_y = self.current as f32 * ROW_HEIGHT;

                                    let scroll_y = need_scroll_to as f32 * ROW_HEIGHT;

                                    let scroll_y = scroll_y - current_y;
                                    self.current = need_scroll_to;

                                    ui.scroll_with_delta(
                                        egui::Vec2::new(0.0, -1.0 * scroll_y)
                                    );

                                    self.song_select_tx.send(
                                        SongSelectionEvents::SelectBeatmap(entry.into())
                                    ).expect(
                                        "Failed to send SelectBeatmap event to the SongSelectState"
                                    );
                                }
                            }

                            let min_row = (rect.min.y / ROW_HEIGHT).floor() as usize;
                            let max_row = (rect.max.y / ROW_HEIGHT).floor() as usize;


                            let fill_top = (min_row - 0) as f32 * (ROW_HEIGHT);
                            egui::Frame::NONE
                                .show(ui, |ui| {
                                    ui.set_height(fill_top);
                                });

                            if max_row != self.max || min_row != self.min {
                                self.db.fetch_beatmaps_range(min_row, max_row);
                            }

                            let current = min_row;

                            let lock = self.db.cache.lock().unwrap();
                            
                            for (i, beatmap) in lock.iter().enumerate() {
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
                                    if id == self.current {
                                        self.song_select_tx.send(
                                            SongSelectionEvents::StartBeatmap(beatmap.clone())
                                        ).expect("Failed to send StartBeatmap event to the SongSelectState");
                                    }

                                    self.current = id;

                                    self.song_select_tx.send(
                                        SongSelectionEvents::SelectBeatmap(beatmap.clone())
                                    ).expect(
                                        "Failed to send SelectBeatmap event to the SongSelectState"
                                    );

                                    res.response.scroll_to_me(Some(Align::Center));
                                }

                                if sense.double_clicked() {
                                    self.current = id;
                                    self.song_select_tx.send(
                                        SongSelectionEvents::StartBeatmap(beatmap.clone())
                                    ).expect(
                                        "Failed to send StartBeatmap event to the SongSelectState"
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
    }

    fn render_beatmap_card_info(&mut self, ui: &mut egui::Ui) {
        let _span = tracy_client::span!("osu_song_select_state::render_beatmap_card_info");
        egui::Frame::default()
            .corner_radius(5.0)
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

    fn render_beatmap_footer(&mut self, ui: &mut egui::Ui) {
        let _span = tracy_client::span!("osu_song_select_state::render_beatmap_footer");
        ui.with_layout(egui::Layout::centered_and_justified(Direction::LeftToRight), |ui| {
            let text = format!("Beatmaps: {}", self.db.beatmaps_amount());
            ui.add(Label::new(RichText::new(text).heading())
                .selectable(false)
            );

            egui::Frame::NONE
                .show(ui, |ui| {
                    ui.set_min_width(50.0);
                    ui.set_max_width(50.0);
                    ui.set_width(50.0);

                    if ui.button("⚙").clicked() {
                        let _ = self.song_select_tx.send(SongSelectionEvents::ToggleSettings);
                    };
                });
        });
    }
}
