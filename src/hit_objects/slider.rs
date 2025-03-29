use std::sync::Arc;

use cgmath::Vector2;
use rosu_map::{section::hit_objects::Curve, util::Pos};

use crate::{osu_input::{KeyboardState, OsuInput}, texture::Texture};

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
    pub lenience_passed: bool,
    pub holding_since: Option<f64>,
    pub in_radius_since: Option<f64>,
    pub start_keys: u8,
}

pub struct Slider {
    pub start_time: f64,
    pub duration: f64,

    pub curve: Curve,
    pub pos: Pos, // TODO: Make the same as in circle

    /// Total repeats
    /// Example:
    /// `*===R===R===*` => 3 repeats
    /// `*===R===*` => 2 repeats
    /// `*===*` => 1 repeats
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
    
    /// Returns slide index for certain time
    /// Indexes starts from 1
    /// Example:
    /// `*===R===*`
    ///  ^ ^ ^   ^
    ///  1 2 3   4
    ///
    ///  1 - Slider head at 0
    ///  3 - Reverse at 50
    ///  4 - Slider end at 100
    ///  2 - Position we want to know slide of at 25
    ///  For that time we will get 1
    ///  If time were 75 we should get 2
    #[inline]
    pub fn slide(&self, time: f64) -> i32 {
        let v1 = time - self.start_time;
        let v2 = self.duration / self.repeats as f64;
        ((v1 / v2).floor() as i32 + 1).max(1)
    }
    
    /// Gets progress taking slides and repeats into
    /// account
    ///
    /// 0.0 >= Return value <= 1.0
    #[inline]
    pub fn get_slider_progress(&self, time: f64) -> f64 {
        let v1 = time - self.start_time;
        let v2 = self.duration / self.repeats as f64;
        let slide = (v1 / v2).floor() + 1.0;

        let slide_start = self.start_time + (v2 * (slide - 1.0));

        let current = time;
        let end = slide_start + v2;

        let min = slide_start.min(end);
        let max = slide_start.max(end);

        if slide % 2.0 == 0.0 {
            1.0 - ((current - min)) / (max - min)
        } else {
            ((current - min)) / (max - min)
        }
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

        if !input.is_keys_hit_no_hold() {
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
                    in_radius_since: Some(input.ts),
                    lenience_passed: false,
                    start_keys: {
                        if input.keys.k1 && !input.hold.k1 {
                            1
                        } else if input.keys.k2 && !input.hold.k2 {
                            2
                        } else { panic!("Hitting a slider without any keys pressed?") }
                        /*
                        if input.keys.k1 {
                            1
                        } 
                        else if input.keys.k2 {
                            2
                        } else { panic!("Hitting a slider without any keys pressed?") }
                        */
                    },
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
        let mut slider_radius = circle_diameter as f64 / 2.0;

        let result = match &self.hit_result {
            Some(v) => v,
            None => return,
        };


        if result.state == SliderResultState::Passed {
            return
        }

        // TODO we should check if we are in radious only on 
        // checkpoints (slider points)
        slider_radius *= 2.4;

        // Position at slider for current input
        let slider_progress = self.get_slider_progress(input.ts);

        let pos_at_slider = self.curve.position_at(
            slider_progress
        );

        // Checking if cursor inside circle on slider 
        let (cx, cy) = (
            self.pos.x as f64 + pos_at_slider.x as f64, 
            self.pos.y as f64 + pos_at_slider.y as f64
        );
        let (px, py) = (input.pos.x, input.pos.y);

        let distance = ((px - cx).powf(2.0) + (py - cy).powf(2.0)).sqrt();
        let is_inside_circle = distance <= slider_radius;

        let result = self.hit_result.as_mut().unwrap();
        
        //println!("ts: {} | prg: {}", input.ts, &slider_progress);
        //println!("holding_since: {:?} | in_radius_since: {:?}", &result.holding_since, &result.in_radius_since);
        //println!(
            //"is: {} | distance: {} | radious: {} | circle_diameter: {}", 
            //&is_inside_circle, &distance, &slider_radius,
           //circle_diameter / 2.0
        //);
        //println!("pos: ({cx}, {cy})");


        // oh right, did i forget to say that we check slider end not at
        // slider end time?
        let lenience_hack_time = (self.start_time + self.duration / 2.0)
            .max(self.start_time + self.duration - 36.0);

        if !result.lenience_passed {
            if input.ts >= lenience_hack_time {
                //println!("LENIENCE CHECK: {} | hold: {:?} | rad: {:?}", lenience_hack_time, result.holding_since, result.in_radius_since);
                match (result.holding_since, result.in_radius_since) {
                    (Some(holding_since), Some(in_radius_since)) => {
                        if holding_since <= lenience_hack_time
                        && in_radius_since <= lenience_hack_time {
                            result.lenience_passed = true
                        }
                    },
                    _ => {}
                }
            }
        }

        if result.start_keys > 0 {
            if result.start_keys == 2 && !input.keys.k1 
            || result.start_keys == 1 && !input.keys.k2 {
                result.start_keys = 0;
            }
        }
        
        // Try to evaluate holding time 
        // and cursor position only if
        // input time is actually inside slider duration
        // attempt to avoid incorrect calculation
        // when incoming input is after slider end time
        if input.ts <= self.start_time + self.duration {
            let mouse_down_acceptance = if result.start_keys == 1 { 
                input.is_k1_hold()
            } else {
                input.is_k2_hold()
            };

            let is_holding = if result.start_keys < 1 {
                input.is_keys_hold()
            } else {
                mouse_down_acceptance
            };

            if !is_holding
            && result.holding_since.is_some() {
                result.holding_since = None
            }

            if is_holding
            && result.holding_since.is_none() {
                result.holding_since = Some(input.ts)
            }

            if !is_inside_circle
            && result.in_radius_since.is_some() {
                result.in_radius_since = None
            }

            if is_inside_circle
            && result.in_radius_since.is_none() {
                result.in_radius_since = Some(input.ts)
            }
        }

        if result.state == SliderResultState::Middle {
            // Gets a passed checkpoint 
            let closest_checkpoint = self.ticks.iter().enumerate().rev().find(|(i, x)| {
                x.time < input.ts && !result.passed_checkpoints.contains(i)
            });

            if let Some((i, checkpoint)) = closest_checkpoint {
                if let Some(holding_since) = result.holding_since {
                    if holding_since < checkpoint.time {
                        result.passed_checkpoints.push(i)
                    }
                }

                if (i + 1) == self.ticks.len() {
                    result.state = SliderResultState::End
                }
            } else {
                if self.ticks.is_empty() {
                    result.state = SliderResultState::End
                }
            }
        }

        if result.state == SliderResultState::End {
            if input.ts < self.start_time + self.duration {
                return;
            }
        
            // TODO temp for testing
            if result.lenience_passed {
                result.state = SliderResultState::Passed;
                result.end_passed = true;
                return;
            } else {
                result.end_passed = false;
                result.state = SliderResultState::Passed;
                return;
            }

            /*
            match (result.holding_since, result.in_radius_since) {
                (None, None) => {
                    if input.ts >= self.start_time + self.duration {
                        result.state = SliderResultState::Passed;
                    }
                },
                (None, Some(in_radius)) => {
                    if in_radius <= self.start_time + self.duration {
                        result.state = SliderResultState::Passed;
                        result.end_passed = true;
                    }
                },
                (Some(holding), None) => {
                    if holding <= self.start_time + self.duration {
                        result.state = SliderResultState::Passed;
                        result.end_passed = true;
                    }
                },
                (Some(holding), Some(in_radius)) => {
                    if holding < self.start_time + self.duration 
                    && in_radius < self.start_time + self.duration {
                        result.state = SliderResultState::Passed;
                        result.end_passed = true;
                    }
                },
            };
            if let (Some(holding_since), Some(in_radius_since)) = (
                result.holding_since, result.in_radius_since
            ) {
                if (holding_since <= self.start_time + self.duration 
                || in_radius_since <= self.start_time + self.duration)
                && input.ts >= self.start_time + self.duration 
                {
                    result.state = SliderResultState::Passed;
                    result.end_passed = true;
                }
            } else {
                if input.ts > self.start_time + self.duration {
                    result.state = SliderResultState::Passed;
                }
            }
            */
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
