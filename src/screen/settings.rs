use std::sync::{Arc, RwLock};

use egui::{color_picker::show_color, Slider, TextStyle, Ui};

use crate::{config::Config, skin_manager::SkinManager};

pub struct SettingsScreen {
    config: Arc<RwLock<Config>>,
    skin_manager: Arc<RwLock<SkinManager>>,
    is_open: bool
}

impl SettingsScreen {
    pub fn new(
        config: Arc<RwLock<Config>>,
        skin_manager: Arc<RwLock<SkinManager>>,
    ) -> Self {
        Self {
            is_open: false,
            config,
            skin_manager,
        }
    }

    pub fn toggle(&mut self) {
        self.is_open = if self.is_open { false } else { true}
    }
    
    pub fn close(&mut self) {
        self.is_open = false;
    }

    #[inline]
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        if !self.is_open {
            return
        }

        // TODO: calculate dynamicly instead of hardcoded value
        let width = 512.0; 

        // TODO: Animation doesn't work
        let _offset = ctx.animate_bool_with_time_and_easing(
            egui::Id::new("settings_expand_animation"),
            !self.is_open,
            0.125,
            egui::emath::easing::quadratic_out,
        ) * width;

        if width <= 0.0 {
            return;
        }

        egui::Window::new("Settings")
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .fixed_size([width, ctx.screen_rect().height()])
            .fixed_pos([0.0, 0.0])
            .frame(
                egui::Frame::NONE
                .fill(egui::Color32::from_rgba_unmultiplied(4, 4, 4, 253))
                //.outer_margin(egui::epaint::Marginf { left: -offset, ..Default::default() }),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.show_settings_ui(ui);
                        self.show_skin_settings_ui(ui);
                    });
            });
    }

    pub fn show_skin_settings_ui(&self, ui: &mut Ui) {
        let heading_font = egui::FontId::new(20.0, egui::FontFamily::Proportional);

        let skin = self.skin_manager.read().expect("failed to acquire read lock");

        ui.collapsing(egui::RichText::new("Skin").font(heading_font), |ui| {
            ui.label(format!("Name: {}", skin.ini.general.name));
            ui.label(format!("Author: {}", skin.ini.general.author));

            ui.collapsing("Skin colours", |ui| {
                ui.collapsing("Combo colours", |ui| {
                    for (i, c) in skin.ini.colours.combo_colors.iter().enumerate() {
                        ui.group(|ui| {
                            ui.label(format!("Colour {}: ", i));
                            show_color(ui, c.to_egui_color(), egui::Vec2::new(30.0, 10.0));
                        });
                    }
                });

                ui.collapsing("Slider colours", |ui| {
                    ui.label("Slider border color:");
                    show_color(
                        ui, 
                        skin.ini.colours.slider_border.to_egui_color(),
                        egui::Vec2::new(30.0, 10.0)
                    );

                    ui.label("Slider body color:");
                    show_color(
                        ui, 
                        skin.ini.colours.slider_body.to_egui_color(),
                        egui::Vec2::new(30.0, 10.0)
                    );
                });
            });

        });
    }
    
    /// Shows a settings UI that can be placed in any container
    pub fn show_settings_ui(&self, ui: &mut Ui) {
        let heading_font = egui::FontId::new(20.0, egui::FontFamily::Proportional);

        let mut config = self.config.write().expect("failed to acquire write lock");

        ui.collapsing(egui::RichText::new("Renderer").font(heading_font), |ui| {
            ui.heading("Slider");

            ui.checkbox(&mut config.store_slider_textures, "Store slider textures");

            ui.add(Slider::new(
                &mut config.slider.border_feather,
                0.0..=2.0
            ).text("Slider border feather"));

            ui.add(Slider::new(
                &mut config.slider.border_size_multiplier,
                0.0..=2.0
            ).text("Slider border size"));

            ui.add(Slider::new(
                &mut config.slider.body_color_saturation,
                0.0..=2.0
            ).text("Slider body color saturation"));

            ui.add(Slider::new(
                &mut config.slider.body_alpha_multiplier,
                0.0..=2.0
            ).text("Slider body alpha multiplier"));

            ui.heading("Judgements");

            ui.add(Slider::new(
                &mut config.judgements.fade_in_ms,
                0.0..=1000.0
            ).text("Fade-In milliseconds"));

            ui.add(Slider::new(
                &mut config.judgements.stay_on_screen_ms,
                0.0..=1000.0
            ).text("Stay on screen milliseconds"));

            ui.add(Slider::new(
                &mut config.judgements.fade_out_ms,
                0.0..=1000.0
            ).text("Fade-out milliseconds"));
        });
    }
}
