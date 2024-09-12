use cgmath::Vector2;
use rosu_map::util::Pos;

use crate::osu_state::HitWindow;

use super::{Hit, HitResult, CIRCLE_FADEOUT_TIME};

pub struct Circle {
    pub start_time: f64,
    pub pos: Pos,

    pub hit_result: Option<HitResult>,
}

impl Circle {
    pub fn is_visible(&self, time: f64, preempt: f32) -> bool {
        time > self.start_time - preempt as f64 && time < self.start_time + CIRCLE_FADEOUT_TIME
    }

    pub fn is_hittable(
        &self, 
        time: f64, 
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

        let x50_early = self.start_time - hit_window.x50;
        let x50_late = self.start_time + hit_window.x50;

        let x100_early = self.start_time - hit_window.x100;
        let x100_late = self.start_time + hit_window.x100;

        let x300_early = self.start_time - hit_window.x300;
        let x300_late = self.start_time + hit_window.x300;

        if (x300_early..x300_late).contains(&time) {
            return Some(Hit::X300);
        }

        if (x100_early..x100_late).contains(&time) {
            return Some(Hit::X100);
        }

        if (x50_early..x50_late).contains(&time) {
            return Some(Hit::X50);
        }

        None
    }
}
