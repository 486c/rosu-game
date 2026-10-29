use cgmath::{InnerSpace, Vector2};
use rosu::camera::Camera;
use winit::dpi::PhysicalPosition;

use crate::{replay_log::ReplayFrame, tools::ToolOutcome};

/// Smudge: drags nearby points along with the cursor movement.
pub struct SmudgeTool {
    position: Vector2<f32>,
    prev_position: Vector2<f32>,

    pub radius: f32,
    pub strength: f32,
}

impl SmudgeTool {
    pub fn new() -> Self {
        Self {
            position: (0.0, 0.0).into(),
            prev_position: (0.0, 0.0).into(),
            radius: 40.0,
            strength: 1.0,
        }
    }

    pub fn on_mouse_moved(&mut self, position: &PhysicalPosition<f64>) {
        let new_pos = Vector2::new(position.x as f32, position.y as f32);
        self.prev_position = self.position;
        self.position = new_pos;
    }

    pub fn on_left_mouse_holding(
        &mut self,
        frames: &mut [ReplayFrame],
        camera: &Camera,
    ) -> ToolOutcome {
        let brush_delta = self.position - self.prev_position;
        let brush_speed = brush_delta.magnitude();

        if brush_speed < 0.001 {
            return ToolOutcome::none();
        }

        let scale = camera.scale.max(0.001);

        // Screen px -> world units.
        let world_delta = brush_delta / scale;
        let smudge_pos = camera.screen_to_world(self.position);
        let radius = (self.radius / scale).max(1.0);

        let direction = world_delta.normalize();
        let move_amount = self.strength * world_delta.magnitude();

        let mut changed = false;

        for frame in frames {
            let mut frame_pos = Vector2::new(frame.pos.0 as f32, frame.pos.1 as f32);

            let to_point = frame_pos - smudge_pos;
            let distance = to_point.magnitude();

            if distance < radius {
                let influence = self.calculate_influence(distance, radius);
                let displacement = direction * (move_amount * influence);

                frame_pos += displacement;

                frame.pos.0 = frame_pos.x as f64;
                frame.pos.1 = frame_pos.y as f64;
                changed = true;
            }
        }

        if changed {
            ToolOutcome::changed()
        } else {
            ToolOutcome::none()
        }
    }

    fn calculate_influence(&self, distance: f32, radius: f32) -> f32 {
        let t = (distance / radius).clamp(0.0, 1.0);

        // Smooth falloff that peaks at the center and falls off to zero at edge
        // Using (1 - t^falloff) for natural smudge feel
        (1.0 - t * t).max(0.0)
    }
}
