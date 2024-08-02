use std::sync::mpsc::Sender;

use egui::{Button, Context, Label, Ui};
use egui_file::FileDialog;

use crate::osu_state::OsuStateEvent;

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
    pub fn window(&mut self, ctx: &Context) {
        egui::Window::new("Settings").show(ctx, |ui| {
            self.skin_settings_ui(ui)
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

    pub fn skin_settings_ui(&mut self, ui: &mut Ui) {
        ui.heading("Skin");
        if ui.add(Button::new("Open skin")).clicked() {
            let mut dialog = FileDialog::select_folder(None);
            dialog.open();
            self.file_dialog = Some(dialog);
        }

    }


}
