pub struct HitWindow {
    pub x300: f64,
    pub x100: f64,
    pub x50: f64,
}

impl HitWindow {
    pub fn from_od(od: f32) -> Self {
        HitWindow {
            x300: 80.0 - 6.0 * (od as f64),
            x100: 140.0 - 8.0 * (od as f64),
            x50: 200.0 - 10.0 * (od as f64),
        }
    }
}

impl Default for HitWindow {
    fn default() -> Self {
        Self {
            x300: 0.0,
            x100: 0.0,
            x50: 0.0,
        }
    }
}
