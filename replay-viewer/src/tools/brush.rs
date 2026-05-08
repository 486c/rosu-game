use std::sync::mpsc::Sender;

use cgmath::{InnerSpace, Vector2};
use rosu::camera::Camera;
use winit::dpi::PhysicalPosition;

use crate::{replay_log::ReplayFrame, state::ReplayViewerEvents};

/// Basic outward pushing brush
pub struct BrushTool {
    state_tx: Sender<ReplayViewerEvents>,

    active: bool,
    position: Vector2<f32>,
    radius: f32,
    strength: f32,
    falloff: f32,
}

impl BrushTool {
    pub fn new(state_tx: Sender<ReplayViewerEvents>) -> Self {
        Self {
            active: true,
            position: (0.0, 0.0).into(),
            radius: 20.0,
            strength: 1.2,
            falloff: 1.2,
            state_tx,
        }
    }

    pub fn on_mouse_moved(&mut self, position: &PhysicalPosition<f64>) {
        self.position = Vector2::new(position.x as f32, position.y as f32);
    }

    pub fn on_left_mouse_holding(
        &mut self, 
        visible_frames: &mut [ReplayFrame],
        camera: &Camera,
    ) {
        if !self.active {
            return;
        }
        
        let mut changed = false;
        for frame in visible_frames {
            let brush_world_pos = camera.screen_to_world(Vector2::new(
                self.position.x, self.position.y
            ));

            let frame_pos = Vector2::new(frame.pos.0 as f32, frame.pos.1 as f32);

            let to_point = frame_pos - brush_world_pos;
            let distance = to_point.magnitude();

            if distance < self.radius && distance > 0.0 {
                let direction = to_point.normalize();
                let influence = self.calculate_influence(distance);
                let push_vector = direction * 1.0 * influence;
                frame.pos.0 += push_vector.x as f64;
                frame.pos.1 += push_vector.y as f64;
                changed = true;
            }
        };

        if changed {
            let _ = self.state_tx.send(ReplayViewerEvents::SyncCursor);
        }
    }

    fn calculate_influence(&self, distance: f32) -> f32 {
        let t = distance / self.radius;
        
        let t = t.min(1.0).max(0.0);
        1.0 - (3.0 * t * t - 2.0 * t * t * t)
    }
}
