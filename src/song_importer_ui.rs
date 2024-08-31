use std::{path::PathBuf, sync::mpsc::Sender};

use egui::Button;
use egui_file::FileDialog;

use crate::song_select_state::{SongSelectionEvents, SongsImportJob};

pub struct SongImporter {
    file_dialog: Option<FileDialog>,
    is_opened: bool,

    current_jobs: Vec<(PathBuf, Option<oneshot::Sender<()>>)>,

    song_select_tx: Sender<SongSelectionEvents>,
}

impl SongImporter {
    pub fn new(tx: Sender<SongSelectionEvents>) -> Self {
        Self { 
            is_opened: false,
            file_dialog: None,
            song_select_tx: tx,
            current_jobs: Vec::new(),
        }
    }

    pub fn toggle(&mut self) {
        self.is_opened = !self.is_opened;
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        if !self.is_opened {
            return
        };

        let screen_rect = ctx.screen_rect();

        egui::Window::new("Songs Importer")
            .resizable(false)
            .fixed_size(egui::Vec2 {x: 250.0, y: 200.0})
            .default_pos((screen_rect.width() / 2.0, screen_rect.height() / 2.0))
            .open(&mut self.is_opened)
            .show(ctx, |ui| {
                let button = Button::new("Select directory to import")
                    .min_size((ui.available_rect_before_wrap().width(), 20.0).into());

                if ui.add(button).clicked() {
                    let mut dialog = FileDialog::select_folder(None)
                        .title("Select a Songs folder to import into rosu");

                    dialog.open();

                    self.file_dialog = Some(dialog);
                }

                ui.separator();

                egui::ScrollArea::vertical()
                    .show(ui, |ui| {
                        self.current_jobs.retain_mut(|(path, tx)| {
                            let mut not_retain = true;
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(format!("{}", &path.display()));
                                if ui.button("Stop").clicked() {
                                    let _ = tx.take().unwrap().send(());
                                    not_retain = false;
                                }
                            });
                            ui.end_row();

                            not_retain
                        });
                    })
            });

        if let Some(dialog) = &mut self.file_dialog {
            if dialog.show(ctx).selected() {
                let path = dialog.path().unwrap();
                let (tx, rx) = oneshot::channel();

                let job = SongsImportJob {
                    path: path.to_path_buf(),
                    stop_rx: rx,
                };

                self.current_jobs.push((path.to_path_buf(), Some(tx)));

                let _ = self.song_select_tx.send(SongSelectionEvents::ImportSongsDirectory(job));
            }
        };
    }
}
