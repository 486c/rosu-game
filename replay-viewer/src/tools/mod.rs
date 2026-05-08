use std::sync::mpsc::Sender;

use egui::Ui;
use rosu::camera::Camera;
use winit::dpi::PhysicalPosition;

use crate::{replay_log::ReplayFrame, state::ReplayViewerEvents, tools::{brush::BrushTool, select::SelectTool, smudge::SmudgeTool}};

pub mod brush;
pub mod select;
pub mod smudge;

#[derive(Debug, Eq, PartialEq)]
pub enum Tools {
    Select,
    Brush,
    Smudge,
    None
}

pub struct ToolBox {
    state_tx: Sender<ReplayViewerEvents>,

    current: Tools,
    brush: BrushTool,
    select: SelectTool,
    smudge: SmudgeTool,
}

impl ToolBox {
    pub fn new(
        state_tx: Sender<ReplayViewerEvents>,
    ) -> Self {
        Self {
            current: Tools::None,
            brush: BrushTool::new(state_tx.clone()),
            select: SelectTool::new(state_tx.clone()),
            smudge: SmudgeTool::new(state_tx.clone()),
            state_tx,
        }
    }

    pub fn on_mouse_moved(&mut self, position: &PhysicalPosition<f64>) {
        match self.current {
            Tools::Select => self.select.on_mouse_moved(position),
            Tools::Brush => self.brush.on_mouse_moved(position),
            Tools::Smudge => self.smudge.on_mouse_moved(position),
            Tools::None => {},
        };
    }

    pub fn on_left_mouse_holding(
        &mut self, 
        visible_frames: &mut [ReplayFrame],
        camera: &Camera,
    ) {
        match self.current {
            Tools::Select => self.select.on_left_mouse_holding(visible_frames, camera),
            Tools::Brush => self.brush.on_left_mouse_holding(visible_frames, camera),
            Tools::Smudge => self.smudge.on_left_mouse_holding(visible_frames, camera),
            Tools::None => {},
        };
    }

    pub fn on_click(
        &mut self, 
        visible_frames: &mut [ReplayFrame],
        camera: &Camera,
    ) {
        match self.current {
            Tools::Select => self.select.on_click(visible_frames, camera),
            Tools::Brush => {}
            Tools::Smudge => {}
            Tools::None => {},
        };
    }

    pub fn render_ui(&mut self, ui: &mut Ui) {
        ui.selectable_value(&mut self.current, Tools::None, "None");
        ui.selectable_value(&mut self.current, Tools::Select, "Select");
        ui.selectable_value(&mut self.current, Tools::Brush, "Brush");
        ui.selectable_value(&mut self.current, Tools::Smudge, "Smudge");
    }
}
