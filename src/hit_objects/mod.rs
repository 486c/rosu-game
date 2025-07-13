pub mod circle;
pub mod slider;
pub mod hit_window;

use cgmath::Vector2;
use hit_window::HitWindow;
use rosu_map::Beatmap;

use slider::{Slider, Tick};
use circle::Circle;

use crate::math::{calc_opposite_direction_degree, calc_progress};

// In ms
pub const SLIDER_FADEOUT_TIME: f64 = 80.0;
pub const CIRCLE_FADEOUT_TIME: f64 = 60.0;
pub const JUDGMENTS_FADEOUT_TIME: f64 = 300.0;
pub const CIRCLE_SCALEOUT_MAX: f64 = 1.4;
pub const REVERSE_ARROW_FADEOUT: f64 = 200.0;
pub const REVERSE_ARROW_FADEIN: f64 = 300.0;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Hit {
    X300,
    X100,
    X50,
    MISS,
}

// TODO: Remove this rectangle shit or move it to other place
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
    pub fn is_visible(&self, time: f64, preempt: f32, hit_window: &HitWindow) -> bool {
        match &self.kind {
            ObjectKind::Circle(circle) => circle.is_visible(time, preempt, hit_window),
            ObjectKind::Slider(slider) => slider.is_visible(time, preempt),
        }
    }

    pub fn is_judgements_visible(&self, time: f64, preempt: f32) -> bool {
        match &self.kind {
            ObjectKind::Circle(circle) => circle.is_judgements_visible(time, preempt),
            ObjectKind::Slider(_) => false,
        }
    }

    pub fn from_rosu(map: &Beatmap) -> Vec<Object> {

        let mut color_index = 1;
        let tick_rate = map.slider_tick_rate;


        let values = &map.hit_objects;
        let mut objects = Vec::with_capacity(values.len());

        for value in values {
            if value.new_combo() {
                color_index += 1;
            }

            if color_index > 8 {
                color_index = 0;
            }

            match &value.kind {
                rosu_map::section::hit_objects::HitObjectKind::Slider(slider) => {
                    //dbg!("====++=========");
                    let timing = map.control_points.timing_point_at(value.start_time);
                    let beat_len = timing.unwrap().beat_len; // TODO remove unwrap
                    let tick_every_ms = beat_len / tick_rate;

                    let mut slider = slider.clone();

                    let pos = slider.pos;
                    let duration = slider.duration();
                    let curve = slider.path.curve().clone();

                    let slide_duration = slider.duration() / f64::from(slider.span_count());

                    let mut ticks = Vec::new();
                    let mut checkpoints = Vec::new();

                    //dbg!(tick_every_ms);
                    //dbg!(&value.start_time);
                    //dbg!(duration);
                    
                    // Calculating ticks for the slider
                    let mut i = value.start_time + tick_every_ms;
                    while i < value.start_time + duration {
                        if tick_every_ms >= slide_duration {
                            break
                        };

                        let v1 = i - value.start_time;
                        let v2 = duration / slider.span_count() as f64;
                        let slide = (v1 / v2).floor() as usize + 1;

                        let slide_start = value.start_time + (v2 * (slide as f64 - 1.0));
                        let slide_end = slide_start + v2;

                        let mut progress = calc_progress(
                            i, 
                            slide_start, 
                            slide_end
                        );

                        if slide % 2 == 0 {
                            progress = 1.0 - progress;
                        }

                        let curve_pos = curve.position_at(progress);

                        let pos = Vector2::new(
                            slider.pos.x + curve_pos.x, 
                            slider.pos.y + curve_pos.y
                        );

                        if (slide_end - i) > 10.0 {
                            ticks.push(Tick {
                                pos,
                                time: i,
                                slide,
                                is_reverse: false,
                            });

                            checkpoints.push(Tick {
                                pos,
                                time: i,
                                slide,
                                is_reverse: false,
                            });
                        }

                        i += tick_every_ms;
                    }
                
                    let mut reverse_arrows = Vec::new();
                    //dbg!(slider.span_count());
                    for repeat in 0..slider.span_count() - 1 {
                        let repeat = repeat + 1;
                        let v2 = duration / slider.span_count() as f64;

                        let (pos1, pos2) = if repeat % 2 == 0 {
                            let p1 = curve.position_at(0.0);
                            let p2 = curve.position_at(0.05);

                            (
                                Vector2::new(slider.pos.x + p1.x, slider.pos.y + p1.y),
                                Vector2::new(slider.pos.x + p2.x, slider.pos.y + p2.y),
                            )
                        } else {
                            let p1 = curve.position_at(1.0);
                            let p2 = curve.position_at(0.95);

                            (
                                Vector2::new(slider.pos.x + p1.x, slider.pos.y + p1.y),
                                Vector2::new(slider.pos.x + p2.x, slider.pos.y + p2.y),
                            )
                        };

                        let angle = -calc_opposite_direction_degree(pos2, pos1);

                        let slide_start = value.start_time + (v2 * (repeat as f64));

                        //dbg!(slide_start);

                        reverse_arrows.push(
                            slider::ReverseArrow {
                                time: slide_start,
                                angle,
                            }
                        );

                        checkpoints.push(Tick {
                            pos: pos1,
                            time: slide_start,
                            slide: repeat as usize,
                            is_reverse: true,
                        });
                    }

                    checkpoints
                        .sort_by(|a, b| 
                            a.time.partial_cmp(&b.time).expect("failed to compare")
                        );

                    objects.push(Self {
                        start_time: value.start_time,
                        color: color_index,
                        kind: ObjectKind::Slider(Slider {
                            repeats: slider.span_count(),
                            start_time: value.start_time,
                            pos,
                            duration,
                            curve,
                            ticks,
                            render: None,
                            reverse_arrows,
                            hit_result: None,
                            checkpoints,
                        }),
                    })
                }
                rosu_map::section::hit_objects::HitObjectKind::Circle(circle) => objects.push(Self {
                    start_time: value.start_time,
                    color: color_index,
                    kind: ObjectKind::Circle(Circle {
                        start_time: value.start_time,
                        pos: circle.pos,
                        hit_result: None,
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
