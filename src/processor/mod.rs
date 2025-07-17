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
        let _span = tracy_client::span!("processor::on_pressed_down");

        self.set_cursor_pos(pos);
        let last = self.replay_log.last_input();

        if let Some(last) = last {
            self.store_input(OsuInput {
                ts,
                pos,
                keys: last.keys,
                hold: last.hold,
            });
        } else {
            self.store_input(OsuInput {
                ts,
                pos,
                keys: KeyboardState::empty(),
                hold: KeyboardState::empty(),
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
        let _span = tracy_client::span!("processor::process_all");

        'input_loop: for input in &self.queue {
            for object in objects.iter_mut() {
                match &mut object.kind {
                    crate::hit_objects::ObjectKind::Circle(circle) => {
                        let res = circle.update(
                            input,
                            hit_window,
                            circle_diameter
                        );

                        if res {
                            continue 'input_loop;
                        }

                    },
                    crate::hit_objects::ObjectKind::Slider(slider) => {
                        if slider.update(
                            input,
                            hit_window,
                            circle_diameter
                        ).is_some() {
                            continue 'input_loop;
                        };

                        slider.update_post(
                            input,
                            hit_window,
                            circle_diameter
                        );

                        continue;
                    },
                }
            }
        }

        self.queue.clear();
    }
    
    pub fn process(&mut self, _ts: f64, _objects: &mut [Object]) {
        todo!();
    }

    /// This function treats KeyboardState with reversed meaning
    /// `true` means that particular key is released
    pub fn store_keyboard_released(&mut self, ts: f64, state: KeyboardState) {
        let _span = tracy_client::span!("processor::store_keyboard_released");

        let last = self.replay_log.last_input();

        if let Some(last) = last {
            let last = last.keys;
            
            // Nothing to release
            if !last.is_keys_hit() {
                return;
            }

            self.store_input(OsuInput {
                ts,
                pos: self.last_cursor_pos,
                keys: KeyboardState {
                    k1: if state.k1 { false } else { last.k1 },
                    k2: if state.k2 { false } else { last.k2 },
                },
                hold: KeyboardState {
                    k1: !(last.k1 && state.k1),
                    k2: !(last.k2 && state.k2),
                },
            });
        } else {
            tracing::warn!("Trying to store release without previous input")
        }
    }
    
    pub fn store_keyboard_pressed(&mut self, ts: f64, state: KeyboardState) {
        let _span = tracy_client::span!("processor::store_keyboard_pressed");

        let last = self.replay_log.last_input();

        if let Some(last) = last {
            let last = last.keys;

            self.store_input(OsuInput {
                ts,
                pos: self.last_cursor_pos,
                keys: KeyboardState {
                    k1: state.k1,
                    k2: state.k2,
                },
                hold: KeyboardState {
                    k1: last.k1 && state.k1,
                    k2: last.k2 && state.k2,
                },
            });
        } else {
            self.store_input(OsuInput {
                ts,
                pos: self.last_cursor_pos,
                keys: state,
                hold: KeyboardState {
                    k1: false,
                    k2: false,
                },
            });
        }
    }

    pub fn store_input(&mut self, input: OsuInput) {
        let _span = tracy_client::span!("processor::store_input");
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
                },
                hold: KeyboardState::default(),
            };

            inputs.push(input);
        }
        
        // Post proccesing frames
        // stole from osu!lazer
        if inputs.len() >= 2 && inputs[1].ts < inputs[0].ts {
            inputs[1].ts = inputs[0].ts;
            inputs[0].ts = 0.0;
        }

        if inputs.len() >= 3 && inputs[0].ts > inputs[2].ts {
            inputs[0].ts = inputs[2].ts;
            inputs[1].ts = inputs[2].ts;
        }

        if inputs.len() >= 2 && inputs[1].pos == (256.0, -500.0).into() {
            inputs.remove(1);
        }

        if inputs.len() >= 1 && inputs[0].pos == (256.0, -500.0).into() {
            inputs.remove(0);
        }

        let mut current_frame: Option<OsuInput> = None;
        let mut new_inputs = Vec::new();

        for legacy_frame in &inputs {
            if let Some(current_frame) = &current_frame {
                if legacy_frame.ts < current_frame.ts {
                    continue
                }
            }

            current_frame = Some(legacy_frame.clone());
            new_inputs.push(legacy_frame.clone());
        }


        // Calculating is frame is hold
        let mut last = KeyboardState::default();
        for input in &mut new_inputs {

            input.hold = KeyboardState {
                k1: input.keys.k1 && last.k1,
                k2: input.keys.k2 && last.k2,
                //m1: input.keys.m1 && last.m1,
                //m2: input.keys.m2 && last.m2,
            };

            last = input.keys.clone();
        }
        
        new_inputs.iter().for_each(|x| {
            //println!("{} | is pressed: k1: {} k2: {} | is_hold: k1: {} k2: {}", x.ts, x.keys.k1, x.keys.k2, x.hold.k1, x.hold.k2);
        });

        Self {
            replay_log: ReplayLog::default(),
            queue: new_inputs,
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
            //m1: false,
            //m2: false,
        }
    );

    processor.store_keyboard_pressed(
        150.0, 
        KeyboardState {
            k1: true,
            k2: false,
            //m1: false,
            //m2: false,
        }
    );

    processor.store_keyboard_released(
        150.0, 
        KeyboardState {
            k1: true,
            k2: false,
            //m1: false,
            //m2: false,
        }
    );

    let last_input = processor.replay_log.last_input().unwrap();

    assert_eq!(last_input.keys.is_keys_hit(), false);

    processor.store_keyboard_released(
        200.0, 
        KeyboardState {
            k1: true,
            k2: false,
            //m1: false,
            //m2: false,
        }
    );

    let last_input = processor.replay_log.last_input().unwrap();
    assert_eq!(last_input.keys.is_keys_hit(), false);
    assert_eq!(last_input.ts, 150.0);
}
