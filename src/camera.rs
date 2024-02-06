use cgmath::{ortho, Matrix4, SquareMatrix};
//use ultraviolet::{Mat4, projection::lh_ydown::orthographic_wgpu_dx};
use winit::dpi::PhysicalSize;

pub struct Camera {
    pub proj: Matrix4<f32>,
    pub view: Matrix4<f32>,
}

impl Camera {
    pub fn new(width: f32, height: f32, scale: f32) -> Self {
        Self {
            proj: ortho(
                0.0,
                width,
                height,
                0.0,
                -1.0,
                1.0,
            ),
            view: Matrix4::identity()
                * Matrix4::from_scale(scale),
        }
    }

    pub fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        self.proj = ortho(
                0.0,
                new_size.width as f32,
                new_size.height as f32,
                0.0,
                -1.0, // znear
                1.0, // zfar
        );
    }

    pub fn scale(&mut self, scale: f32) {
        self.view
            = Matrix4::identity()
            * Matrix4::from_scale(scale);
    }

    pub fn calc_view_proj(&self) -> Matrix4<f32> {
        return self.proj * self.view;
    }
}
