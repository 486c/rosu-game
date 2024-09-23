use cgmath::{Matrix4, Vector3, Vector2};

use crate::rgb::Rgb;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct HitCircleInstance {
    pub pos: [f32; 3], 
    pub color: [f32; 3], 
    pub alpha: f32,
    pub scale: f32,
}

impl HitCircleInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 4] = 
        wgpu::vertex_attr_array![
            2 => Float32x3,
            3 => Float32x3,
            4 => Float32,
            5 => Float32,
        ];

    pub fn new(
        x: f32, 
        y: f32, 
        z: f32, 
        alpha: f32,
        scale: f32,
        color: &Rgb
    ) -> HitCircleInstance {
        let mat = Vector3::new(x, y, z);

        Self {
            pos: mat.into(),
            color: color.to_gpu_values(),
            alpha,
            scale,
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct ApproachCircleInstance {
    pub pos: [f32; 3], 
    pub alpha: f32,
    pub scale: f32,
}

impl ApproachCircleInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = 
        wgpu::vertex_attr_array![
            2 => Float32x3,
            3 => Float32,
            4 => Float32,
        ];

    pub fn new(
        x: f32, y: f32, z: f32, alpha: f32, scale: f32
    ) -> Self {
        let mat = Vector3::new(x, y, z);

        Self {
            pos: mat.into(),
            alpha,
            scale
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
