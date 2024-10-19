use cgmath::Vector2;
use replay_log::ReplayLog;

use crate::osu_input::{KeyboardState, OsuInput};

mod replay_log;

/// Responsible for 
/// 1. Handling inputs
/// 2. Assigning hit results based on recorded inputs
pub struct OsuProcessor {
    replay_log: ReplayLog,

    last_cursor_pos: Vector2<f64>,
}

impl Default for OsuProcessor {
    fn default() -> Self {
        Self {
            last_cursor_pos: Vector2::new(0.0, 0.0),
            replay_log: Default::default()
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
            self.replay_log.store_input(OsuInput {
                ts,
                pos,
                keys: last.keys,
            });
        } else {
            self.replay_log.store_input(OsuInput {
                ts,
                pos,
                keys: KeyboardState::empty(),
            });
        }

    }

    // TODO: unit test this shit
    /// This function treats KeyboardState with reversed meaning
    /// `true` means that particular key is released
    pub fn store_keyboard_released(&mut self, ts: f64, state: KeyboardState) {
        let last = self.replay_log.last_input();

        if let Some(last) = last {
            let last = last.keys;
            self.replay_log.store_input(OsuInput {
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

            self.replay_log.store_input(OsuInput {
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
            self.replay_log.store_input(OsuInput {
                ts,
                pos: self.last_cursor_pos,
                keys: state,
            });
        }
    }
}
