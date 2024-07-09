use rosu_map::util::Pos;

use super::CIRCLE_FADEOUT_TIME;

pub struct Circle {
    pub start_time: f64,
    pub pos: Pos,
}

impl Circle {
    pub fn is_visible(&self, time: f64, preempt: f32) -> bool {
        time > self.start_time - preempt as f64 && time < self.start_time + CIRCLE_FADEOUT_TIME
    }
}
