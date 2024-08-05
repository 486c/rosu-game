use std::sync::mpsc::Sender;

use egui::{color_picker::show_color, Button, Context, Label, Ui};
use egui_file::FileDialog;

use crate::{osu_state::OsuStateEvent, skin_manager::SkinManager};

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
    ) {
        egui::Window::new("Settings").show(ctx, |ui| {
            self.skin_settings_ui(ui, skin)
        });


        if let Some(dialog) = &mut self.file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(dir) = dialog.path() {
                    let _ = self.sender.send(OsuStateEvent::ChangeSkin(dir.to_path_buf()));
                }
            }
        }
    }

    pub fn ui(&self, ui: &mut Ui) {
        ui.add(Label::new("Settings Bebra"));
    }

    pub fn skin_settings_ui(
        &mut self, 
        ui: &mut Ui,
        skin: &SkinManager,
    ) {
        ui.heading("Skin");
        if ui.add(Button::new("Open skin")).clicked() {
            let mut dialog = FileDialog::select_folder(None);
            dialog.open();
            self.file_dialog = Some(dialog);
        }

        ui.label(format!("Name: {}", skin.ini.general.name));
        ui.label(format!("Author: {}", skin.ini.general.author));

        ui.collapsing("Skin colours", |ui| {
            for (i, c) in skin.ini.colours.combo_colors.iter().enumerate() {
                ui.group(|ui| {
                    ui.label(format!("Colour {}: ", i));
                    show_color(ui, c.to_egui_color(), egui::Vec2::new(10.0, 10.0));
                });
            }
        });

    }


}
