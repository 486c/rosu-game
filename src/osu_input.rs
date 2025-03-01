use cgmath::Vector2;

#[derive(Debug, Copy, Clone)]
pub struct KeyboardState {
    pub k1: bool,
    pub k2: bool,
    pub m1: bool,
    pub m2: bool,
}

impl KeyboardState {
    pub fn is_key_hit(&self) -> bool {
        self.k1 || self.k2 || self.m1 || self.m2
    }

    pub fn empty() -> Self {
        Self {
            k1: false,
            k2: false,
            m1: false,
            m2: false,
        }
    }
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self {
            k1: false,
            k2: false,
            m1: false,
            m2: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OsuInput {
    /// A timestamp relative to the beginning of the map
    pub ts: f64, 

    /// Cursors position
    pub pos: Vector2<f64>,

    /// Keys pressed
    pub keys: KeyboardState,

    /// Is current input is holding one
    pub hold: KeyboardState,
}

impl OsuInput {
    /// Retruns only there's input which is not currently hold
    pub fn is_key_hit_no_hold(&self) -> bool {
        let k1 = self.keys.k1 && !self.hold.k1;
        let k2 = self.keys.k2 && !self.hold.k2;
        let m1 = self.keys.m1 && !self.hold.m1;
        let m2 = self.keys.m2 && !self.hold.m2;

        k1 || k2 || m1 || m2
    }
}
