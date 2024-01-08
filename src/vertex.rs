use std::mem;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = 
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
    
    /*
    pub fn quad(min: f32, max: f32) -> Vec<Self> {
        vec![
            Vertex {pos: [min, min]},
            Vertex {pos: [min, max]},
            Vertex {pos: [max, max]},
            Vertex {pos: [max, max]},
            Vertex {pos: [max, min]},
            Vertex {pos: [min, min]},
        ]
    }
    */
}
