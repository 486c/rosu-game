use cgmath::{InnerSpace, Vector2};
use rosu::camera::Camera;
use winit::dpi::PhysicalPosition;

use crate::{
    replay_log::ReplayFrame,
    tools::{SelectionChange, ToolOutcome},
};

/// Click to select the nearest visible frame point, hold to drag it.
///
/// Hit-testing happens in **screen space** (comparing pixel distances), not
/// world space: the old constant of 2.0 world units on a 512x400 playfield
/// made points practically impossible to click.
pub struct SelectTool {
    /// Absolute index into `ReplayLog::frames`. Slice-relative indices went
    /// stale as soon as the visible window slid (any playback tick), which
    /// made the tool drag the wrong frame.
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

    /// Frame points are world-space quads; convert their centers to screen
    /// pixels and pick the nearest one within the hit radius.
    fn hit_test(&self, frames: &[ReplayFrame], camera: &Camera) -> Option<usize> {
        // A bit beyond the drawn point radius (5 world units * scale), so
        // points are comfortably clickable.
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
        // If the press started on a point, grab it immediately (no need to
        // click first). The old code additionally required the cursor to
        // still be within a tiny radius of the selected point, which
        // silently dropped most drags.
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

        // The selection may be outside the visible window (time moved on);
        // only drag when the frame is actually in it.
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
