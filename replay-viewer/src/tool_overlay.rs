use std::sync::Arc;

use rosu::{graphics::Graphics, osu_renderer::QUAD_INDECIES, vertex::Vertex};
use wgpu::{util::DeviceExt, Buffer, BufferUsages, RenderPipeline};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
pub struct OverlayInstance {
    pub pos: [f32; 3],
    pub color: [f32; 3],
    pub alpha: f32,
    pub scale: f32,
}

impl OverlayInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![
            2 => Float32x3,
            3 => Float32x3,
            4 => Float32,
            5 => Float32,
        ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct ToolOverlayRenderer<'r> {
    graphics: Arc<Graphics<'r>>,
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    instance_buffer: Buffer,
}

impl<'r> ToolOverlayRenderer<'r> {
    pub fn new(graphics: Arc<Graphics<'r>>) -> Self {
        let surface_config = graphics.get_surface_config();

        // Local quad spanning [-1, 1]; `scale` is the world-space radius.
        let vertex_data = Vertex::quad_centered(2.0, 2.0);

        let vertex_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("tool_overlay_vertex_buffer"),
                    contents: bytemuck::cast_slice(&vertex_data),
                    usage: BufferUsages::VERTEX,
                });

        let instance_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("tool_overlay_instance_buffer"),
                    contents: bytemuck::cast_slice(&[OverlayInstance {
                        pos: [0.0, 0.0, 0.0],
                        color: [1.0, 1.0, 1.0],
                        alpha: 1.0,
                        scale: 1.0,
                    }]),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });

        let shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/tool_overlay.wgsl"));

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
                    label: Some("tool_overlay_camera_bind_group_layout"),
                });

        let pipeline_layout =
            graphics
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tool_overlay pipeline Layout"),
                    bind_group_layouts: &[&camera_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = graphics
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("tool_overlay render pipeline"),
                cache: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc(), OverlayInstance::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
            pipeline,
            vertex_buffer,
            instance_buffer,
        }
    }

    pub fn set_instance(&mut self, instance: &OverlayInstance) {
        self.graphics
            .queue
            .write_buffer(&self.instance_buffer, 0, bytemuck::bytes_of(instance));
    }

    pub fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>, camera_bind_group: &wgpu::BindGroup) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw_indexed(0..QUAD_INDECIES.len() as u32, 0, 0..1);
    }
}
