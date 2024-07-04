use cgmath::Vector2;
use rosu_map::section::hit_objects::{HitObjectSlider, SplineType};

#[derive(PartialEq)]
pub enum SliderKind {
    Linear,
    Bezier,
    Catmull,
    PerfCircle
}

impl From<SplineType> for SliderKind {
    fn from(value: SplineType) -> Self {
        match value {
            SplineType::Catmull => Self::Catmull,
            SplineType::BSpline => Self::Bezier,
            SplineType::Linear => Self::Linear,
            SplineType::PerfectCurve => Self::PerfCircle,
        }
    }
}

pub struct Slider {
    pub kind: SliderKind,
    pub control_points: Vec<Vector2<f32>>
}

impl Slider {
    pub fn from_rosu_map(value: &HitObjectSlider) -> Option<Self> {
        let control_points = value.path.control_points();

        if control_points.len() < 2 {
            return None
        }

        let first_control_point = control_points.first();

        if first_control_point.is_none() {
            return None
        }

        let path_type = first_control_point.unwrap()
            .path_type;

        if path_type.is_none() {
            return None
        }

        let kind: SliderKind = path_type.unwrap().kind.into();

        if kind != SliderKind::Linear {
            return None
        }
        
        /*
        for i in 0..control_points.len() {
            let current = control_points[i];
        }
        */

        for point in control_points {
        }

        todo!();
    }
}
