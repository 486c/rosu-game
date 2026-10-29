use std::sync::Arc;


use rosu::{graphics::Graphics, vertex::Vertex};
use wgpu::{util::DeviceExt, Buffer, BufferUsages, RenderPipeline};

use crate::lines_vertex::LinesVertex;

/// Value of the `selected` window-uniform field when nothing is selected.
pub const NO_SELECTED_FRAME: u32 = u32::MAX;

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

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
struct WindowUniform {
    start: u32,
    end: u32,
    selected: u32,
    pad: u32,
}

/// Initial element capacity of the GPU buffers, grown on demand.
const INITIAL_CAPACITY: usize = 1024;

pub struct AnalyzeCursorRenderer<'acr> {
    graphics: Arc<Graphics<'acr>>,

    pub points_pipeline: RenderPipeline,
    pub points_instance_buffer: Buffer,

    pub lines_pipeline: RenderPipeline,
    pub lines_vertex_buffer: Buffer,

    pub window_bind_group: wgpu::BindGroup,

    window_buffer: Buffer,

    // Capacity (in elements) of the GPU buffers.
    points_capacity: usize,
    lines_capacity: usize,
}

impl<'acr> AnalyzeCursorRenderer<'acr> {
    pub fn new(graphics: Arc<Graphics<'acr>>) -> Self {
        let surface_config = graphics.get_surface_config();

        let point_shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/point.wgsl"));

        let lines_shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/line.wgsl"));

        let points_instance_buffer = create_zeroed_buffer(
            &graphics.device,
            "analyze points instance buffer",
            (INITIAL_CAPACITY * std::mem::size_of::<PointsInstance>()) as u64,
        );

        let lines_vertex_buffer = create_zeroed_buffer(
            &graphics.device,
            "analyze lines vertex buffer",
            (INITIAL_CAPACITY * std::mem::size_of::<LinesVertex>()) as u64,
        );

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

        let window_bind_group_layout =
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
                    label: Some("analyze_window_bind_group_layout"),
                });

        let window_buffer = graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("analyze_window_buffer"),
                contents: bytemuck::bytes_of(&WindowUniform {
                    start: 0,
                    end: 0,
                    selected: NO_SELECTED_FRAME,
                    pad: 0,
                }),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let window_bind_group = graphics
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &window_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: window_buffer.as_entire_binding(),
                }],
                label: Some("analyze_window_bind_group"),
            });

        let lines_pipeline_layout =
            graphics
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("lines pipeline Layout"),
                    bind_group_layouts: &[
                        &camera_bind_group_layout,
                        &window_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let lines_pipeline =
            graphics
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("lines render pipeline"),
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
                        &window_bind_group_layout,
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
            lines_pipeline,
            lines_vertex_buffer,
            window_bind_group,
            window_buffer,
            points_capacity: INITIAL_CAPACITY,
            lines_capacity: INITIAL_CAPACITY,
        }
    }

    fn ensure_points_capacity(&mut self, len: usize) {
        if len <= self.points_capacity {
            return;
        }

        let new_cap = (len * 3 / 2).max(INITIAL_CAPACITY);
        let buffer = create_zeroed_buffer(
            &self.graphics.device,
            "analyze points instance buffer",
            (new_cap * std::mem::size_of::<PointsInstance>()) as u64,
        );

        // Dropping the old buffer is safe: wgpu keeps GPU memory alive
        // until in-flight commands referencing it complete.
        self.points_instance_buffer = buffer;
        self.points_capacity = new_cap;
    }

    fn ensure_lines_capacity(&mut self, len: usize) {
        if len <= self.lines_capacity {
            return;
        }

        let new_cap = (len * 3 / 2).max(INITIAL_CAPACITY);
        let buffer = create_zeroed_buffer(
            &self.graphics.device,
            "analyze lines vertex buffer",
            (new_cap * std::mem::size_of::<LinesVertex>()) as u64,
        );

        self.lines_vertex_buffer = buffer;
        self.lines_capacity = new_cap;
    }

    /// Replace all cursor data and upload it.
    pub fn set_data(&mut self, points: Vec<PointsInstance>, lines: Vec<LinesVertex>) {
        self.ensure_points_capacity(points.len());
        self.ensure_lines_capacity(lines.len());

        if !points.is_empty() {
            self.graphics.queue.write_buffer(
                &self.points_instance_buffer,
                0,
                bytemuck::cast_slice(&points),
            );
        }

        if !lines.is_empty() {
            self.graphics.queue.write_buffer(
                &self.lines_vertex_buffer,
                0,
                bytemuck::cast_slice(&lines),
            );
        }
    }

    /// Re-upload a sub-range of cursor data (`start` = absolute frame index).
    pub fn update_range(
        &mut self,
        start: usize,
        points: &[PointsInstance],
        lines: &[LinesVertex],
    ) {
        assert_eq!(points.len(), lines.len());
        assert!(start + points.len() <= self.points_capacity);
        assert!(start + lines.len() <= self.lines_capacity);

        self.graphics.queue.write_buffer(
            &self.points_instance_buffer,
            (start * std::mem::size_of::<PointsInstance>()) as u64,
            bytemuck::cast_slice(points),
        );
        self.graphics.queue.write_buffer(
            &self.lines_vertex_buffer,
            (start * std::mem::size_of::<LinesVertex>()) as u64,
            bytemuck::cast_slice(lines),
        );
    }

    /// Update the visible-window uniform (written once per rendered frame).
    pub fn set_window(&self, start: usize, end: usize, selected: Option<usize>) {
        let uniform = WindowUniform {
            start: start as u32,
            end: end as u32,
            selected: selected.map(|s| s as u32).unwrap_or(NO_SELECTED_FRAME),
            pad: 0,
        };

        self.graphics
            .queue
            .write_buffer(&self.window_buffer, 0, bytemuck::bytes_of(&uniform));
    }
}

fn create_zeroed_buffer(device: &wgpu::Device, label: &str, size: u64) -> Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
