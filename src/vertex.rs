use std::mem;

use cgmath::Vector3;

static PI: f32 = 3.1415926535897932384626433832795028841971693993751058209749445923078164;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: Vector3::<f32>,
    pub uv: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = 
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    pub fn cone(radius: f32) -> (Vec<Vertex>, Vec<u16>) {
        let mut v = Vec::new();
        let mut ind = Vec::new();

        const SEGMENTS: i32 = 40;

        for i in 0..SEGMENTS + 2{
            let theta = i as f32 * 2.0 * PI / SEGMENTS as f32;

            let x = radius * theta.sin();
            let z = radius * theta.cos();

            //v.push(Vertex {pos: [x, z, 1.0], uv: [0.0, 0.0]});
            v.push(Vertex {pos: [x, z, -1.0].into(), uv: [0.0, 0.0]});
        }

        v.push(Vertex {pos: [0.0, 0.0, 0.0].into(), uv: [1.0, 0.0]});


        for i in 0..SEGMENTS + 2 {
            ind.push(i as u16);
            ind.push((i as u16 + 1) % (SEGMENTS as u16 + 2));
            ind.push(SEGMENTS as u16 + 2);
        }

        (v, ind)
    }

    /// Used to create quad with centered origin
    pub fn quad_centered(width: f32, height: f32) -> [Vertex; 4] {
        let x = -width/2.0;
        let y = -height/2.0;

        [
            Vertex {pos: [x, y, 0.0].into(), uv:[0.0, 0.0]},
            Vertex {pos: [x, y + height, 0.0].into(), uv:[0.0, 1.0]},
            Vertex {pos: [x + width, y + height, 0.0].into(), uv:[1.0, 1.0]},
            Vertex {pos: [x + width, y, 0.0].into(), uv:[1.0, 0.0]},
        ]
    }

    pub fn quad_positional(x: f32, y: f32, width: f32, height: f32) -> [Vertex; 4] {
        [
            Vertex {pos: [x, y, 0.0].into(), uv:[0.0, 0.0]},
            Vertex {pos: [x, y + height, 0.0].into(), uv:[0.0, 1.0]},
            Vertex {pos: [x + width, y + height, 0.0].into(), uv:[1.0, 1.0]},
            Vertex {pos: [x + width, y, 0.0].into(), uv:[1.0, 0.0]},
        ]
    }

    pub fn quad_origin(origin_x: f32, origin_y: f32, width: f32, height: f32) -> [Vertex; 4] {
        [
            Vertex {pos: [origin_x, origin_y, 0.0].into(), uv:[0.0, 0.0]},
            Vertex {pos: [origin_x, origin_y + height, 0.0].into(), uv:[0.0, 1.0]},
            Vertex {pos: [origin_x + width, origin_y + height, 0.0].into(), uv:[1.0, 1.0]},
            Vertex {pos: [origin_x + width, origin_y, 0.0].into(), uv:[1.0, 0.0]},
        ]
    }
}
