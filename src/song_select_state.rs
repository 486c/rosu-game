use std::{sync::mpsc::{Receiver, Sender}, time::Duration};

use egui::{scroll_area::ScrollBarVisibility, Align, Color32, Label, Margin, RichText, Stroke};
use egui_extras::{Size, StripBuilder};
use rosu_map::Beatmap;

use crate::osu_db::{BeatmapEntry, OsuDatabase};


const CARD_INNER_MARGIN: Margin = Margin {
    left: 5.0,
    right: 0.0,
    top: 8.0,
    bottom: 0.0,
};


enum SongSelectionEvents {
    SelectBeatmap(BeatmapEntry),
    LoadedBeatmap(Beatmap),
}

pub struct SongSelectionState {
    db: OsuDatabase,

    // Min & Max row that we currently need to draw
    min: usize,
    max: usize,

    // Current selected row
    current: usize,

    current_beatmap: Option<Beatmap>,

    inner_tx: Sender<SongSelectionEvents>,
    inner_rx: Receiver<SongSelectionEvents>,
}

impl SongSelectionState {
    pub fn new() -> Self {
        let (inner_tx, inner_rx) = std::sync::mpsc::channel();

        Self {
            db: OsuDatabase::new().unwrap(), // TODO: REMOVE UNRAP
            min: 0,
            max: 0,
            current: 0,
            inner_tx,
            inner_rx,
            current_beatmap: None,
        }
    }
    
    // Spawns a thread to parse a beatmap
    fn open_beatmap(&self, beatmap: &BeatmapEntry) {
        let tx = self.inner_tx.clone();
        let path = beatmap.path.clone();

        std::thread::spawn(move || {
            let parsed_beatmap = Beatmap::from_path(&path).unwrap();
            let bg_filename = parsed_beatmap.background_file.clone();
            let _bg_path = path.parent()
                .unwrap()
                .join(&bg_filename);
        
            /*
            let img = image::open(bg_path).unwrap();
            let img = img.blur(5.0);
            let rgba_image = img.to_rgba8();

            let image = egui::ColorImage::from_rgba_unmultiplied(
                [img.width() as usize, img.height() as usize],
                &rgba_image,
            );
            */

            tx.send(SongSelectionEvents::LoadedBeatmap(
                parsed_beatmap
            ))
        });
    }

    pub fn update(&mut self) {
        match self.inner_rx.try_recv() {
            Ok(event) => {
                match event {
                    SongSelectionEvents::SelectBeatmap(entry) => {
                        self.open_beatmap(&entry);
                    },
                    SongSelectionEvents::LoadedBeatmap(b) => {
                        self.current_beatmap = Some(b)
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
                    ui.add(Label::new(RichText::new(format!("{} - {} [{}]", &b.artist, &b.title, &b.version)).heading()).selectable(false));
                    ui.add(Label::new(format!("Mapped by {}", &b.creator)).selectable(false));

                    let last_hitobject_time = if let Some(obj) = b.hit_objects.last_mut() {
                        obj.end_time() as u64
                    } else {
                        0
                    };

                    let length = 
                        Duration::from_millis(last_hitobject_time);

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

                    let text = format!(
                        "Length: {} BPM: {:.0}-{:.0} Objects: {}",
                        length_str, 
                        bpm_min, bpm_max,
                        b.hit_objects.len() 
                    );
                    ui.add(Label::new(RichText::new(&text).strong()).selectable(false));

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

                    ui.add(Label::new(format!("Circles: {} Slider: {} Spinners: {}", circles, sliders, spinners)).selectable(false));
                    ui.add(Label::new(format!(
                                "CS:{:.2} AR:{:.2} OD:{:.2} HP:{:.2} Stars:TODO", 
                                b.circle_size, b.approach_rate, b.overall_difficulty, b.hp_drain_rate
                    )).selectable(false));
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.spinner();
                    });
                }
            });

    }

    pub fn render(&mut self, input: egui::RawInput, ctx: &egui::Context) -> egui::FullOutput {
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
                        let row_height = 72.0;

                        egui::ScrollArea::vertical()
                        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                        .show_viewport(ui, |ui, rect| {
                            let min_row = (rect.min.y / row_height).floor() as usize;
                            let max_row = (rect.max.y / row_height).floor() as usize;
                            let total_height = 64.0 * self.db.beatmaps_amount() as f32;

                            ui.set_height(total_height);

                            let fill_top = (min_row - 0) as f32 * (row_height);
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
                                    .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 160))
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
                                        self.inner_tx.send(SongSelectionEvents::SelectBeatmap(beatmap.clone())); // TODO handle this shit
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
