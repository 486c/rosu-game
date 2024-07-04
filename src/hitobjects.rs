use std::sync::Arc;

use cgmath::Vector2;
use rosu_map::{section::hit_objects::{Curve, HitObject}, util::Pos};

use crate::texture::Texture;

pub const SLIDER_FADEOUT_TIME: f64 = 80.0;
pub const CIRCLE_FADEOUT_TIME: f64 = 60.0;

#[derive(Clone)]
pub struct Rectangle {
    pub top_left: Vector2::<f32>,
    pub bottom_right: Vector2::<f32>,
}

impl Rectangle {
    pub fn width(&self) -> f32 {
        (self.bottom_right - self.top_left).x.abs()
    }

    pub fn height(&self) -> f32 {
        (self.bottom_right - self.top_left).y.abs()
    }
}

pub struct Object {
    pub start_time: f64,
    pub kind: ObjectKind,
}

pub enum ObjectKind {
    Circle(Circle),
    Slider(Slider)
}

pub struct Circle {
    pub start_time: f64,
    pub pos: Pos,
}

impl Circle {
    pub fn is_visible(&self, time: f64, preempt: f32) -> bool {
        time > self.start_time - preempt as f64 
        && time < self.start_time + CIRCLE_FADEOUT_TIME 
    }
}

pub struct Slider {
    pub start_time: f64,
    pub duration: f64,
    pub curve: Curve,
    pub pos: Pos,
    pub repeats: i32,

    pub texture: Option<Arc<Texture>>,
    pub quad: Option<Arc<wgpu::Buffer>>,
    pub bounding_box: Option<Rectangle>
}

impl Slider {
    pub fn is_visible(&self, time: f64, preempt: f32) -> bool {
        time > self.start_time - preempt as f64 
        && time < self.start_time + self.duration + SLIDER_FADEOUT_TIME 
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
                x: self.pos.x + pos.x,
                y: self.pos.y + pos.y,
            };

            min_x = min_x.min(pos.x - radius);
            min_y = min_y.min(pos.y - radius);

            max_x = max_x.max(pos.x + radius);
            max_y = max_y.max(pos.y + radius);

            t += 0.01;
        }

        let bottom_right = Vector2 { x: max_x, y: max_y };
        let top_left = Vector2 { x: min_x, y: min_y };

        //(top_left, top_right, bottom_left, bottom_right)
        Rectangle {
            top_left, bottom_right
        }
            

        //(min_x, min_y, max_x-min_x, max_y - min_y)
        //(min_x, min_y, max_x, min_y)
        //(top_left.x, top_left.y, bottom_right.x, bottom_right.y)
    }
}

impl Object {
    pub fn is_visible(&self, time: f64, preempt: f32) -> bool {
        match &self.kind {
            ObjectKind::Circle(circle) => circle.is_visible(time, preempt),
            ObjectKind::Slider(slider) => slider.is_visible(time, preempt),
        }
    }

    pub fn from_rosu(value: &HitObject) -> Option<Self> {
        match &value.kind {
            rosu_map::section::hit_objects::HitObjectKind::Slider(slider) => {
                let mut slider = slider.clone();

                let pos = slider.pos;
                let duration = slider.duration();
                let curve = slider.path.curve().clone();

                Some(Self {
                    start_time: value.start_time,
                    kind: ObjectKind::Slider(Slider {
                        repeats: slider.span_count(),
                        start_time: value.start_time,
                        pos,
                        duration,
                        curve,
                        texture: None,
                        quad: None,
                        bounding_box: None
                    })
                })
            },
            rosu_map::section::hit_objects::HitObjectKind::Circle(circle) => {
                Some (
                    Self {
                        start_time: value.start_time,
                        kind: ObjectKind::Circle(
                            Circle {
                                start_time: value.start_time,
                                pos: circle.pos
                            }
                        )
                    }
                )
            }
            _ => None,
        }
    }
}
