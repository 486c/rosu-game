use std::sync::Arc;

use cgmath::Vector2;
use rosu_map::{section::hit_objects::Curve, util::Pos};

use crate::texture::Texture;

use super::{circle::CircleHitResult, hit_window::HitWindow, Hit, Rectangle, SLIDER_FADEOUT_TIME};

#[derive(Debug)]
pub struct ReverseArrow {
    pub time: f64,
    pub angle: f32,
}

#[derive(Debug)]
pub struct Tick {
    pub time: f64,
    pub pos: Vector2<f32>,
    pub slide: usize,
    pub is_reverse: bool,
}

pub struct SliderRender {
    pub texture: Arc<Texture>,
    pub quad: Arc<wgpu::Buffer>,
}

pub struct SliderResult {
    pub head: CircleHitResult,
    pub passed_checkpoints: Vec<usize>,
    pub end_passed: bool,
}

pub struct Slider {
    pub start_time: f64,
    pub duration: f64,
    pub curve: Curve,
    pub pos: Pos,
    pub repeats: i32,
    pub ticks: Vec<Tick>,


    pub reverse_arrows: Vec<ReverseArrow>,
    pub render: Option<SliderRender>,

    pub result: Option<SliderResult>,
}

impl Slider {
    pub fn is_visible(&self, time: f64, preempt: f32) -> bool {
        time > (self.start_time - preempt as f64)
            && time < self.start_time + self.duration + SLIDER_FADEOUT_TIME
    }

    pub fn is_head_hittable(
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

        if hit_error < hit_window.x50.round() {
            return Some(Hit::X300)
        }

        None
    }

    /// (x, y, width, height)
    pub fn bounding_box(&self, radius: f32) -> Rectangle {
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        let mut t = 0.0;
        while t <= 1.0 {
            let pos = self.curve.position_at(t);

            let pos = Pos {
                x: (self.pos.x + pos.x),
                y: (self.pos.y + pos.y),
            };

            min_x = min_x.min(pos.x - radius);
            min_y = min_y.min(pos.y - radius);

            max_x = max_x.max(pos.x + radius);
            max_y = max_y.max(pos.y + radius);

            t += 0.01;
        }

        let bottom_right = Vector2 { x: max_x, y: max_y };
        let top_left = Vector2 { x: min_x, y: min_y };

        Rectangle {
            top_left,
            bottom_right,
        }
    }
}
