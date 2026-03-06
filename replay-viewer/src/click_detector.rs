use std::time::Instant;

use winit::{dpi::PhysicalPosition, event::MouseButton};

#[derive(Debug, Clone, Copy)]
pub enum ClickState {
    Idle,
    Pressed { start: Instant, at: (f64, f64) },
    Holding,
}

impl ClickState {
    pub fn is_holding(&self) -> bool {
        match self {
            ClickState::Idle => false,
            ClickState::Pressed { .. } => false,
            ClickState::Holding => true,
        }
    }

    pub fn is_pressed(&self) -> bool {
        match self {
            ClickState::Idle => false,
            ClickState::Pressed { .. } => true,
            ClickState::Holding => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickResult {
    None,
    Click,
    HoldStart,
    HoldEnd,
}

pub struct ClickDetector {
    hold_threeshold: f32,
    state: ClickState,
    curr_position: (f64, f64),
}

const MOVE_TOLERANCE: f64 = 3.0;

impl ClickDetector {
    pub fn new(hold_threeshold: f32) -> Self {
        Self {
            state: ClickState::Idle,
            hold_threeshold,
            curr_position: (0.0, 0.0),
        }
    }

    pub fn update(&mut self) -> ClickResult {
        match self.state {
            ClickState::Idle => ClickResult::None,
            ClickState::Pressed { start, .. } => {
                if start.elapsed().as_secs_f32() * 1000.0 >= self.hold_threeshold {
                    self.state = ClickState::Holding;
                    ClickResult::HoldStart
                } else {
                    ClickResult::None
                }
            }
            ClickState::Holding => ClickResult::None,
        }
    }

    pub fn update_moved(&mut self, position: &PhysicalPosition<f64>) -> ClickResult {
        match self.state {
            ClickState::Idle => {},
            ClickState::Pressed { at, .. } => {
                let dx = (position.x - at.0).abs();
                let dy = (position.y - at.1).abs();

                if dx > MOVE_TOLERANCE || dy > MOVE_TOLERANCE {
                    self.state = ClickState::Holding;
                    return ClickResult::HoldStart;
                }

            },
            ClickState::Holding => {},
        }

        if self.state.is_pressed() {
        }

        self.curr_position = (position.x, position.y);
        ClickResult::None
    }

    pub fn update_pressed(&mut self) {
        self.state = ClickState::Pressed {
            start: Instant::now(),
            at: self.curr_position
        }
    }

    pub fn update_released(&mut self) -> ClickResult {
        match self.state {
            ClickState::Pressed { start, .. } => {
                self.state = ClickState::Idle;

                if start.elapsed().as_secs_f32() * 1000.0 >= self.hold_threeshold {
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
