use std::sync::Arc;

use rosu::{
    graphics::Graphics,
    math::{OSU_COORDS_HEIGHT, OSU_COORDS_WIDTH},
    quad_instance::QuadInstance,
    vertex::Vertex,
};
use wgpu::{util::DeviceExt, BufferUsages, RenderPipeline};

pub struct PlayfieldRenderer {
    pub vertex_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub pipeline: RenderPipeline,
}

impl PlayfieldRenderer {
    pub fn new(graphics: Arc<Graphics<'_>>) -> Self {
        let surface_config = graphics.get_surface_config();

        let vertex_data = vec![Vertex::quad_centered(OSU_COORDS_WIDTH, OSU_COORDS_HEIGHT)];

        let instance_data = vec![QuadInstance {
            pos: [OSU_COORDS_WIDTH / 2.0, OSU_COORDS_HEIGHT / 2.0, 1.0],
            color: [92.0, 92.0, 92.0],
            alpha: 0.3,
            degree: 0.0,
        }];

        let instance_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("playfield instance buffer"),
                    contents: bytemuck::cast_slice(&instance_data),
                    usage: BufferUsages::VERTEX,
                });

        let vertex_buffer = graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer_playfield"),
                contents: bytemuck::cast_slice(&vertex_data),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            });

        let shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/quad.wgsl"));

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
                    label: Some("playfield_camera_bind_group_layout"),
                });

        let pipeline_layout =
            graphics
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("playfield pipeline Layout"),
                    bind_group_layouts: &[&camera_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = graphics
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("playfield render pipeline"),
                cache: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc(), QuadInstance::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
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
            vertex_buffer,
            pipeline,
            instance_buffer,
        }
    }
}
