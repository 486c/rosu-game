use cgmath::{InnerSpace, Vector2};
use rosu::camera::Camera;
use winit::dpi::PhysicalPosition;

use crate::{
    replay_log::ReplayFrame,
    tools::{SelectionChange, ToolOutcome},
};

/// Click to select the nearest visible frame point, hold to drag it.
pub struct SelectTool {
    selected: Option<usize>,
    position: Vector2<f32>,
}

impl SelectTool {
    pub fn new() -> Self {
        Self {
            selected: None,
            position: (0.0, 0.0).into(),
        }
    }

    pub fn on_mouse_moved(&mut self, position: &PhysicalPosition<f64>) {
        self.position = Vector2::new(position.x as f32, position.y as f32);
    }

    fn hit_test(&self, frames: &[ReplayFrame], camera: &Camera) -> Option<usize> {
        let radius = (camera.scale * 6.0 + 5.0).max(1.0);

        let mut best: Option<(usize, f32)> = None;

        for (i, frame) in frames.iter().enumerate() {
            let screen = Vector2::new(
                frame.pos.0 as f32 * camera.scale + camera.offsets.x,
                frame.pos.1 as f32 * camera.scale + camera.offsets.y,
            );

            let distance = (screen - self.position).magnitude();

            if distance <= radius && best.map_or(true, |(_, best_d)| distance < best_d) {
                best = Some((i, distance));
            }
        }

        best.map(|(i, _)| i)
    }

    pub fn on_click(
        &mut self,
        frames: &mut [ReplayFrame],
        frames_offset: usize,
        camera: &Camera,
    ) -> ToolOutcome {
        let _ = frames;

        if let Some(i) = self.hit_test(frames, camera) {
            let absolute = frames_offset + i;
            self.selected = Some(absolute);

            return ToolOutcome {
                changed: false,
                selection: Some(SelectionChange::Selected(absolute)),
            };
        }

        if self.selected.take().is_some() {
            return ToolOutcome {
                changed: false,
                selection: Some(SelectionChange::Deselected),
            };
        }

        ToolOutcome::none()
    }

    pub fn on_left_mouse_holding(
        &mut self,
        frames: &mut [ReplayFrame],
        frames_offset: usize,
        camera: &Camera,
    ) -> ToolOutcome {
        let mut newly_selected: Option<usize> = None;

        if self.selected.is_none() {
            if let Some(i) = self.hit_test(frames, camera) {
                let absolute = frames_offset + i;
                self.selected = Some(absolute);
                newly_selected = Some(absolute);
            }
        }

        let Some(selected) = self.selected else {
            return ToolOutcome::none();
        };

        let Some(relative) = selected.checked_sub(frames_offset) else {
            return ToolOutcome::none();
        };

        let Some(frame) = frames.get_mut(relative) else {
            return ToolOutcome::none();
        };

        let world_pos = camera.screen_to_world(self.position);
        frame.pos = (world_pos.x as f64, world_pos.y as f64);

        ToolOutcome {
            changed: true,
            selection: newly_selected.map(SelectionChange::Selected),
        }
    }
}
