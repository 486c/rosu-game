use egui::Ui;
use rosu::camera::Camera;
use winit::dpi::PhysicalPosition;

use crate::{
    replay_log::ReplayFrame,
    tools::{brush::BrushTool, select::SelectTool, smudge::SmudgeTool},
};

pub mod brush;
pub mod select;
pub mod smudge;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Tools {
    Select,
    Brush,
    Smudge,
    None
}

/// Selection change a tool reports back to the state; the index is
/// absolute (into `ReplayLog::frames`).
#[derive(Debug, Clone, Copy)]
pub enum SelectionChange {
    Selected(usize),
    Deselected,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ToolOutcome {
    /// Some frames were edited and need a cursor data re-upload.
    pub changed: bool,
    pub selection: Option<SelectionChange>,
}

impl ToolOutcome {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn changed() -> Self {
        Self {
            changed: true,
            selection: None,
        }
    }
}

pub struct ToolBox {
    current: Tools,
    brush: BrushTool,
    select: SelectTool,
    smudge: SmudgeTool,
}

impl ToolBox {
    pub fn new() -> Self {
        Self {
            current: Tools::None,
            brush: BrushTool::new(),
            select: SelectTool::new(),
            smudge: SmudgeTool::new(),
        }
    }

    pub fn current(&self) -> Tools {
        self.current
    }

    pub fn screen_radius(&self) -> Option<f32> {
        match self.current {
            Tools::Brush => Some(self.brush.radius),
            Tools::Smudge => Some(self.smudge.radius),
            _ => None,
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

    /// `frames_offset` is the absolute index of `frames[0]` in
    /// `ReplayLog::frames`.
    pub fn on_left_mouse_holding(
        &mut self,
        frames: &mut [ReplayFrame],
        frames_offset: usize,
        camera: &Camera,
    ) -> ToolOutcome {
        match self.current {
            Tools::Select => self.select.on_left_mouse_holding(frames, frames_offset, camera),
            Tools::Brush => self.brush.on_left_mouse_holding(frames, camera),
            Tools::Smudge => self.smudge.on_left_mouse_holding(frames, camera),
            Tools::None => ToolOutcome::none(),
        }
    }

    pub fn on_click(
        &mut self,
        frames: &mut [ReplayFrame],
        frames_offset: usize,
        camera: &Camera,
    ) -> ToolOutcome {
        match self.current {
            Tools::Select => self.select.on_click(frames, frames_offset, camera),
            Tools::Brush | Tools::Smudge | Tools::None => ToolOutcome::none(),
        }
    }

    pub fn render_ui(&mut self, ui: &mut Ui) {
        ui.selectable_value(&mut self.current, Tools::None, "None");
        ui.selectable_value(&mut self.current, Tools::Select, "Select");
        ui.selectable_value(&mut self.current, Tools::Brush, "Brush");
        ui.selectable_value(&mut self.current, Tools::Smudge, "Smudge");

        ui.label("Tools edit frames in the visible window; increase \"Show frames\" to edit more.");

        match self.current {
            Tools::Brush => {
                ui.label(format!("Radius: {:.0} px", self.brush.radius));
                ui.add(
                    egui::Slider::new(&mut self.brush.radius, 5.0..=300.0).text("Brush radius"),
                );
                ui.add(
                    egui::Slider::new(&mut self.brush.strength, 0.1..=10.0).text("Brush strength"),
                );
            }
            Tools::Smudge => {
                ui.label(format!("Radius: {:.0} px", self.smudge.radius));
                ui.add(
                    egui::Slider::new(&mut self.smudge.radius, 5.0..=300.0).text("Smudge radius"),
                );
                ui.add(
                    egui::Slider::new(&mut self.smudge.strength, 0.1..=10.0)
                        .text("Smudge strength"),
                );
            }
            Tools::Select => {
                ui.label("Click a point to select it, then hold to drag it. Click empty space to deselect.");
            }
            Tools::None => {}
        }
    }
}
