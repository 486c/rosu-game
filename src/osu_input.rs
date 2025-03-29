use cgmath::Vector2;

#[derive(Debug, Copy, Clone)]
pub struct KeyboardState {
    pub k1: bool,
    pub k2: bool,
}

impl KeyboardState {
    pub fn is_keys_hit(&self) -> bool {
        self.k1 || self.k2
    }

    pub fn empty() -> Self {
        Self {
            k1: false,
            k2: false,
        }
    }
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self {
            k1: false,
            k2: false,
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
    /// Returns true if there's input which is not currently being held
    pub fn is_keys_hit_no_hold(&self) -> bool {
        let k1 = self.keys.k1 && !self.hold.k1;
        let k2 = self.keys.k2 && !self.hold.k2;

        k1 || k2
    }
    
    /// Returns true if there's any input that currently being held
    pub fn is_keys_hold(&self) -> bool {
        (self.keys.k1 && self.hold.k1) || (self.keys.k2 && self.hold.k2)
    }

    pub fn is_k1_hold(&self) -> bool {
        self.keys.k1 && self.hold.k1
    }

    pub fn is_k2_hold(&self) -> bool {
        self.keys.k2 && self.hold.k2
    }
}
