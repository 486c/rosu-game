use std::sync::Arc;

use cgmath::Vector3;
use rosu::{buffer_write_or_init, graphics::Graphics, rgb::Rgb, vertex::Vertex};
use wgpu::{util::DeviceExt, BufferUsages, RenderPipeline};


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
    points_instance_data: Vec<PointsInstance>,
}



impl<'acr> AnalyzeCursorRenderer<'acr> {
    pub fn new(graphics: Arc<Graphics<'acr>>) -> Self {
        let surface_config = graphics.get_surface_config();
        
        let point_shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("point.wgsl"));

        let points_instance_data = Vec::new();
        let points_instance_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Hit Instance Buffer"),
                    contents: bytemuck::cast_slice(&points_instance_data),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
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
                        entry_point: "vs_main",
                        buffers: &[Vertex::desc(), PointsInstance::desc()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &point_shader,
                        entry_point: "fs_main",
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
        }
    }


    pub fn clear_cursor_data(&mut self) {
        self.points_instance_data.clear();
    }

    pub fn append_cursor_from_slice<T: Iterator<Item=PointsInstance>>(&mut self, iter: T) {
        for inst in iter {
            self.points_instance_data.push(
                PointsInstance {
                    pos: [inst.pos[0], inst.pos[1], 1.0],
                    color: inst.color,
                    alpha: 1.0,
                    scale: 1.0,
                }
            )
        }

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.points_instance_buffer,
            &self.points_instance_data,
            PointsInstance 
        )
    }
}
