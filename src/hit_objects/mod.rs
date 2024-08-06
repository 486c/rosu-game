pub mod circle;
pub mod slider;

use cgmath::Vector2;
use rosu_map::section::hit_objects::HitObject;

use slider::Slider;
use circle::Circle;

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
    pub color: usize,
}

impl Object {
    pub fn is_visible(&self, time: f64, preempt: f32) -> bool {
        match &self.kind {
            ObjectKind::Circle(circle) => circle.is_visible(time, preempt),
            ObjectKind::Slider(slider) => slider.is_visible(time, preempt),
        }
    }

    pub fn from_rosu(values: &[HitObject]) -> Vec<Object> {
        let mut objects = Vec::with_capacity(values.len());

        let mut color_index = 1;

        for value in values {
            if value.new_combo() {
                color_index += 1;
            }

            if color_index > 8 {
                color_index = 0;
            }

            match &value.kind {
                rosu_map::section::hit_objects::HitObjectKind::Slider(slider) => {
                    let mut slider = slider.clone();

                    let pos = slider.pos;
                    let duration = slider.duration();
                    let curve = slider.path.curve().clone();

                    objects.push(Self {
                        start_time: value.start_time,
                        color: color_index,
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
                rosu_map::section::hit_objects::HitObjectKind::Circle(circle) => objects.push(Self {
                    start_time: value.start_time,
                    color: color_index,
                    kind: ObjectKind::Circle(Circle {
                        start_time: value.start_time,
                        pos: circle.pos,
                    }),
                }),
                _ => {},
            };
        };

        objects
    }
}

pub enum ObjectKind {
    Circle(Circle),
    Slider(Slider),
}
