use std::sync::mpsc::Sender;

use cgmath::Vector2;
use rosu::camera::Camera;
use winit::dpi::PhysicalPosition;

use crate::{replay_log::ReplayFrame, state::ReplayViewerEvents};

const CLICKABLE_SELECTION_RADIUS: f64 = 4.0;

pub struct SelectTool {
    state_tx: Sender<ReplayViewerEvents>,
    selected: Option<usize>,
    position: Vector2<f32>,


}

impl SelectTool {
    pub fn new(state_tx: Sender<ReplayViewerEvents>) -> Self {
        Self {
            state_tx,
            selected: None,
            position: (0.0, 0.0).into(),
        }
    }

    pub fn on_mouse_moved(&mut self, position: &PhysicalPosition<f64>) {
        self.position = Vector2::new(position.x as f32, position.y as f32)
    }

    pub fn on_click(
        &mut self,
        visible_frames: &mut [ReplayFrame],
        camera: &Camera,
    ) {
        let s = camera.screen_to_world(self.position);
        
        for (i, frame) in visible_frames.iter().enumerate() {
            let (px, py) = (s.x as f64, s.y as f64);
            let (cx, cy) = (frame.pos.0, frame.pos.1);

            let distance = ((px - cx).powf(2.0) + (py - cy).powf(2.0)).sqrt();

            if !(distance <= (CLICKABLE_SELECTION_RADIUS / 2.0) as f64) {
                continue;
            };

            let selected_index = i;

            let _ = self.state_tx.send(ReplayViewerEvents::SetSelectedFrame(selected_index));
            let _ = self.state_tx.send(ReplayViewerEvents::UpdateAnalyzeCursorBuffers);
            self.selected = Some(selected_index);
            return;
        }
        
        if let Some(selected_index) = self.selected.take() {
            let _ = self.state_tx.send(ReplayViewerEvents::SetUnselectedFrame(selected_index));
            let _ = self.state_tx.send(ReplayViewerEvents::UpdateAnalyzeCursorBuffers);
        };
        //self.prev_selected_frame = self.selected_frame.take();
        //self.update_analyze_cursor_buffers();
    }

    pub fn on_left_mouse_holding(
        &mut self, 
        visible_frames: &mut [ReplayFrame],
        camera: &Camera,
    ) {
        if let Some(selected_frame) = self.selected {
            let world_pos = camera
                .screen_to_world(Vector2::new(self.position.x as f32, self.position.y as f32));

            let (px, py) = (world_pos.x as f64, world_pos.y as f64);
            let (cx, cy) = (
                visible_frames[selected_frame].pos.0,
                visible_frames[selected_frame].pos.1,
            );

            let distance = ((px - cx).powf(2.0) + (py - cy).powf(2.0)).sqrt();

            let is_in_radius = distance <= (CLICKABLE_SELECTION_RADIUS * 2.0 / 2.0) as f64;

            if is_in_radius {
                visible_frames[selected_frame].pos =
                    (world_pos.x as f64, world_pos.y as f64);

                let _ = self.state_tx.send(ReplayViewerEvents::SyncCursor);
                let _ = self.state_tx.send(ReplayViewerEvents::UpdateAnalyzeCursorBuffers);
            }
        }

    }
}
