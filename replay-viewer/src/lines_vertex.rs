use std::mem;

use cgmath::Vector3;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LinesVertex {
    pub pos: Vector3::<f32>,
    pub alpha: f32,
}

impl LinesVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = 
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}
