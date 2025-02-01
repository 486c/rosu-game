use cgmath::Vector2;
use osu_replay_parser::replay::Replay;
use replay_log::ReplayLog;

use crate::{hit_objects::{circle::CircleHitResult, hit_window::HitWindow, slider::SliderResult, Object}, osu_input::{KeyboardState, OsuInput}};

pub mod replay_log;

/// Responsible for 
/// 1. Handling inputs
/// 2. Assigning hit results based on recorded inputs
pub struct OsuProcessor {
    replay_log: ReplayLog,
    queue: Vec<OsuInput>,

    last_cursor_pos: Vector2<f64>,
}

impl Default for OsuProcessor {
    fn default() -> Self {
        Self {
            last_cursor_pos: Vector2::new(0.0, 0.0),
            replay_log: Default::default(),
            queue: Vec::new(),
        }
    }
}

impl OsuProcessor {
    pub fn set_cursor_pos(&mut self, pos: Vector2<f64>) {
        self.last_cursor_pos = pos;
    }

    pub fn store_cursor_moved(&mut self, ts: f64, pos: Vector2<f64>) {
        self.set_cursor_pos(pos);
        let last = self.replay_log.last_input();

        if let Some(last) = last {
            self.store_input(OsuInput {
                ts,
                pos,
                keys: last.keys,
            });
        } else {
            self.store_input(OsuInput {
                ts,
                pos,
                keys: KeyboardState::empty(),
            });
        }
    }
    
    /// Processes all inputs frame by frame
    pub fn process_all(
        &mut self, 
        objects: &mut [Object], 
        hit_window: &HitWindow,
        circle_diameter: f32,
    ) {
        for input in &self.queue {
            for object in objects.iter_mut() {
                match &mut object.kind {
                    crate::hit_objects::ObjectKind::Circle(circle) => {
                        circle.update(
                            input,
                            hit_window,
                            circle_diameter
                        );
                    },
                    // Slider has a few states
                    // 1. SLIDER_START
                    // 2. All of the checkpoints aka slider ticks (also reverse slides)
                    // 2. SLIDER_END
                    crate::hit_objects::ObjectKind::Slider(slider) => {
                        continue;
                        /*
                        // if result is present that means slider head is already hit
                        if let Some(result) = &slider.result {
                        } else {
                            let result = slider.is_head_hittable(
                                input.ts,
                                hit_window,
                                input.pos,
                                circle_diameter
                            );

                            if result.is_none() {
                                continue
                            }

                            let result = result.unwrap();

                            slider.result = Some(SliderResult {
                                head: CircleHitResult {
                                    at: input.ts,
                                    pos: input.pos,
                                    result
                                },
                                passed_checkpoints: Vec::new(),
                                end_passed: false,
                            })
                        }
                        */
                    },
                }
            }
        }
    }
    
    pub fn process(&mut self, _ts: f64, _objects: &mut [Object]) {
        todo!();
    }

    /// This function treats KeyboardState with reversed meaning
    /// `true` means that particular key is released
    pub fn store_keyboard_released(&mut self, ts: f64, state: KeyboardState) {
        let last = self.replay_log.last_input();

        if let Some(last) = last {
            let last = last.keys;

            if !last.is_key_hit() {
                return;
            }

            self.store_input(OsuInput {
                ts,
                pos: self.last_cursor_pos,
                keys: KeyboardState {
                    k1: if last.k1 && state.k1 { false } else { last.k1 },
                    k2: if last.k2 && state.k2 { false } else { last.k2 },
                    m1: if last.m1 && state.m1 { false } else { last.m1 },
                    m2: if last.m2 && state.m2 { false } else { last.m2 },
                },
            });
        } else {
            tracing::warn!("Trying to store release without previous input")
        }

    }
    
    pub fn store_keyboard_pressed(&mut self, ts: f64, state: KeyboardState) {
        let last = self.replay_log.last_input();

        if let Some(last) = last {
            let last = last.keys;

            self.store_input(OsuInput {
                ts,
                pos: self.last_cursor_pos,
                keys: KeyboardState {
                    k1: if last.k1 && !state.k1 { last.k1 } else { state.k1 },
                    k2: if last.k2 && !state.k2 { last.k2 } else { state.k2 },
                    m1: if last.m1 && !state.m1 { last.m1 } else { state.m1 },
                    m2: if last.m2 && !state.m2 { last.m2 } else { state.m2 },
                },
            });
        } else {
            self.store_input(OsuInput {
                ts,
                pos: self.last_cursor_pos,
                keys: state,
            });
        }
    }

    fn store_input(&mut self, input: OsuInput) {
        self.queue.push(input.clone());
        self.replay_log.store_input(input);
    }
}

impl From<Replay> for OsuProcessor {
    fn from(value: Replay) -> Self {
        let mut ts = 0;
        let mut inputs = Vec::new();

        for frame in value.replay_data.frames {
            ts += frame.w;

            let input = OsuInput {
                ts: ts as f64,
                pos: Vector2::new(frame.x as f64, frame.y as f64),
                keys: KeyboardState {
                    k1: frame.z.contains(osu_replay_parser::replay::replay_data::Keys::K1),
                    k2: frame.z.contains(osu_replay_parser::replay::replay_data::Keys::K2),
                    m1: frame.z.contains(osu_replay_parser::replay::replay_data::Keys::M1),
                    m2: frame.z.contains(osu_replay_parser::replay::replay_data::Keys::M2),
                },
            };

            inputs.push(input);
        }

        inputs.iter().for_each(|x| {
            println!("time: {} | k1: {}, k2: {}", x.ts, x.keys.k1, x.keys.k2);
        });

        Self {
            replay_log: ReplayLog::default(),
            queue: inputs,
            last_cursor_pos: Vector2::new(0.0, 0.0),
        }
    }
}

#[test]
fn test_input_released() {
    let mut processor = OsuProcessor::default();

    processor.store_keyboard_pressed(
        100.0, 
        KeyboardState {
            k1: true,
            k2: false,
            m1: false,
            m2: false,
        }
    );

    processor.store_keyboard_pressed(
        150.0, 
        KeyboardState {
            k1: true,
            k2: false,
            m1: false,
            m2: false,
        }
    );

    processor.store_keyboard_released(
        150.0, 
        KeyboardState {
            k1: true,
            k2: false,
            m1: false,
            m2: false,
        }
    );

    let last_input = processor.replay_log.last_input().unwrap();

    assert_eq!(last_input.keys.is_key_hit(), false);

    processor.store_keyboard_released(
        200.0, 
        KeyboardState {
            k1: true,
            k2: false,
            m1: false,
            m2: false,
        }
    );

    let last_input = processor.replay_log.last_input().unwrap();
    assert_eq!(last_input.keys.is_key_hit(), false);
    assert_eq!(last_input.ts, 150.0);
}
