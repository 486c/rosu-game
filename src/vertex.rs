use std::mem;

use cgmath::{Vector2, Vector3};

static PI: f32 = 3.1415926535897932384626433832795028841971693993751058209749445923078164;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
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

        v.push(Vertex {pos: [0.0, 0.0, 0.0], uv: [1.0, 0.0]});

        for i in 0..SEGMENTS + 2 {
            let theta = i as f32 * 2.0 * PI / SEGMENTS as f32;

            let x = radius * theta.sin();
            let z = radius * theta.cos();

            v.push(Vertex {pos: [x, z, 1.0], uv: [0.0, 0.0]});
        }

        for i in 2..SEGMENTS + 2 {
            let v1 = i;
            let v2 = 0;
            let v3 = i + 1;

            ind.push(v1 as u16);
            ind.push(v2 as u16);
            ind.push(v3 as u16);
        }

        (v, ind)
    }

    pub fn cone55(radius: f32) -> (Vec<Vertex>, Vec<u16>) {
        let mut v = Vec::new();
        let mut ind = Vec::new();

        const SEGMENTS: i32 = 40;

        for i in 0..SEGMENTS + 2{
            let theta = i as f32 * 2.0 * PI / SEGMENTS as f32;

            let x = radius * theta.sin();
            let z = radius * theta.cos();

            //v.push(Vertex {pos: [x, z, 1.0], uv: [0.0, 0.0]});
            v.push(Vertex {pos: [x, z, -0.5], uv: [0.0, 0.0]});
        }

        v.push(Vertex {pos: [0.0, 0.0, 0.0], uv: [1.0, 0.0]});


        for i in 0..SEGMENTS + 2 {
            ind.push(i as u16);
            ind.push((i as u16 + 1) % (SEGMENTS as u16 + 2));
            ind.push(SEGMENTS as u16 + 2);
        }

        (v, ind)
    }

    pub fn cone3(radius: f32) -> Vec<Vertex> {
        let mut v = Vec::new();

        // tip
        v.push(Vertex { pos: [0.0, 0.0, 0.5], uv: [1.0, 0.0] });

        const SEGMENTS: i32 = 42;

        for i in 0..SEGMENTS {
            let phase = i as f32 * PI * 2.0 / SEGMENTS as f32;

            v.push(Vertex { pos: [phase.sin() * radius, phase.cos() * radius, 0.0], uv: [0.0, 0.0] });
        }
        
        // bebra
        v.push(Vertex { pos: [radius * (0.0_f32).sin(), radius * (0.0_f32).cos(), 0.5], uv: [0.0, 0.0] });

        v
    }

    pub fn cone2(radius: f32) -> Vec<Vertex> {
        let mut v = Vec::new();

        let mut output = Vec::new();

        const SEGMENTS: i32 = 42;

        v.push(1.0);
        v.push(0.0);

        v.push(0.0);
        v.push(0.0);
        v.push(0.5);

        for i in 0..SEGMENTS {
            let phase = i as f32 * PI * 2.0 / SEGMENTS as f32;

            v.push(0.0);
            v.push(0.0);

            v.push(phase.sin());
            v.push(phase.cos());
            v.push(0.0);
        }

        v.push(0.0);
        v.push(0.0);

        v.push((0.0_f32).sin());
        v.push((0.0_f32).cos());
        v.push(0.5);

        // Fan
        /*
        for i in 0..v.len() / 5 {
            let pos = Vector3::new(
                radius * v[i * 5 + 2],
                radius * v[i * 5 + 3],
                radius * v[i * 5 + 4],
            );

            let text = Vector2::new(
                v[i * 5 + 0],
                v[i * 5 + 1]
            );

            output.push(
                Vertex { pos: pos.into(), uv: text.into() }
            );
        }
        */
        
        let start_vertex = Vector3::new(
            radius * v[0 * 5 + 2],
            radius * v[0 * 5 + 3],
            radius * v[0 * 5 + 4],
        );

        let start_uv = Vector2::new(
            v[0 * 5 + 0],
            v[0 * 5 + 1],
        );


        for i in 0..v.len() / 5 - 1 {
            output.push(Vertex { pos: start_vertex.into(), uv: start_uv.into() });

            output.push(Vertex {
                pos: Vector3::new(
                    radius * v[i * 5 + 2],
                    radius * v[i * 5 + 3],
                    radius * v[i * 5 + 4],
                ).into(),
                uv: Vector2::new(
                    v[i * 5 + 0],
                    v[i * 5 + 1],
                ).into()
            });

            output.push(Vertex {
                pos: Vector3::new(
                    radius * v[(i + 1) * 5 + 2],
                    radius * v[(i + 1) * 5 + 3],
                    radius * v[(i + 1) * 5 + 4],
                ).into(),
                uv: Vector2::new(
                    v[(i + 1) * 5 + 0],
                    v[(i + 1) * 5 + 1],
                ).into()
            })
        }

        output

    }

    /// Used to create quad with centered origin
    pub fn quad_centered(width: f32, height: f32) -> [Vertex; 4] {
        let x = -width/2.0;
        let y = -height/2.0;

        [
            Vertex {pos: [x, y, 0.0], uv:[0.0, 0.0]},
            Vertex {pos: [x, y + height, 0.0], uv:[0.0, 1.0]},
            Vertex {pos: [x + width, y + height, 0.0], uv:[1.0, 1.0]},
            Vertex {pos: [x + width, y, 0.0], uv:[1.0, 0.0]},
        ]
    }

    pub fn quad_positional(x: f32, y: f32, width: f32, height: f32) -> [Vertex; 4] {
        [
            Vertex {pos: [x, y, 0.0], uv:[0.0, 0.0]},
            Vertex {pos: [x, y + height, 0.0], uv:[0.0, 1.0]},
            Vertex {pos: [x + width, y + height, 0.0], uv:[1.0, 1.0]},
            Vertex {pos: [x + width, y, 0.0], uv:[1.0, 0.0]},
        ]
    }

    pub fn quad_origin(origin_x: f32, origin_y: f32, width: f32, height: f32) -> [Vertex; 4] {
        [
            Vertex {pos: [origin_x, origin_y, 0.0], uv:[0.0, 0.0]},
            Vertex {pos: [origin_x, origin_y + height, 0.0], uv:[0.0, 1.0]},
            Vertex {pos: [origin_x + width, origin_y + height, 0.0], uv:[1.0, 1.0]},
            Vertex {pos: [origin_x + width, origin_y, 0.0], uv:[1.0, 0.0]},
        ]
    }
}
