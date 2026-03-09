use egui::Ui;

use crate::{replay_log::ReplayLog, ui::{frametimes::FrameTimesUi, holdtimes::HoldTimesUi}};

pub struct AnalyzeUi {
    hold_times: HoldTimesUi,
    frame_times: FrameTimesUi,
}

impl AnalyzeUi {
    pub fn new() -> Self {
        Self { 
            hold_times: HoldTimesUi::new(),
            frame_times: FrameTimesUi::new(),
        }
    }

    pub fn update_replay(&mut self, replay: &ReplayLog) {
        self.hold_times.update_replay(replay);
        self.frame_times.update_replay(replay);
    }

    pub fn render_ui(&mut self, ui: &mut Ui) {
        if ui.button("Open replay Hold Times").clicked() {
            self.hold_times.toggle();
        };

        if ui.button("Open replay Frame Times").clicked() {
            self.frame_times.toggle();
        };

        self.hold_times.render_ui(ui);
        self.frame_times.render_ui(ui);
    }
}
