use cgmath::Vector2;
use rosu_map::util::Pos;

use crate::osu_input::OsuInput;

use super::{hit_window::HitWindow, Hit, CIRCLE_FADEOUT_TIME, JUDGMENTS_FADEOUT_TIME};

#[derive(Debug)]
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

    pub fn update(
        &mut self,
        input: &OsuInput,
        hit_window: &HitWindow,
        circle_diameter: f32,
    ) {
        if self.hit_result.is_some() {
            return;
        }

        if !input.keys.is_key_hit() {
            return;
        }

        let (cx, cy) = (self.pos.x as f64, self.pos.y as f64);
        let (px, py) = (input.pos.x, input.pos.y);

        let distance = ((px - cx).powf(2.0) + (py - cy).powf(2.0)).sqrt();

        if !(distance <= (circle_diameter / 2.0) as f64) {
            return;
        }

        let hit_error = (self.start_time - input.ts).abs();

        if hit_error < hit_window.x300.round() {
            self.hit_result = Some(CircleHitResult {
                at: input.ts,
                pos: input.pos,
                result: Hit::X300,
            });
            return;
        }

        if hit_error < hit_window.x100.round() {
            self.hit_result = Some(CircleHitResult {
                at: input.ts,
                pos: input.pos,
                result: Hit::X100,
            });
            return;
        }

        if hit_error < hit_window.x50.round() {
            self.hit_result = Some(CircleHitResult {
                at: input.ts,
                pos: input.pos,
                result: Hit::X50,
            });
            return;
        }
    }
}
