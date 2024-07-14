pub mod circle;
pub mod slider;

use cgmath::Vector2;
use circle::Circle;
use rosu_map::section::hit_objects::HitObject;
use slider::Slider;

pub const SLIDER_FADEOUT_TIME: f64 = 80.0;
pub const CIRCLE_FADEOUT_TIME: f64 = 60.0;

#[derive(Clone)]
pub struct Rectangle {
    pub top_left: Vector2<f32>,
    pub bottom_right: Vector2<f32>,
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
                        render: None,
                    }),
                })
            }
            rosu_map::section::hit_objects::HitObjectKind::Circle(circle) => Some(Self {
                start_time: value.start_time,
                kind: ObjectKind::Circle(Circle {
                    start_time: value.start_time,
                    pos: circle.pos,
                }),
            }),
            _ => None,
        }
    }
}

pub enum ObjectKind {
    Circle(Circle),
    Slider(Slider),
}
