use cgmath::{ortho, Matrix4, SquareMatrix, Vector2, Vector3};
use winit::dpi::PhysicalSize;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera {
    pub proj: Matrix4<f32>,
    pub view: Matrix4<f32>,
}

impl Camera {
    pub fn new(width: f32, height: f32, scale: f32) -> Self {
        Self {
            proj: ortho(0.0, width, height, 0.0, -1.0, 1.0),
            view: Matrix4::identity() * Matrix4::from_scale(scale),
        }
    }

    pub fn ortho(left: f32, right: f32, bottom: f32, top: f32) -> Self {
        Self {
            proj: ortho(left, right, bottom, top, -1.0, 1.0),
            view: Matrix4::identity(),
        }
    }

    pub fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        self.proj = ortho(
            0.0,
            new_size.width as f32,
            new_size.height as f32,
            0.0,
            2.0,  // znear
            -2.0, // zfar
        );
    }

    pub fn transform(&mut self, scale: f32, offsets: Vector2<f32>) {
        self.view = Matrix4::identity()
            * Matrix4::from_translation(Vector3::new(offsets.x, offsets.y, 0.0))
            * Matrix4::from_nonuniform_scale(scale, scale, 1.0);
    }
}
