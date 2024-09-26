#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct QuadInstance {
    pub pos: [f32; 3],
    pub color: [f32; 3],
    pub alpha: f32,
}

impl QuadInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = 
        wgpu::vertex_attr_array![
            2 => Float32x3,
            3 => Float32x3,
            4 => Float32,
        ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }

    pub fn from_xy_pos(width: f32, height: f32) -> Self {
        Self {
            pos: [width, height, 1.0],
            color: [0.0, 0.0, 0.0],
            alpha: 1.0
        }
    }


    pub fn from_xy_pos_alpha(width: f32, height: f32, alpha: f32) -> Self {
        Self {
            pos: [width, height, 1.0],
            color: [0.0, 0.0, 0.0],
            alpha
        }
    }
}


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct QuadInstanceAtlas {
    pub pos: [f32; 3],
    pub color: [f32; 3],
    // 4 uv's for each corner containing two x and y coordinates
    pub uvs: [[f32; 2]; 4],
    pub alpha: f32,
}
