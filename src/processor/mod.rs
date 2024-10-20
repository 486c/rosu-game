use cgmath::Vector2;
use replay_log::ReplayLog;

use crate::osu_input::{KeyboardState, OsuInput};

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
