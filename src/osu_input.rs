use cgmath::Vector2;

#[derive(Debug)]
pub struct OsuInputState {
    pub ts: f64,
    pub cursor: CursorState,
    pub keyboard: KeyboardState,
    pub prev_keyboard: KeyboardState,
}

impl OsuInputState {
    pub fn update(&mut self, input: &OsuInput) {
        assert!(self.ts < input.ts); // TODO sanity check

        self.ts = input.ts;

        match input.kind {
            OsuInputKind::Keyboard(state) => {
                self.prev_keyboard = self.keyboard;
                self.keyboard = state;
            },
            OsuInputKind::CursorMoved(state) => self.cursor = state,
        }
    }

    pub fn is_key_hit(&self) -> bool {
        self.keyboard.is_key_hit()
    }

    pub fn is_holding(&self) -> bool {
        self.keyboard.is_key_hit() && self.prev_keyboard.is_key_hit()
    }
}

impl Default for OsuInputState {
    fn default() -> Self {
        Self {
            ts: 0.0,
            cursor: CursorState { x: 0.0, y: 0.0 },
            keyboard: KeyboardState { k1: false, k2: false, m1: false, m2: false },
            prev_keyboard: KeyboardState { k1: false, k2: false, m1: false, m2: false },
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct KeyboardState {
    k1: bool,
    k2: bool,
    m1: bool,
    m2: bool,
}

impl KeyboardState {
    pub fn is_key_hit(&self) -> bool {
        self.k1 || self.k2 || self.m1 || self.m2
    }
}

/// X and Y coordinates SHOULD BE in osu pixels
#[derive(Debug, Copy, Clone)]
pub struct CursorState {
    x: f64,
    y: f64,
}

impl From<CursorState> for Vector2<f64> {
    fn from(value: CursorState) -> Self {
        Vector2::new(
            value.x,
            value.y,
        )
    }
}

#[derive(Debug)]
pub enum OsuInputKind {
    Keyboard(KeyboardState),
    CursorMoved(CursorState)
}

#[derive(Debug)]
pub struct OsuInput {
    /// A timestamp relative to the beginning of the map (usually its 0)
    pub ts: f64, 

    kind: OsuInputKind,
}

impl OsuInput {
    pub fn key(ts: f64, k1: bool, k2: bool, m1: bool, m2: bool) -> Self {
        Self {
            ts,
            kind: OsuInputKind::Keyboard (KeyboardState{
                k1,
                k2,
                m1,
                m2,
            }),
        }
    }

    pub fn moved(ts: f64, pos: Vector2<f64>) -> Self {
        Self {
            ts,
            kind: OsuInputKind::CursorMoved(CursorState {
                x: pos.x,
                y: pos.y,
            }),
        }
    }

    pub fn is_key_hit(&self) -> bool {
        match &self.kind {
            OsuInputKind::Keyboard( state ) => state.is_key_hit(),
            _ => false,
        }
    }
}
