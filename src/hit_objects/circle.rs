use cgmath::Vector2;
use rosu_map::util::Pos;

use super::{hit_window::HitWindow, Hit, CIRCLE_FADEOUT_TIME, JUDGMENTS_FADEOUT_TIME};

pub struct CircleHitResult {
    pub at: f64,
    pub pos: Vector2<f64>,
    pub result: Hit
}

pub struct Circle {
    pub start_time: f64,
    pub pos: Pos,

    pub hit_result: Option<CircleHitResult>,
}

impl Circle {
    pub fn is_visible(&self, time: f64, preempt: f32, hit_window: &HitWindow) -> bool {
        time > self.start_time - preempt as f64 && time < self.start_time + CIRCLE_FADEOUT_TIME + hit_window.x50
    }


    pub fn is_judgements_visible(&self, time: f64, preempt: f32) -> bool {
        time > self.start_time - preempt as f64 && time < self.start_time + (CIRCLE_FADEOUT_TIME * 2.0) + (JUDGMENTS_FADEOUT_TIME * 2.0)
    }
    
    /// Checks if circle is hittable as well as calculating hit result
    pub fn is_hittable(
        &self, 
        hit_time: f64, 
        hit_window: &HitWindow,
        pos: Vector2<f64>,
        diameter: f32,
    ) -> Option<Hit> {
        let (cx, cy) = (self.pos.x as f64, self.pos.y as f64);
        let (px, py) = (pos.x, pos.y);

        let distance = ((px - cx).powf(2.0) + (py - cy).powf(2.0)).sqrt();

        if !(distance <= (diameter / 2.0) as f64) {
            return None
        }

        let hit_error = (self.start_time - hit_time).abs();

        if hit_error < hit_window.x300.round() {
            return Some(Hit::X300);
        }

        if hit_error < hit_window.x100.round() {
            return Some(Hit::X100);
        }

        if hit_error < hit_window.x50.round() {
            return Some(Hit::X50);
        }

        None
    }
}
