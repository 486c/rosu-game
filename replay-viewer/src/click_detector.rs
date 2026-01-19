use std::time::Instant;

use winit::event::MouseButton;

const HOLD_THREESHOLD_MS: f32 = 120.0;

#[derive(Debug, Clone, Copy)]
pub enum ClickState {
    Idle,
    Pressed { start: Instant },
    Holding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickResult {
    None,
    Click,
    HoldStart,
    HoldEnd,
}

pub struct ClickDetector {
    button: MouseButton,
    state: ClickState,
}

impl ClickDetector {
    pub fn new(button: MouseButton) -> Self {
        Self {
            button,
            state: ClickState::Idle,
        }
    }

    pub fn update(&mut self) -> ClickResult {
        match self.state {
            ClickState::Idle => ClickResult::None,
            ClickState::Pressed { start } => {
                if start.elapsed().as_secs_f32() * 1000.0 >= HOLD_THREESHOLD_MS {
                    self.state = ClickState::Holding;
                    ClickResult::HoldStart
                } else {
                    ClickResult::None
                }
            }
            ClickState::Holding => ClickResult::None,
        }
    }

    pub fn update_pressed(&mut self) {
        self.state = ClickState::Pressed {
            start: Instant::now(),
        }
    }

    pub fn update_released(&mut self) -> ClickResult {
        match self.state {
            ClickState::Pressed { start } => {
                self.state = ClickState::Idle;

                if start.elapsed().as_secs_f32() * 1000.0 >= HOLD_THREESHOLD_MS {
                    ClickResult::HoldEnd
                } else {
                    ClickResult::Click
                }
            }
            ClickState::Holding => {
                self.state = ClickState::Idle;

                ClickResult::HoldEnd
            }
            _ => {
                self.state = ClickState::Idle;
                ClickResult::None
            }
        }
    }
}
