use std::sync::Arc;

use cgmath::Vector2;
use rosu_map::{section::hit_objects::Curve, util::Pos};

use crate::{osu_input::OsuInput, texture::Texture};

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


#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum SliderResultState {
    /// Checking for hit on slider head
    #[default]
    Start,
    /// Hit a slider head, looking through all
    /// checkpoints
    Middle,
    /// Hit a slider head, passed all checkpoints
    /// checking a slider end
    End,
    Passed
}

#[derive(Debug)]
pub struct SliderResult {
    pub state: SliderResultState,
    pub head: CircleHitResult,
    pub passed_checkpoints: Vec<usize>,
    pub end_passed: bool,
    pub holding_since: Option<f64>,
}

pub struct Slider {
    pub start_time: f64,
    pub duration: f64,

    pub curve: Curve,
    pub pos: Pos, // TODO: Make the same as in circle
    pub repeats: i32,

    /// Including all ticks aka checkpoints
    pub ticks: Vec<Tick>,

    pub reverse_arrows: Vec<ReverseArrow>,
    pub render: Option<SliderRender>,

    pub hit_result: Option<SliderResult>,
}

impl Slider {
    #[inline]
    pub fn end_time(&self) -> f64 {
        self.start_time + self.duration
    }

    pub fn is_visible(&self, time: f64, preempt: f32) -> bool {
        time > (self.start_time - preempt as f64)
            && time < self.start_time + self.duration + SLIDER_FADEOUT_TIME
    }
    
    /// Check first slider head hit
    pub fn update(
        &mut self, 
        input: &OsuInput,
        hit_window: &HitWindow,
        circle_diameter: f32
    ) {
        if self.hit_result.is_some() {
            return;
        }

        if !input.keys.is_key_hit() {
            return;
        }

        let (cx, cy) = (self.pos.x as f64, self.pos.y as f64);
        let (px, py) = (self.pos.x as f64, self.pos.y as f64);

        let distance = ((px - cx).powf(2.0) + (py - cy).powf(2.0)).sqrt();

        if !(distance <= (circle_diameter / 2.0) as f64) {
            return;
        }

        let hit_error = (self.start_time - input.ts).abs();

        if hit_error < hit_window.x50.round() {
            self.hit_result = Some(
                SliderResult {
                    head: CircleHitResult {
                        at: input.ts,
                        pos: input.pos,
                        result: Hit::X300,
                    },
                    passed_checkpoints: vec![],
                    end_passed: false,
                    state: SliderResultState::Middle,
                    holding_since: Some(input.ts),
                }
            );
            return;
        }
    }

    pub fn update_post(
        &mut self, 
        input: &OsuInput,
        hit_window: &HitWindow,
        circle_diameter: f32
    ) {
        let result = match &mut self.hit_result {
            Some(v) => v,
            None => return,
        };

        if result.state == SliderResultState::Passed {
            return
        }

        if !input.keys.is_key_hit() && result.holding_since.is_some() {
            result.holding_since = None
        }

        if input.keys.is_key_hit() && result.holding_since.is_none() {
            result.holding_since = Some(input.ts)
        }

        if result.state == SliderResultState::Middle {
            // Gets a passed checkpoint 
            let closest_checkpoint = self.ticks.iter().enumerate().find(|(i, x)| {
                x.time < input.ts && !result.passed_checkpoints.contains(i)
            });

            if let Some((i, checkpoint)) = closest_checkpoint {
                if let Some(holding_since) = result.holding_since {
                    if holding_since < checkpoint.time {
                        println!("Passed {} input ts: {}", checkpoint.time, input.ts);
                        dbg!(&self.ticks);
                        result.passed_checkpoints.push(i)
                    }
                }

                if (i + 1) == self.ticks.len() {
                    result.state = SliderResultState::End
                }
            }
        }

        if result.state == SliderResultState::End {
            if let Some(holding_since) = result.holding_since {
                if holding_since < self.start_time + self.duration {
                    result.state = SliderResultState::Passed;
                    result.end_passed = true;
                }
            } else {
                if input.ts > self.start_time + self.duration {
                    result.state = SliderResultState::Passed;
                }
            }
        }
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
