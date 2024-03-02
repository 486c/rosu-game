use cgmath::{Matrix4, Vector3};

pub struct ApproachCircleInstance {
    pub mat: [[f32; 4]; 4],
    pub time: f32,
}

impl ApproachCircleInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 5] = 
        wgpu::vertex_attr_array![
            2 => Float32x4,
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32,
        ];

    pub fn new(
        x: f32, y: f32, time: f32
    ) -> Self {
        let mat =
            Matrix4::from_translation(Vector3::new(x, y, 0.0));

        Self {
            mat: mat.into(),
            time,
        }
    }
}
