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
    
    /// Used to create quad with centered origin
    pub fn quad(width: f32, height: f32) -> [Vertex; 4] {
        let x = -width/2.0;
        let y = -height/2.0;

        [
            Vertex {pos: [x, y], uv:[0.0, 0.0]},
            Vertex {pos: [x, y + height], uv:[0.0, 1.0]},
            Vertex {pos: [x + width, y + height], uv:[1.0, 1.0]},
            Vertex {pos: [x + width, y], uv:[1.0, 0.0]},
        ]
    }
}
