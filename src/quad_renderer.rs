use std::sync::Arc;

use cgmath::Vector2;
use wgpu::{util::DeviceExt, BindGroup, Buffer, BufferUsages, IndexFormat, TextureView};

use crate::{buffer_write_or_init, camera::Camera, graphics::Graphics, quad_instance::QuadInstance, texture::{AtlasTexture, Texture}, vertex::{AtlasQuadVertex, Vertex}};

pub struct AtlasInfo {
    atlas_vertex_buffer: wgpu::Buffer,
    atlas_vertex_data: Vec<AtlasQuadVertex>,
    atlas_pipeline: wgpu::RenderPipeline
}

pub struct QuadRenderer<'qr> {
    graphics: Arc<Graphics<'qr>>,

    quad_vertex_buffer: wgpu::Buffer,
    quad_index_buffer: wgpu::Buffer,
    quad_pipeline: wgpu::RenderPipeline,
    
    camera: Camera,
    camera_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,

    /// Present if quad renderer is atlas based
    /// All atlas based operations should be
    /// handled inside QuadRenderer struct
    atlas: Option<AtlasInfo>,
}

impl<'qr> QuadRenderer<'qr> {
    pub fn new(
        graphics: Arc<Graphics<'qr>>, 
        is_using_atlas: bool
    ) -> Self {
        let quad_shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/quad.wgsl"));

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
                    label: None,
                    contents: bytemuck::cast_slice(crate::osu_renderer::QUAD_INDECIES),
                    usage: BufferUsages::INDEX,
                });

        let quad_vertex_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
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
                    label: None,
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
                    label: Some("quad render pipeline"),
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

        let atlas = if is_using_atlas {
            let atlas_quad_shader = graphics
                .device
                .create_shader_module(wgpu::include_wgsl!("shaders/quad_atlas.wgsl"));

            let atlas_vertex_data = Vec::new();

            let atlas_vertex_buffer = graphics.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("atlas quad vertex buffer"),
                    contents: bytemuck::cast_slice(&atlas_vertex_data),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });

            let quad_pipeline_layout =
                graphics
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("atlas quad pipeline layout"),
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
                    label: Some("altas quad render pipeline"),
                    layout: Some(&quad_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &atlas_quad_shader,
                        entry_point: "vs_main",
                        buffers: &[AtlasQuadVertex::desc()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        compilation_options: Default::default(),
                        module: &atlas_quad_shader,
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

            let atlas = AtlasInfo {
                atlas_vertex_buffer,
                atlas_vertex_data,
                atlas_pipeline: quad_pipeline,
            };

            Some(atlas)
        } else {
            None
        };

        Self {
            quad_vertex_buffer,
            quad_index_buffer,
            quad_pipeline,
            camera_bind_group,
            camera_buffer,
            graphics,
            camera,
            atlas,
        }
    }

    pub fn resize_camera(&mut self, new_size: &winit::dpi::PhysicalSize<u32>) {
        self.camera.resize(new_size);

        self.graphics
            .queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&self.camera));
    }

    pub fn transform_camera(&mut self, scale: f32, offset: Vector2<f32>) {
        self.camera.transform(scale, offset);

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

    pub fn atlas_quad_centered(
        x: f32, y: f32,
        width: f32, height: f32,
        image_index: u32,
        alpha: f32,
        atlas: &AtlasTexture
    ) -> [AtlasQuadVertex; 6] {
        let atlas_width = atlas.width();
        let u_min = image_index as f32 * atlas.image_width() / atlas_width;
        let u_max = (image_index as f32 + 1.0) * atlas.image_width() / atlas_width;
        let v_min = 0.0; // Assuming the images are aligned at the top
        let v_max = atlas.image_height() / atlas.height();

        let half_width = width / 2.0;
        let half_height = height / 2.0;
        
        /*
        [
            Vertex { pos: [x - half_width, y - half_height, 0.0].into(), uv: [u_min, v_min] }, // Bottom-left
            Vertex { pos: [x - half_width, y + half_height, 0.0].into(), uv: [u_min, v_max] }, // Top-left
            Vertex { pos: [x + half_width, y + half_height, 0.0].into(), uv: [u_max, v_max] }, // Top-right
            Vertex { pos: [x + half_width, y - half_height, 0.0].into(), uv: [u_max, v_min] }, // Bottom-right
        ]
        */

        [
            // First triangle (bottom-left, top-left, top-right)
            AtlasQuadVertex { pos: [x - half_width, y - half_height, 0.0].into(), uv: [u_min, v_min], alpha }, // Bottom-left
            AtlasQuadVertex { pos: [x - half_width, y + half_height, 0.0].into(), uv: [u_min, v_max], alpha }, // Top-left
            AtlasQuadVertex { pos: [x + half_width, y + half_height, 0.0].into(), uv: [u_max, v_max], alpha }, // Top-right

            // Second triangle (bottom-left, top-right, bottom-right)
            AtlasQuadVertex { pos: [x - half_width, y - half_height, 0.0].into(), uv: [u_min, v_min], alpha }, // Bottom-left
            AtlasQuadVertex { pos: [x + half_width, y + half_height, 0.0].into(), uv: [u_max, v_max], alpha }, // Top-right
            AtlasQuadVertex { pos: [x + half_width, y - half_height, 0.0].into(), uv: [u_max, v_min], alpha }, // Bottom-right
        ]
    }

    pub fn add_atlas_quad(
        &mut self,
        x: f32, y: f32,
        width: f32, height: f32,
        image_index: u32,
        alpha: f32,
        atlas: &AtlasTexture
    ) {
        let verticies = Self::atlas_quad_centered(x,y, width, height, image_index, alpha, atlas);

        if let Some(ref mut atlas) = &mut self.atlas {
            atlas.atlas_vertex_data.extend(verticies);
        }

        if let Some(ref mut atlas) = self.atlas {
            let data_len = atlas.atlas_vertex_data.len() as u64;
            let buffer_bytes_size = atlas.atlas_vertex_buffer.size();
            
            let buffer_len = buffer_bytes_size / size_of::<AtlasQuadVertex>() as u64;

            if data_len <= buffer_len {
                self.graphics.queue.write_buffer(&atlas.atlas_vertex_buffer, 0, bytemuck::cast_slice(&atlas.atlas_vertex_data));
            } else {
                atlas.atlas_vertex_buffer = self.graphics.device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(&atlas.atlas_vertex_data),
                        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                    }
                );
            }
        }
    }

    pub fn clear_atlas_buffers(&mut self) {
        if let Some(ref mut atlas) = self.atlas {
            atlas.atlas_vertex_data.clear();
        }
    }

    pub fn render_atlas_test(
        &self,
        view: &TextureView,
        texture: &BindGroup
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

            if let Some(atlas) = &self.atlas {
                render_pass.set_pipeline(&atlas.atlas_pipeline);
                render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(0, &texture, &[]);
                render_pass.set_vertex_buffer(0, atlas.atlas_vertex_buffer.slice(..));

                render_pass.draw(
                    0..atlas.atlas_vertex_data.len() as u32,
                    0..1
                );
            }
        }

        self.graphics.queue.submit([encoder.finish()]);
    }

    pub fn render_on_view_instanced(
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

