use rosu::hit_objects::Hit;

#[derive(Debug)]
pub enum JudgementObjectKind {
    Circle,
    // Slider,
    // Spinner,
}

pub struct JudgementPoint {
    pub ts: f64,
    pub kind: JudgementObjectKind,
    pub hit: Hit,
}
