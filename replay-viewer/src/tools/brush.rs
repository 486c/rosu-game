use cgmath::{InnerSpace, Vector2};
use rosu::camera::Camera;
use winit::dpi::PhysicalPosition;

use crate::{replay_log::ReplayFrame, tools::ToolOutcome};

/// Basic outward pushing brush.
pub struct BrushTool {
    position: Vector2<f32>,
    pub radius: f32,
    pub strength: f32,
}

impl BrushTool {
    pub fn new() -> Self {
        Self {
            position: (0.0, 0.0).into(),
            radius: 40.0,
            strength: 1.5,
        }
    }

    pub fn on_mouse_moved(&mut self, position: &PhysicalPosition<f64>) {
        self.position = Vector2::new(position.x as f32, position.y as f32);
    }

    pub fn on_left_mouse_holding(
        &mut self,
        frames: &mut [ReplayFrame],
        camera: &Camera,
    ) -> ToolOutcome {
        let brush_world_pos = camera.screen_to_world(self.position);

        // Screen px -> world units.
        let radius = (self.radius / camera.scale.max(0.001)).max(1.0);

        let mut changed = false;

        for frame in frames {
            let frame_pos = Vector2::new(frame.pos.0 as f32, frame.pos.1 as f32);

            let to_point = frame_pos - brush_world_pos;
            let distance = to_point.magnitude();

            if distance < radius && distance > f32::EPSILON {
                let direction = to_point.normalize();
                let influence = self.calculate_influence(distance, radius);

                let push_vector = direction * (self.strength * influence);

                frame.pos.0 += push_vector.x as f64;
                frame.pos.1 += push_vector.y as f64;
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
        1.0 - (3.0 * t * t - 2.0 * t * t * t)
    }
}
