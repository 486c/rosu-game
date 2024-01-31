use cgmath::{Matrix4, ortho};
//use ultraviolet::{Mat4, projection::lh_ydown::orthographic_wgpu_dx};
use winit::dpi::PhysicalSize;

pub struct Camera {
    pub mat: Matrix4<f32>,
}

impl Camera {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            mat: ortho(
                0.0,
                width,
                height,
                0.0,
                -1.0,
                1.0,
            ),
        }
    }

    pub fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        self.mat = ortho(
                0.0,
                new_size.width as f32,
                new_size.height as f32,
                0.0,
                -1.0, // znear
                1.0, // zfar
        );
    }
}
