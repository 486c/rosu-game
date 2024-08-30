use std::sync::Arc;

use wgpu::{util::DeviceExt, BindGroup, Buffer, BufferUsages, TextureView};

use crate::{camera::Camera, graphics::Graphics, quad_instance::QuadInstance, texture::Texture, vertex::Vertex};

pub struct QuadRenderer<'qr> {
    graphics: Arc<Graphics<'qr>>,

    quad_vertex_buffer: wgpu::Buffer,
    quad_index_buffer: wgpu::Buffer,
    quad_pipeline: wgpu::RenderPipeline,
    
    camera: Camera,
    camera_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
}

impl<'qr> QuadRenderer<'qr> {
    pub fn new(graphics: Arc<Graphics<'qr>>) -> Self {
        let quad_shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/hit_circle.wgsl"));

        let surface_config = graphics.get_surface_config();

        let camera = Camera::new(
            surface_config.width as f32,
            surface_config.height as f32,
            1.0,
        );

        let quad_index_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_index_buffer"),
                    contents: bytemuck::cast_slice(crate::osu_renderer::QUAD_INDECIES),
                    usage: BufferUsages::INDEX,
                });

        let quad_vertex_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&Vertex::quad_centered(1.0, 1.0)),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });
        
        let camera_buffer = graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("uniform_buffer"),
                contents: bytemuck::bytes_of(&camera),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

        let camera_bind_group_layout =
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


        let camera_bind_group = graphics
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
                label: Some("camera_bind_group"),
            });

        let quad_pipeline_layout =
            graphics
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("hitcircle pipeline Layout"),
                    bind_group_layouts: &[
                        &Texture::default_bind_group_layout(&graphics, 1),
                        &camera_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let quad_pipeline =
            graphics
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("hit_circle render pipeline"),
                    layout: Some(&quad_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &quad_shader,
                        entry_point: "vs_main",
                        buffers: &[Vertex::desc(), QuadInstance::desc()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        compilation_options: Default::default(),
                        module: &quad_shader,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: surface_config.format,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::SrcAlpha,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent::OVER,
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                });

        Self {
            quad_vertex_buffer,
            quad_index_buffer,
            quad_pipeline,
            camera_bind_group,
            camera_buffer,
            graphics,
            camera,
        }
    }

    pub fn resize_camera(&mut self, new_size: &winit::dpi::PhysicalSize<u32>) {
        self.camera.resize(new_size);

        self.graphics
            .queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&self.camera));
    }

    pub fn resize_vertex_centered(&self, width: f32, height: f32) {
        self.graphics
            .queue
            .write_buffer(&self.quad_vertex_buffer, 0, bytemuck::cast_slice(
                &Vertex::quad_centered(width, height)
            ));
    }

    pub fn create_instance_buffer(&self) -> Buffer {
        self.graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[0]),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            })
    }

    pub fn render_on_view(
        &self, 
        view: &TextureView,
        texture: &BindGroup,
        instances: &Buffer,
        amount: u32
    ) {
        let mut encoder =
            self.graphics
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: None,
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.quad_pipeline);
            render_pass.set_bind_group(0, &texture, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, instances.slice(..));

            render_pass.set_index_buffer(
                self.quad_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );

            render_pass.draw_indexed(
                0..crate::osu_renderer::QUAD_INDECIES.len() as u32,
                0,
                0..amount,
            );
        }

        self.graphics.queue.submit([encoder.finish()]);
    }
}

