use std::sync::mpsc::Sender;

use cgmath::{InnerSpace, Vector2};
use rosu::camera::Camera;
use winit::dpi::PhysicalPosition;

use crate::{replay_log::ReplayFrame, state::ReplayViewerEvents};

pub struct SmudgeTool {
    state_tx: Sender<ReplayViewerEvents>,
    position: Vector2<f32>,
    prev_position: Vector2<f32>,

    radius: f32,
    falloff: f32,
}

impl SmudgeTool {
    pub fn new(state_tx: Sender<ReplayViewerEvents>) -> Self {
        Self {
            state_tx,
            position: (0.0, 0.0).into(),
            radius: 20.0,
            falloff: 1.2,
            prev_position: (0.0, 0.0).into(), 
        }
    }


    pub fn on_mouse_moved(&mut self, position: &PhysicalPosition<f64>) {
        let new_pos = Vector2::new(position.x as f32, position.y as f32);
        self.prev_position = self.position;
        self.position = new_pos;
    }


    pub fn on_left_mouse_holding(
        &mut self, 
        visible_frames: &mut [ReplayFrame],
        camera: &Camera,
    ) {
        let brush_delta = self.position - self.prev_position;
        let brush_speed = brush_delta.magnitude();

        if brush_speed < 0.001 {
            return;
        };

        let brush_direction = brush_delta.normalize();

        for frame in visible_frames {
            let smudge_pos = camera.screen_to_world(
                self.position
            );

            let mut frame_pos = 
                Vector2::new(frame.pos.0 as f32, frame.pos.1 as f32);

            let to_point = frame_pos - smudge_pos;
            let distance = to_point.magnitude();

            if distance < self.radius {
                let influence = self.calculate_influence(distance);

                let move_amount = 1.2 * influence * brush_speed;
                let displacement = brush_direction * move_amount;
                frame_pos += displacement;

                frame.pos.0 = frame_pos.x as f64;
                frame.pos.1 = frame_pos.y as f64;
                let _ = self.state_tx.send(ReplayViewerEvents::SyncCursor);
            }
        }
    }

    fn calculate_influence(&self, distance: f32) -> f32 {
        let t = (distance / self.radius).min(1.0);
        
        // Smooth falloff that peaks at the center and falls off to zero at edge
        // Using (1 - t^falloff) for natural smudge feel
        (1.0 - t.powf(self.falloff)).max(0.0)
    }
}
