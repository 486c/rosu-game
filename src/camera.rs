use std::num::NonZero;

use cgmath::{ortho, Matrix4, SquareMatrix, Vector2, Vector3};
use wgpu::{util::DeviceExt, BufferUsages};
use winit::dpi::PhysicalSize;

use crate::graphics::Graphics;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraGpu {
    pub proj: Matrix4<f32>,
    pub view: Matrix4<f32>,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Camera {
    pub gpu: CameraGpu,

    pub screen: Vector2<f32>,
    pub scale: f32,
    pub offsets: Vector2<f32>,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl Camera {
    pub fn new(
        graphics: &Graphics,
        width: f32, 
        height: f32, 
        scale: f32
    ) -> Self {
        let gpu = CameraGpu {
            proj: ortho(0.0, width, height, 0.0, -1.0, 1.0),
            view: Matrix4::identity() * Matrix4::from_scale(scale),
        };

        let buffer = graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("uniform_buffer"),
                contents: bytemuck::bytes_of(&gpu),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

        let bind_group_layout =
            graphics
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("camera_bind_group_layout"),
                });


        let bind_group = graphics
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
                label: Some("camera_bind_group"),
            });

        Self {
            gpu,
            screen: Vector2::new(width, height),
            scale,
            offsets: Vector2::new(0.0, 0.0),
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn ortho(
        graphics: &Graphics,
        left: f32, 
        right: f32, 
        bottom: f32, 
        top: f32
    ) -> Self {
        let gpu = CameraGpu {
            proj: ortho(left, right, bottom, top, -1.0, 1.0),
            view: Matrix4::identity(),
        };

        let buffer = graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("uniform_buffer"),
                contents: bytemuck::bytes_of(&gpu),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

        let bind_group_layout =
            graphics
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("camera_bind_group_layout"),
                });

        let bind_group = graphics
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
                label: Some("camera_bind_group"),
            });

        Self {
            gpu,
            screen: Vector2::new(right, bottom),
            scale: 1.0,
            offsets: Vector2::new(0.0, 0.0),
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn set_ortho(
        &mut self,
        graphics: &Graphics,
        left: f32, 
        right: f32, 
        bottom: f32, 
        top: f32
    ) {
        self.gpu.proj = ortho(left, right, bottom, top, -1.0, 1.0);

        self.write_buffers(graphics);
    }

    pub fn move_camera(&mut self, delta: Vector2<f32>) {
        self.offsets -= delta;
        self.transform(self.scale, self.offsets);
    }

    pub fn zoom(&mut self, zoom_delta: f32, zoom_center: Vector2<f32>) {
        let old_scale = self.scale;
        let old_offsets = self.offsets;

        let zoom_factor = if zoom_delta > 0.0 {
            1.0 + zoom_delta
        } else {
            1.0 / (1.0 - zoom_delta)
        };

        self.scale *= zoom_factor;

        let playfield_center = (zoom_center - old_offsets) / old_scale;
        let offset_adjustment = zoom_center - (playfield_center * self.scale + old_offsets);

        self.offsets += offset_adjustment;

        self.transform(self.scale, self.offsets);
    }

    pub fn screen_to_world(&self, screen_pos: Vector2<f32>) -> Vector2<f32> {
        (screen_pos - self.offsets) / self.scale
    }

    pub fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        self.screen = Vector2::new(new_size.width as f32, new_size.height as f32);
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
        self.scale = scale;
        self.offsets = offsets;

        self.gpu.view = Matrix4::identity()
            * Matrix4::from_translation(Vector3::new(offsets.x, offsets.y, 0.0))
            * Matrix4::from_nonuniform_scale(scale, scale, 1.0);
    }

    #[inline]
    pub fn write_buffers(&mut self, graphics: &Graphics) {
        log::debug!("writing camera buffers with scale: {}", self.scale);
        let mut view = graphics.queue.write_buffer_with(
            &self.buffer,
            0,
            NonZero::new(self.buffer.size()).unwrap()
        ).unwrap();

        view.copy_from_slice(bytemuck::bytes_of(&self.gpu))
    }

    #[inline]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    #[inline]
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    #[inline]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}


