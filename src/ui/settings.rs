use std::sync::mpsc::Sender;

use egui::{color_picker::show_color, Button, Context, Label, Slider, Ui};
use egui_file::FileDialog;

use crate::{config::Config, osu_state::OsuStateEvent, skin_manager::SkinManager};

pub struct SettingsView {
    sender: Sender<OsuStateEvent>,
    file_dialog: Option<FileDialog>,
}

impl SettingsView {
    pub fn new(sender: Sender<OsuStateEvent>) -> Self {
        Self {
            sender,
            file_dialog: None,
        }
    }

    pub fn window(
        &mut self, 
        ctx: &Context,
        skin: &SkinManager,
        config: &mut Config,
    ) {
        egui::Window::new("Settings")
            .max_width(150.0)
            .default_width(150.0)
            .min_width(150.0)
            .resizable(true)
            .show(ctx, |ui| {
                self.ui(ui, config);
                ui.separator();
                self.skin_settings_ui(ui, skin);
                ui.separator();
            });


        if let Some(dialog) = &mut self.file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(dir) = dialog.path() {
                    let _ = self.sender.send(OsuStateEvent::ChangeSkin(dir.to_path_buf()));
                }
            }
        }
    }

    pub fn ui(&self, ui: &mut Ui, config: &mut Config) {
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
    }

    pub fn skin_settings_ui(
        &mut self, 
        ui: &mut Ui,
        skin: &SkinManager,
    ) {

        ui.set_min_width(250.0);
        ui.heading("Skin");
        if ui.add(Button::new("Open skin")).clicked() {
            let mut dialog = FileDialog::select_folder(None);
            dialog.open();
            self.file_dialog = Some(dialog);
        }

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

    }


}
