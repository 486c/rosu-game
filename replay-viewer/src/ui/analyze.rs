use egui::Ui;

use crate::{replay_log::ReplayLog, ui::holdtimes::HoldTimesUi};

pub struct AnalyzeUi {
    hold_times: HoldTimesUi
}

impl AnalyzeUi {
    pub fn new() -> Self {
        Self { 
            hold_times: HoldTimesUi::new()
        }
    }

    pub fn update_replay(&mut self, replay: &ReplayLog) {
        self.hold_times.update_replay(replay);
    }

    pub fn render_ui(&mut self, ui: &mut Ui) {
        if ui.button("Open replay Hold Times").clicked() {
            self.hold_times.toggle();
        };


        self.hold_times.render_ui(ui);
    }
}
