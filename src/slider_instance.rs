
use cgmath::{Matrix4, Vector3, Vector2};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct SliderInstance {
    pub pos: [f32; 2], 
    pub alpha: f32,
}

impl SliderInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = 
        wgpu::vertex_attr_array![
            2 => Float32x2,
            3 => Float32,
        ];

    pub fn new(
        x: f32, y: f32, alpha: f32
    ) -> Self {
        let mat = Vector2::new(x, y);

        Self {
            pos: mat.into(),
            alpha
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
