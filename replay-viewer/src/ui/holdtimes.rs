use std::collections::HashMap;
use egui_extras::RetainedImage;

use egui::Ui;
use plotters::{chart::ChartBuilder, prelude::{BitMapBackend, IntoDrawingArea}, style::Color};

use crate::replay_log::ReplayLog;

pub struct HoldTimesUi {
    open: bool,
    k1_map: HashMap<i32, i32>,
    k2_map: HashMap<i32, i32>,
    bar_image: Option<RetainedImage>,
}

impl HoldTimesUi {
    pub fn new() -> Self {
        Self {
            open: false,
            bar_image: None,
            k1_map: HashMap::new(),
            k2_map: HashMap::new(),
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    fn calculate_hold_times(
        replay: &ReplayLog
    ) -> (HashMap<i32, i32>, HashMap<i32, i32>) {
        let mut tracking_frame: Option<f64> = None;
        let mut k1: HashMap<i32, i32> = HashMap::new();
        let mut k2: HashMap<i32, i32> = HashMap::new();

        // K1
        for frame in replay.frames.iter() {
            // Pressed / Continius press
            if frame.keys.k1 && frame.keys.m1 {
                if let Some(_) = tracking_frame {
                    continue;
                } else {
                    tracking_frame = Some(frame.ts);
                    continue
                }
            };
            
            // Released
            if !frame.keys.k1 && !frame.keys.m1 {
                if let Some(start_at) = tracking_frame {
                    let duration = (frame.ts - start_at) as i32;
                    tracking_frame = None;

                    if duration >= 100 {
                        continue;
                    };

                    *k1.entry(duration).or_insert(0) += 1;
                }
            };
        };

        // k2
        for frame in replay.frames.iter() {
            // Pressed / Continius press
            if frame.keys.k2 && frame.keys.m2 {
                if let Some(_) = tracking_frame {
                    continue;
                } else {
                    tracking_frame = Some(frame.ts);
                    continue
                }
            };
            
            // Released
            if !frame.keys.k2 && !frame.keys.m2 {
                if let Some(start_at) = tracking_frame {
                    let duration = (frame.ts - start_at) as i32;
                    tracking_frame = None;

                    if duration >= 100 {
                        continue;
                    };

                    *k2.entry(duration).or_insert(0) += 1;
                }
            };
        };

        (k1, k2)

    }

    fn render_graph(
        &mut self,
    ) {
        let x_max = *self.k1_map.iter()
            .chain(self.k2_map.iter())
            .map(|(k, _)| k)
            .max()
            .unwrap_or(&10);

        let y_max = *self.k1_map.iter()
            .chain(self.k2_map.iter())
            .map(|(_, v)| v)
            .max()
            .unwrap_or(&10);

        let mut buffer = vec![0u8; 800 * 600 * 3];

        {
            let backend = BitMapBackend::with_buffer(&mut buffer, (800, 600));
            let root = backend.into_drawing_area();
            root.fill(&plotters::style::WHITE).unwrap();

            let mut chart = ChartBuilder::on(&root)
                .caption("Hold Times", ("FreeSans", 20))
                .margin(10)
                .x_label_area_size(40)
                .y_label_area_size(60)
                .build_cartesian_2d(
                    0..x_max + 1,
                    0..y_max + 1 
                )
                .unwrap();

            chart.configure_mesh()
                .x_labels(self.k1_map.len())
                .y_labels(self.k2_map.len())
                .x_label_style(("FreeSans", 15, &plotters::style::BLACK))
                .y_label_style(("FreeSans", 15, &plotters::style::BLACK))
                .draw()
                .unwrap();

            chart.draw_series(
                self.k1_map.iter().map(|(x, y)| {
                    plotters::prelude::Rectangle::new(
                        [
                            (*x - 4/2, 0),
                            (*x + 4/2, *y),
                        ],
                        plotters::style::BLUE.filled(),
                    )
                })
            ).unwrap();

            chart.draw_series(
                self.k2_map.iter().map(|(x, y)| {
                    plotters::prelude::Rectangle::new(
                        [
                            (*x - 4/2, 0),
                            (*x + 4/2, *y),
                        ],
                        plotters::style::RED.filled(),
                    )
                })
            ).unwrap();
            
            root.present().unwrap();
        }
        
        let image = image::RgbImage::from_raw(800, 600, buffer).unwrap();
        let mut png_buffer = Vec::new();
        image.write_to(
            &mut std::io::Cursor::new(&mut png_buffer), 
            image::ImageFormat::Png
        )
        .unwrap();

        let retained_image = RetainedImage::from_image_bytes(
            "Hold Times retained image",
            &png_buffer
        ).unwrap();

        self.bar_image = Some(retained_image);
    }

    pub fn update_replay(&mut self, replay: &ReplayLog) {
        let (k1, k2) = HoldTimesUi::calculate_hold_times(replay);
        self.k1_map = k1;
        self.k2_map = k2;

        self.render_graph();
    }

    pub fn render_ui(&mut self, ui: &mut Ui) {
        if !self.open {
            return;
        };
        
        let ctx = ui.ctx();
        egui::Window::new("Hold Times").open(&mut self.open)
            .show(ctx, |ui| {
                if let Some(retained_image) = &self.bar_image {
                    retained_image.show(ui);
                }
        });
    }
}
