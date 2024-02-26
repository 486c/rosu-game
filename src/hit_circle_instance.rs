use cgmath::{Matrix4, Vector3, Vector2};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct HitCircleInstance {
    pub mat: [[f32; 4]; 4], 
    pub time: f32,
    pub is_approach: u32, // yep
}

impl HitCircleInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 6] = 
        wgpu::vertex_attr_array![
            2 => Float32x4,
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32,
            7 => Uint32,
        ];

    pub fn new(
        x: f32, y: f32, time: f32, is_approach: bool
    ) -> HitCircleInstance {
        let mat = 
            Matrix4::from_translation(Vector3::new(x, y, 0.0));

        Self {
            mat: mat.into(),
            is_approach: is_approach as u32,
            time
        }
    }

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}
