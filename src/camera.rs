use cgmath::{ortho, Matrix4, SquareMatrix, Vector2, Vector3};
use winit::dpi::PhysicalSize;
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Camera {
    pub gpu: CameraGpu,
}

impl Camera {
    pub fn new(width: f32, height: f32, scale: f32) -> Self {
        Self {
            gpu: CameraGpu {
                proj: ortho(0.0, width, height, 0.0, -1.0, 1.0),
                view: Matrix4::identity() * Matrix4::from_scale(scale),
            }
        }
    }

    pub fn ortho(left: f32, right: f32, bottom: f32, top: f32) -> Self {
        Self {
            gpu: CameraGpu {
                proj: ortho(left, right, bottom, top, -1.0, 1.0),
                view: Matrix4::identity(),
            }
        }
    }

    pub fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        self.gpu.proj = ortho(
            0.0,
            new_size.width as f32,
            new_size.height as f32,
            0.0,
            2.0,  // znear
            -2.0, // zfar
        );
    }

    pub fn transform(&mut self, scale: f32, offsets: Vector2<f32>) {
        self.gpu.view = Matrix4::identity()
            * Matrix4::from_translation(Vector3::new(offsets.x, offsets.y, 0.0))
            * Matrix4::from_nonuniform_scale(scale, scale, 1.0);
    }
}


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraGpu {
    pub proj: Matrix4<f32>,
    pub view: Matrix4<f32>,
}

/*

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Camera {
    pub gpu: CameraGpu,
    pub scale: f32,            // Track current scale
    pub offsets: Vector2<f32>, // Track current offsets
    pub base_size: Vector2<f32>, // Track the base size (width, height)
}

impl Camera {
    pub fn new(width: f32, height: f32, scale: f32) -> Self {
        Self {
            scale,
            offsets: Vector2::new(0.0, 0.0),
            base_size: Vector2::new(width, height),
            gpu: CameraGpu {
                proj: ortho(0.0, width, height, 0.0, -1.0, 1.0),
                view: Matrix4::identity() * Matrix4::from_scale(scale),
            },
        }
    }

    pub fn ortho(left: f32, right: f32, bottom: f32, top: f32) -> Self {
        Self {
            scale: 1.0,
            offsets: Vector2::new(0.0, 0.0),
            base_size: (right - left, top - bottom).into(),
            gpu: CameraGpu {
                proj: ortho(left, right, bottom, top, -1.0, 1.0),
                view: Matrix4::identity(),
            },
        }
    }

    pub fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        self.base_size = Vector2::new(new_size.width as f32, new_size.height as f32);
        self.update_projection();
    }

    pub fn transform(&mut self, scale: f32, offsets: Vector2<f32>) {
        self.scale = scale;
        self.offsets = offsets;
        self.update_view();
    }

    // New method for zooming
    pub fn zoom(&mut self, zoom_factor: f32, zoom_center: Vector2<f32>) {
        // Calculate the position in world space before zoom
        let old_world_pos = self.screen_to_world(zoom_center);

        // Update the scale
        self.scale *= zoom_factor;

        // Calculate the new world position for the same screen point
        let new_world_pos = self.screen_to_world(zoom_center);

        // Adjust the offset to keep the zoom center stable
        self.offsets += old_world_pos - new_world_pos;

        self.update_view();
    }

    // Helper method to convert screen coordinates to world coordinates
    pub fn screen_to_world(&self, screen_pos: Vector2<f32>) -> Vector2<f32> {
        (screen_pos - self.offsets) / self.scale
    }

    // Helper method to update the projection matrix
    fn update_projection(&mut self) {
        self.gpu.proj = ortho(
            0.0,
            self.base_size.x,
            self.base_size.y,
            0.0,
            -2.0,
            2.0,
        );
    }

    // Helper method to update the view matrix
    fn update_view(&mut self) {
        self.gpu.view = Matrix4::identity()
            * Matrix4::from_translation(Vector3::new(self.offsets.x, self.offsets.y, 0.0))
            * Matrix4::from_nonuniform_scale(self.scale, self.scale, 1.0);
    }
}
*/
