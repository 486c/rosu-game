use std::{num::NonZero, sync::Arc};

use cgmath::Vector3;
use rosu::{buffer_write_or_init, graphics::Graphics, rgb::Rgb, vertex::Vertex};
use wgpu::{util::DeviceExt, BufferUsages, RenderPipeline};

use crate::lines_vertex::LinesVertex;


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct PointsInstance {
    pub pos: [f32; 3], 
    pub color: [f32; 3], 
    pub alpha: f32,
    pub scale: f32,
}

impl PointsInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 4] = 
        wgpu::vertex_attr_array![
            2 => Float32x3,
            3 => Float32x3,
            4 => Float32,
            5 => Float32,
        ];

    pub fn new(
        x: f32, y: f32, z: f32, alpha: f32, scale: f32, color: &Rgb
    ) -> Self {
        let mat = Vector3::new(x, y, z);

        Self {
            pos: mat.into(),
            alpha,
            scale,
            color: color.to_gpu_values(),
        }
    }

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}


pub struct AnalyzeCursorRenderer<'acr> {
    graphics: Arc<Graphics<'acr>>,

    /// Points
    pub points_pipeline: RenderPipeline,
    pub points_instance_buffer: wgpu::Buffer,

    pub lines_pipeline: RenderPipeline,
    pub lines_vertex_data: Vec<LinesVertex>,
    pub lines_vertex_buffer: wgpu::Buffer,

    points_instance_data: Vec<PointsInstance>,
}

impl<'acr> AnalyzeCursorRenderer<'acr> {
    pub fn new(graphics: Arc<Graphics<'acr>>) -> Self {
        let surface_config = graphics.get_surface_config();
        

        let vertex_line = vec![Vertex {
            pos: Vector3::new(1.0, 1.0, 0.0),
            uv: [0.0, 0.0],
        }];

        let lines_vertex_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&vertex_line),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });
        
        let point_shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("point.wgsl"));

        let lines_shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("line.wgsl"));

        let points_instance_data = Vec::new();

        let points_instance_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Hit Instance Buffer"),
                    contents: bytemuck::cast_slice(&points_instance_data),
                    usage: BufferUsages::VERTEX,
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


        let lines_pipeline_layout =
            graphics
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("points pipeline Layout"),
                    bind_group_layouts: &[
                        &camera_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let lines_pipeline =
            graphics
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("points render pipeline"),
                    cache: None,
                    layout: Some(&lines_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &lines_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[LinesVertex::desc()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &lines_shader,
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
                        topology: wgpu::PrimitiveTopology::LineStrip,
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

        let points_pipeline_layout =
            graphics
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("points pipeline Layout"),
                    bind_group_layouts: &[
                        &camera_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let points_pipeline =
            graphics
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("points render pipeline"),
                    cache: None,
                    layout: Some(&points_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &point_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[Vertex::desc(), PointsInstance::desc()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &point_shader,
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
            graphics,
            points_pipeline,
            points_instance_buffer,
            points_instance_data,
            lines_pipeline,
            lines_vertex_data: Vec::new(),
            lines_vertex_buffer,
        }
    }

    pub fn points_data_mut(&mut self) -> &mut [PointsInstance] {
        &mut self.points_instance_data
    }

    pub fn lines_vertex_data_mut(&mut self) -> &mut [LinesVertex] {
        &mut self.lines_vertex_data
    }

    pub fn clear_cursor_data(&mut self) {
        let _span = tracy_client::span!("analyze_cursor_renderer::clear_cursor_data");
        self.points_instance_data.clear();
        self.lines_vertex_data.clear();
    }

    pub fn write_buffers(&mut self) {
        let _span = tracy_client::span!("analyze_cursor_renderer::write_buffers");

        let data_len = self.points_instance_data.len() as u64;
        let buffer_bytes_size = self.points_instance_buffer.size();

        let buffer_len = buffer_bytes_size / size_of::<PointsInstance>() as u64;

        if data_len <= buffer_len {
            let mut view = self.graphics.queue.write_buffer_with(
                &self.points_instance_buffer,
                0,
                NonZero::new(buffer_bytes_size).unwrap()
            ).unwrap();

            view.copy_from_slice(bytemuck::cast_slice(&self.points_instance_data))
        } else {
            let buffer = self.graphics.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&self.points_instance_data),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
            );

            self.points_instance_buffer.destroy();
            self.points_instance_buffer = buffer;
        }

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.lines_vertex_buffer,
            &self.lines_vertex_data,
            Vertex
        )
    }

    pub fn append_cursor_from_slice<T: Iterator<Item=PointsInstance>>(&mut self, iter: T) {
        let _span = tracy_client::span!("analyze_cursor_renderer::append_cursor_from_slice");
        for inst in iter {
            self.points_instance_data.push(
                PointsInstance {
                    pos: [inst.pos[0], inst.pos[1], 1.0],
                    color: inst.color,
                    alpha: 1.0,
                    scale: 1.0,
                }
            );

            self.lines_vertex_data.push(
                LinesVertex {
                    pos: Vector3::new(inst.pos[0], inst.pos[1], 1.0),
                    alpha: 1.0
                }
            );
        }

        self.write_buffers();
    }
}
