
use cgmath::{Vector2, Vector3};

use crate::rgb::Rgb;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct SliderInstance {
    pub pos: [f32; 3], 
    pub alpha: f32,
    pub slider_border: [f32; 3],
}

impl SliderInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = 
        wgpu::vertex_attr_array![
            2 => Float32x3,
            3 => Float32,
            4 => Float32x3,
        ];

    pub fn new(
        x: f32, y: f32, z: f32, alpha: f32,
        slider_border: &Rgb,
    ) -> Self {
        let mat = Vector3::new(x, y, z);

        Self {
            pos: mat.into(),
            slider_border: slider_border.to_gpu_values(),
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
