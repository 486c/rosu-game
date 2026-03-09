use std::collections::HashMap;

use egui::Ui;
use egui_extras::RetainedImage;
use plotters::{chart::ChartBuilder, prelude::{BitMapBackend, IntoDrawingArea}, style::Color};

use crate::replay_log::ReplayLog;

pub struct FrameTimesUi {
    open: bool,
    bar_image: Option<RetainedImage>,
    map: HashMap<i32, i32>,
}

impl FrameTimesUi {
    pub fn new() -> Self {
        Self {
            open: false,
            bar_image: None,
            map: HashMap::new(),
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn update_replay(&mut self, replay: &ReplayLog) {
        let mut map: HashMap<i32, i32> = HashMap::new();

        replay.frames
            .iter()
            .for_each(|frame| {
                *map.entry(frame.delta).or_insert(0) += 1;
            });
        
        self.map = map;
        self.render_graph();
    }


    fn render_graph(
        &mut self,
    ) {
        let x_max = *self.map.iter()
            .map(|(k, _)| k)
            .max()
            .unwrap_or(&10);

        let y_max = *self.map.iter()
            .map(|(_, v)| v)
            .max()
            .unwrap_or(&10);

        let mut buffer = vec![0u8; 800 * 600 * 3];

        {
            let backend = BitMapBackend::with_buffer(&mut buffer, (800, 600));
            let root = backend.into_drawing_area();
            root.fill(&plotters::style::WHITE).unwrap();

            let mut chart = ChartBuilder::on(&root)
                .caption("Frame Times", ("FreeSans", 20))
                .margin(10)
                .x_label_area_size(40)
                .y_label_area_size(60)
                .build_cartesian_2d(
                    0..x_max + 1,
                    0..y_max + 1 
                )
                .unwrap();

            chart.configure_mesh()
                .x_labels(self.map.len())
                .y_labels(self.map.len())
                .x_label_style(("FreeSans", 15, &plotters::style::BLACK))
                .y_label_style(("FreeSans", 15, &plotters::style::BLACK))
                .draw()
                .unwrap();

            chart.draw_series(
                self.map.iter().map(|(x, y)| {
                    plotters::prelude::Rectangle::new(
                        [
                            (*x - 4/2, 0),
                            (*x + 4/2, *y),
                        ],
                        plotters::style::BLUE.filled(),
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

    pub fn render_ui(&mut self, ui: &mut Ui) {
        if !self.open {
            return;
        };

        let ctx = ui.ctx();
        egui::Window::new("Frame Times").open(&mut self.open)
            .show(ctx, |ui| {
                if let Some(retained_image) = &self.bar_image {
                    retained_image.show(ui);
                }
            });
    }
}
