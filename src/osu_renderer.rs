use std::{mem::size_of, sync::Arc};

use cgmath::Vector2;
use smallvec::SmallVec;
use wgpu::{
    util::DeviceExt, BindGroup, BindingType, BufferUsages, Extent3d,
    RenderPipeline, ShaderStages, TextureDescriptor, TextureDimension, TextureSampleType,
    TextureUsages, TextureView, TextureViewDimension,
};
use winit::dpi::PhysicalSize;

static SLIDER_SCALE: f32 = 2.0;

pub struct JudgementsEntry {
    pos: Vector2<f64>,
    alpha: f32,
    result: hit_objects::Hit,
}

use crate::{
    camera::Camera, config::Config, graphics::Graphics, hit_circle_instance::{ApproachCircleInstance, HitCircleInstance}, hit_objects::{self, slider::SliderRender, Object, CIRCLE_FADEOUT_TIME, CIRCLE_SCALEOUT_MAX, JUDGMENTS_FADEOUT_TIME, SLIDER_FADEOUT_TIME}, math::{calc_playfield, calc_playfield_scale_factor, calc_progress, get_hitcircle_diameter, lerp}, quad_instance::QuadInstance, quad_renderer::QuadRenderer, skin_manager::SkinManager, slider_instance::SliderInstance, texture::{AtlasTexture, DepthTexture, Texture}, vertex::Vertex
};

#[macro_export]
macro_rules! buffer_write_or_init {
    ($queue:expr, $device:expr, $buffer:expr, $data:expr, $t: ty) => {{
        let data_len = $data.len() as u64;
        let buffer_bytes_size = $buffer.size();

        let buffer_len = buffer_bytes_size / size_of::<$t>() as u64;

        if data_len <= buffer_len {
            $queue.write_buffer(&$buffer, 0, bytemuck::cast_slice($data))
        } else {
            let buffer = $device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice($data),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            });

            $buffer = buffer;
        }
    }};
}

pub const QUAD_INDECIES: &[u16] = &[0, 1, 2, 0, 2, 3];

pub struct OsuRenderer<'or> {
    // Graphics State
    graphics: Arc<Graphics<'or>>,

    // State
    scale: f32,
    offsets: Vector2<f32>,
    hit_circle_diameter: f32,

    // Quad verticies
    quad_verticies: [Vertex; 4],

    // Camera
    camera: Camera,
    camera_bind_group: BindGroup,
    camera_buffer: wgpu::Buffer,

    // Approach circle
    approach_circle_pipeline: RenderPipeline,
    //approach_circle_texture: Texture,
    approach_circle_instance_buffer: wgpu::Buffer,
    approach_circle_instance_data: SmallVec<[ApproachCircleInstance; 32]>,

    // quad textured + color
    quad_colored_pipeline: RenderPipeline,

    // Hit Circle
    hit_circle_pipeline: RenderPipeline,
    hit_circle_vertex_buffer: wgpu::Buffer,
    hit_circle_index_buffer: wgpu::Buffer,
    hit_circle_instance_data: Vec<HitCircleInstance>,
    hit_circle_instance_buffer: wgpu::Buffer,

    // Slider to texture
    slider_instance_buffer: wgpu::Buffer,
    slider_instance_data: Vec<SliderInstance>,
    slider_pipeline: RenderPipeline,
    slider_indecies: SmallVec<[u16; 16]>,

    slider_vertex_buffer: wgpu::Buffer,
    slider_index_buffer: wgpu::Buffer,
    slider_verticies: SmallVec<[Vertex; 256]>,

    // Slider texture to screen
    slider_to_screen_verticies: [Vertex; 4],
    slider_to_screen_vertex_buffer: wgpu::Buffer,
    slider_to_screen_render_pipeline: RenderPipeline,
    slider_to_screen_instance_buffer: wgpu::Buffer,
    slider_to_screen_instance_data: Vec<SliderInstance>,

    // Slider follow circle
    follow_points_instance_data: Vec<HitCircleInstance>,
    follow_points_instance_buffer: wgpu::Buffer,

    // Slider body queue
    slider_to_screen_textures: SmallVec<[(Arc<Texture>, Arc<wgpu::Buffer>, Option<u32>); 32]>,

    // Slider settings
    slider_settings_buffer: wgpu::Buffer,
    slider_settings_bind_group: BindGroup,

    depth_texture: DepthTexture,

    quad_debug: QuadRenderer<'or>,

    quad_debug_instance_data: Vec<QuadInstance>,
    quad_debug_instance_data2: Vec<QuadInstance>,
    quad_debug_buffer: wgpu::Buffer,
    quad_debug_buffer2: wgpu::Buffer,
    
    /// Queue of judgements that needs to be rendered
    /// Should be cleared after everything inside is rendered
    judgements_queue: Vec<JudgementsEntry>,
}

impl<'or> OsuRenderer<'or> {
    pub fn new(graphics: Arc<Graphics<'or>>, config: &Config) -> Self {
        let (graphics_width, graphics_height) = graphics.get_surface_size();
        let surface_config = graphics.get_surface_config();

        let hit_circle_shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/hit_circle.wgsl"));

        let quad_colored_shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/quad_textured.wgsl"));

        let approach_circle_shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/approach_circle.wgsl"));

        let slider_shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/slider.wgsl"));

        let slider_to_screen_shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/slider_to_screen.wgsl"));

        let depth_texture =
            DepthTexture::new(&graphics, graphics_width, graphics_height, 1);

        let quad_verticies = Vertex::quad_centered(1.0, 1.0);

        let all_depth = None;

        let hit_circle_vertex_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&quad_verticies),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });

        let hit_circle_index_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_index_buffer"),
                    contents: bytemuck::cast_slice(QUAD_INDECIES),
                    usage: BufferUsages::INDEX,
                });

        let hit_circle_instance_data = Vec::new();

        let hit_circle_instance_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Hit Instance Buffer"),
                    contents: bytemuck::cast_slice(&hit_circle_instance_data),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });

        let approach_circle_instance_data = SmallVec::new();

        let approach_circle_instance_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Hit Instance Buffer"),
                    contents: bytemuck::cast_slice(&approach_circle_instance_data),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });

        /* Camera stuff */
        let camera = Camera::new(
            graphics_width as f32,
            graphics_height as f32,
            1.0,
        );

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


        let slider_settings_buffer = graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("uniform_buffer"),
                    contents: bytemuck::bytes_of(&config.slider),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });

        let slider_settings_bind_group_layout =
            graphics
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("slider_settings bind group layout"),
                });

        let slider_settings_bind_group = graphics
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &slider_settings_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: slider_settings_buffer.as_entire_binding(),
                }],
                label: Some("slider_settings bind group"),
            });

        let approach_circle_pipeline_layout =
            graphics
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("approach circle pipeline Layout"),
                    bind_group_layouts: &[
                        //&approach_circle_texture.bind_group_layout,
                        &camera_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let approach_circle_pipeline =
            graphics
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("approach circle render pipeline"),
                    layout: Some(&approach_circle_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &approach_circle_shader,
                        entry_point: "vs_main",
                        buffers: &[Vertex::desc(), ApproachCircleInstance::desc()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &approach_circle_shader,
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
                    depth_stencil: all_depth.clone(),
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                });

        let hit_circle_pipeline_layout =
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

        let hit_circle_pipeline =
            graphics
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("hit_circle render pipeline"),
                    layout: Some(&hit_circle_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &hit_circle_shader,
                        entry_point: "vs_main",
                        buffers: &[Vertex::desc(), HitCircleInstance::desc()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        compilation_options: Default::default(),
                        module: &hit_circle_shader,
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
                    depth_stencil: all_depth.clone(),
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                });

        let hit_circle_pipeline_layout =
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

        let quad_colored_pipeline =
            graphics
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("quad colored render pipeline"),
                    layout: Some(&hit_circle_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &quad_colored_shader,
                        entry_point: "vs_main",
                        buffers: &[Vertex::desc(), HitCircleInstance::desc()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        compilation_options: Default::default(),
                        module: &quad_colored_shader,
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
                    depth_stencil: all_depth.clone(),
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                });



        let (slider_verticies, slider_indecies) = Vertex::cone(5.0);
        let slider_instance_data: Vec<SliderInstance> = Vec::with_capacity(10);

        let slider_vertex_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&slider_verticies),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });

        let slider_instance_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("linear instance buffer"),
                    contents: bytemuck::cast_slice(&slider_instance_data),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });

        let slider_index_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_index_buffer"),
                    contents: bytemuck::cast_slice(&slider_indecies),
                    usage: BufferUsages::INDEX,
                });

        let slider_pipeline_layout =
            graphics
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("slider test pipeline Layout"),
                    bind_group_layouts: &[
                        &camera_bind_group_layout,
                        &slider_settings_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let slider_pipeline =
            graphics
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("slider test pipeline"),
                    layout: Some(&slider_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &slider_shader,
                        entry_point: "vs_main",
                        buffers: &[Vertex::desc(), SliderInstance::desc()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &slider_shader,
                        compilation_options: Default::default(),
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: surface_config.format,
                            blend: None,
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
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: DepthTexture::DEPTH_FORMAT,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less, // 1.
                        stencil: wgpu::StencilState::default(),     // 2.
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState {
                        ..Default::default()
                    },
                    multiview: None,
                });

        let slider_to_screen_verticies = Vertex::quad_positional(0.0, 0.0, 1.0, 1.0);

        let slider_to_screen_vertex_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&slider_to_screen_verticies),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });

        let slider_to_screen_bind_group_layout =
            graphics
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("slider to screen bind group layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Texture {
                                sample_type: TextureSampleType::Float { filterable: true },
                                view_dimension: TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let slider_to_screen_pipeline_layout =
            graphics
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("slider to screen pipeline Layout"),
                    bind_group_layouts: &[
                        //&Texture::default_bind_group_layout(&graphics, 1),
                        &slider_to_screen_bind_group_layout,
                        &camera_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        let slider_to_screen_render_pipeline =
            graphics
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("slider to screen render pipeline23"),
                    layout: Some(&slider_to_screen_pipeline_layout),
                    vertex: wgpu::VertexState {
                        compilation_options: Default::default(),
                        module: &slider_to_screen_shader,
                        entry_point: "vs_main",
                        buffers: &[Vertex::desc(), SliderInstance::desc()],
                    },
                    fragment: Some(wgpu::FragmentState {
                        compilation_options: Default::default(),
                        module: &slider_to_screen_shader,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: surface_config.format,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::SrcAlpha,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent::REPLACE,
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
                    depth_stencil: all_depth,
                    multisample: wgpu::MultisampleState {
                        ..Default::default()
                    },
                    multiview: None,
                });

        let slider_to_screen_instance_data = Vec::with_capacity(10);

        let slider_to_screen_instance_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("slider to screen instance buffer"),
                    contents: bytemuck::cast_slice(&slider_instance_data),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });

        let follow_points_instance_data = Vec::with_capacity(10);

        let follow_points_instance_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("slider to screen instance buffer"),
                    contents: bytemuck::cast_slice(&follow_points_instance_data),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });

        let scale =
            calc_playfield_scale_factor(graphics.size.width as f32, graphics.size.height as f32);

        let quad_debug = QuadRenderer::new(graphics.clone(), true);

        let quad_debug_instance_data: Vec<QuadInstance> = Vec::new();
        let quad_debug_instance_data2: Vec<QuadInstance> = Vec::new();
        let quad_debug_buffer = quad_debug.create_instance_buffer();
        let quad_debug_buffer2 = quad_debug.create_instance_buffer();

        quad_debug.resize_vertex_centered(20.0, 20.0);

        Self {
            quad_debug,
            graphics,
            scale,
            quad_verticies,
            camera,
            camera_bind_group,
            camera_buffer,
            approach_circle_pipeline,
            approach_circle_instance_buffer,
            approach_circle_instance_data,
            hit_circle_pipeline,
            hit_circle_vertex_buffer,
            hit_circle_index_buffer,
            hit_circle_instance_data,
            hit_circle_instance_buffer,
            depth_texture,
            slider_instance_buffer,
            slider_instance_data,
            slider_pipeline,
            slider_indecies: slider_indecies.into(),
            slider_vertex_buffer,
            slider_index_buffer,
            slider_verticies: slider_verticies.into(),
            slider_to_screen_verticies,
            slider_to_screen_vertex_buffer,
            slider_to_screen_render_pipeline,
            slider_to_screen_instance_buffer,
            slider_to_screen_instance_data,
            slider_to_screen_textures: SmallVec::new(),
            follow_points_instance_data,
            follow_points_instance_buffer,
            offsets: Vector2::new(0.0, 0.0),
            hit_circle_diameter: 1.0,
            quad_colored_pipeline,
            slider_settings_buffer,
            slider_settings_bind_group,
            quad_debug_instance_data,
            quad_debug_buffer,
            quad_debug_instance_data2,
            quad_debug_buffer2,
            judgements_queue: Vec::new(),
        }
    }

    pub fn prepare(
        &self,
        config: &Config,
    ) {
        self.graphics
            .queue
            .write_buffer(&self.slider_settings_buffer, 0, bytemuck::bytes_of(&config.slider));
    }

    pub fn prepare_judgements(&mut self, time: f64, queue: &[usize], objects: &[Object]) {
        for index in queue {
            let object = &objects[*index];

            match &object.kind {
                hit_objects::ObjectKind::Circle(circle) => {
                    if let Some(hit_result) = &circle.hit_result {
                        match hit_result {
                            hit_objects::HitResult::Hit { at, pos, result } => {

                                let alpha = 1.0 - calc_progress(time, *at, at + JUDGMENTS_FADEOUT_TIME);

                                let entry = JudgementsEntry{
                                    pos: Vector2::new(circle.pos.x as f64, circle.pos.y as f64),
                                    alpha: alpha as f32,
                                    result: *result
                                };

                                self.judgements_queue.push(entry);
                            },
                        }
                    }
                },
                hit_objects::ObjectKind::Slider(_) => todo!(),
            }
        };
    }
    
    // TODO split into separate functions to avoid endless nesting and general mess
    pub fn prepare_objects(
        &mut self,
        time: f64,
        preempt: f32,
        fadein: f32,
        queue: &[usize],
        objects: &[Object],
        skin: &SkinManager,
    ) {
        let _span = tracy_client::span!("osu_renderer prepare_objects2");

        for current_index in queue.iter() {
            let object = &objects[*current_index];

            let color = skin.ini.colours.combo_colors.iter()
                .cycle()
                .skip(object.color)
                .next()
                .unwrap();

            match &object.kind {
                hit_objects::ObjectKind::Circle(circle) => {
                    // Approach should scale-in exactly at hit time
                    // But hit-circle should stay until:
                    // 1. HitResult is present => TODO
                    // 2. If no HitResult is present => stay until end time of late x50 hitwindow is reached

                    let _span = tracy_client::span!("osu_renderer prepare_objects2::circle");
                    let start_time = object.start_time - preempt as f64;
                    let fade_in_end_time = start_time + fadein as f64;

                    let alpha = calc_progress(time, start_time, fade_in_end_time).clamp(0.0, 1.0);

                    let approach_progress = calc_progress(time, start_time, circle.start_time);

                    let approach_scale = lerp(1.0, 4.0, 1.0 - approach_progress).clamp(1.0, 4.0);

                    let mut hit_circle_alpha = alpha;
                    let mut hit_circle_scale = 1.0;
                    let mut render_approach = true;

                    if let Some(hit_result) = &circle.hit_result {
                        self.quad_debug_instance_data.push(
                            QuadInstance::from_xy_pos(circle.pos.x, circle.pos.y)
                        );

                        match hit_result {
                            hit_objects::HitResult::Hit { at, result, .. } => {
                                // Hit appears early than the exact hit point is reached
                                // Apply fadeout immediatly
                                let progress = calc_progress(time, *at, *at + (CIRCLE_FADEOUT_TIME * 2.0));
                                hit_circle_alpha = 1.0 - progress;
                                hit_circle_scale = lerp(1.0, CIRCLE_SCALEOUT_MAX, progress);
                                render_approach = false;

                            },
                        }
                    } else {
                        // In case if there are no hit result keep alpha at 1.0 until late x50 hit window point
                        // is passed

                        if time >= object.start_time {
                            hit_circle_alpha = 1.0;
                        }
                    }

                    if time >= object.start_time {
                        render_approach = false;
                    }
                    
                    if render_approach {
                        self.approach_circle_instance_data
                            .push(ApproachCircleInstance::new(
                                circle.pos.x,
                                circle.pos.y,
                                0.0,
                                alpha as f32,
                                approach_scale as f32,
                            ));
                    }

                    let hit_circle_instance = HitCircleInstance::new(
                        circle.pos.x,
                        circle.pos.y,
                        0.0,
                        hit_circle_alpha as f32,
                        hit_circle_scale as f32,
                        color,
                    );

                    self.hit_circle_instance_data.push(hit_circle_instance);
                }
                hit_objects::ObjectKind::Slider(slider) => {
                    let _span = tracy_client::span!("osu_renderer prepare_objects2::circle");

                    let start_time = slider.start_time - preempt as f64;
                    let end_time = start_time + fadein as f64;

                    let mut body_alpha =
                    ((time - start_time) / (end_time - start_time)).clamp(0.0, 0.95);

                    // FADEOUT
                    if time >= object.start_time + slider.duration
                        && time <= object.start_time + slider.duration + SLIDER_FADEOUT_TIME
                    {
                        let start = object.start_time + slider.duration;
                        let end = object.start_time + slider.duration + SLIDER_FADEOUT_TIME;

                        let min = start.min(end);
                        let max = start.max(end);

                        let percentage = 100.0 - (((time - min) * 100.0) / (max - min)); // TODO remove `* 100.0`

                        body_alpha = (percentage / 100.0).clamp(0.0, 0.95);
                    }

                    // APPROACH
                    let approach_progress = (time - start_time) / (object.start_time - start_time);

                    let approach_scale = lerp(1.0, 3.95, 1.0 - approach_progress).clamp(1.0, 4.0);

                    let approach_alpha = if time >= object.start_time {
                        0.0
                    } else {
                        body_alpha
                    };

                    // FOLLOW CIRCLE STUFF
                    // BLOCK IN WHICH SLIDER IS HITABLE
                    let mut follow_circle = None;
                    if time >= object.start_time && time <= object.start_time + slider.duration {
                        // Calculating current slide
                        let v1 = time - object.start_time;
                        let v2 = slider.duration / slider.repeats as f64;
                        let slide = (v1 / v2).floor() as i32 + 1;

                        let slide_start = object.start_time + (v2 * (slide as f64 - 1.0));

                        let start = slide_start;
                        let current = time;
                        let end = slide_start + v2;

                        let min = start.min(end);
                        let max = start.max(end);

                        let mut percentage = ((current - min) * 100.0) / (max - min); // TODO remove `* 100.0`

                        // If slide is even we should go from 100% to 0%
                        // if not then from 0% to 100%
                        if slide % 2 == 0 {
                            percentage = 100.0 - percentage;
                        }

                        let pos = slider.curve.position_at(percentage / 100.0);

                        self.follow_points_instance_data.push(HitCircleInstance {
                            pos: [pos.x + slider.pos.x, pos.y + slider.pos.y, 0.0],
                            alpha: body_alpha as f32,
                            color: color.to_gpu_values(),
                            scale: 1.0
                        });

                        follow_circle = Some(self.follow_points_instance_data.len() as u32);
                    }

                    // BODY
                    self.slider_to_screen_instance_data.push(SliderInstance {
                        pos: [0.0, 0.0, 0.0],
                        alpha: body_alpha as f32,
                        slider_border: skin.ini.colours.slider_border.to_gpu_values(),
                        slider_body: skin.ini.colours.slider_body.to_gpu_values(),
                    });


                    self.approach_circle_instance_data
                        .push(ApproachCircleInstance::new(
                            slider.pos.x,
                            slider.pos.y,
                            0.0,
                            approach_alpha as f32,
                            approach_scale as f32,
                        ));

                    self.hit_circle_instance_data
                        .push(HitCircleInstance::new(
                                slider.pos.x,
                                slider.pos.y,
                                0.0,
                                if approach_alpha > 0.0 { body_alpha as f32 } else { 0.0 }, // TODO XD
                                1.0,
                                color,
                        ));

                    // That's tricky part. Since every slider have a according
                    // slider texture and a quad where texture will be rendered and presented on screen.
                    // So we are pushing all textures to the "queue" so we can iterate on it later
                    if let Some(render) = &slider.render {
                        self.slider_to_screen_textures.push((
                            render.texture.clone(),
                            render.quad.clone(),
                            follow_circle,
                        ))
                    } else {
                        panic!("Texture and quad should be present");
                    };
                }
            }
        }
    }

    pub fn get_graphics(&self) -> Arc<Graphics> {
        self.graphics.clone()
    }

    /// Render slider to the **texture** not screen
    pub fn prepare_and_render_slider_texture(
        &mut self,
        slider: &mut crate::hit_objects::slider::Slider,
        skin: &SkinManager,
        config: &Config
    ) {
        let _span = tracy_client::span!("osu_renderer prepare_and_render_slider_texture");
        let surface_config = self.graphics.get_surface_config();

        if !slider.render.is_none() && config.store_slider_textures {
            return;
        }

        let bbox = slider.bounding_box(self.hit_circle_diameter / 2.0);

        let (slider_vertices, _) = Vertex::cone((self.hit_circle_diameter / 2.0) * SLIDER_SCALE);

        self.slider_verticies = slider_vertices.into();

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.slider_vertex_buffer,
            &self.slider_verticies,
            Vertex
        );

        let slider_bounding_box = bbox.clone();

        let bbox_width = bbox.width() * SLIDER_SCALE;
        let bbox_height = bbox.height() * SLIDER_SCALE;

        let depth_texture =
            DepthTexture::new(&self.graphics, bbox_width as u32, bbox_height as u32, 1);

        let ortho = Camera::ortho(0.0, bbox_width, bbox_height, 0.0);

        let camera_buffer =
            self.graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("uniform_buffer"),
                    contents: bytemuck::bytes_of(&ortho),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });

        let camera_bind_group_layout =
            self.graphics
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

        let camera_bind_group =
            self.graphics
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &camera_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_buffer.as_entire_binding(),
                    }],
                    label: Some("camera_bind_group"),
                });

        self.slider_instance_data.clear();

        let (slider_texture_width, slider_texture_height) = (bbox_width as u32, bbox_height as u32);

        let slider_texture_not_sampled = self.graphics.device.create_texture(&TextureDescriptor {
            label: Some("SLIDER RENDER TEXTURE"),
            size: Extent3d {
                width: slider_texture_width,
                height: slider_texture_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: surface_config.format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[surface_config.format],
        });

        // Preparing instances
        let curve = &slider.curve;
        let n_segments = curve.dist() / 2.5;
        let step_by = (100.0 / n_segments as f64) / 100.0;

        let mut start = 0.0;
        let mut end = 1.0;
        
        while start <= end {
            let p = curve.position_at(start);
            let x = 0.0 + ((p.x + slider.pos.x) - bbox.top_left.x);
            let y = 0.0 + ((p.y + slider.pos.y) - bbox.top_left.y);
            self.slider_instance_data.push(SliderInstance::new(
                x * SLIDER_SCALE,
                y * SLIDER_SCALE,
                0.0,
                1.0,
                &skin.ini.colours.slider_border,
                &skin.ini.colours.slider_body,
            ));

            let p = curve.position_at(end);
            let x = 0.0 + ((p.x + slider.pos.x) - bbox.top_left.x);
            let y = 0.0 + ((p.y + slider.pos.y) - bbox.top_left.y);
            self.slider_instance_data.push(SliderInstance::new(
                x * SLIDER_SCALE,
                y * SLIDER_SCALE,
                0.0,
                1.0,
                &skin.ini.colours.slider_border,
                &skin.ini.colours.slider_body,
            ));

            end -= step_by;
            start += step_by;
        }

        let mut origin = Vector2::new(slider.pos.x, slider.pos.y);
        origin.x = 0.0 + (origin.x - bbox.top_left.x);
        origin.y = 0.0 + (origin.y - bbox.top_left.y);

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.slider_instance_buffer,
            &self.slider_instance_data,
            SliderInstance
        );

        // Drawing to the texture
        let view = slider_texture_not_sampled.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.graphics
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("SLIDER TEXTURE ENCODER"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("slider render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.slider_pipeline);

            render_pass.set_bind_group(0, &camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.slider_settings_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.slider_vertex_buffer.slice(..));

            render_pass.set_vertex_buffer(1, self.slider_instance_buffer.slice(..));

            render_pass.set_index_buffer(
                self.slider_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            
            render_pass.draw_indexed(
                0..self.slider_indecies.len() as u32,
                0,
                0..self.slider_instance_data.len() as u32,
            );
        }

        self.graphics
            .queue
            .submit(std::iter::once(encoder.finish()));

        let slider_texture = Arc::new(Texture::from_texture(
            slider_texture_not_sampled,
            &self.graphics,
            slider_texture_width,
            slider_texture_height,
            1,
        ));

        // RENDERED SLIDER TEXTURE QUAD
        let verticies = Vertex::quad_origin(
            slider.pos.x - origin.x,
            slider.pos.y - origin.y,
            bbox_width / SLIDER_SCALE,
            bbox_height / SLIDER_SCALE,
        );

        let slider_quad =
            self.graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("another slider to screen verticies buffer"),
                    contents: bytemuck::cast_slice(&verticies),
                    usage: BufferUsages::VERTEX,
                });

        slider.render = Some(SliderRender {
            texture: slider_texture,
            quad: slider_quad.into(),
            bounding_box: slider_bounding_box,
        });
    }

    pub fn on_cs_change(&mut self, cs: f32) {
        let hit_circle_diameter = get_hitcircle_diameter(cs);

        self.hit_circle_diameter = hit_circle_diameter;

        self.quad_verticies = Vertex::quad_centered(hit_circle_diameter, hit_circle_diameter);

        self.hit_circle_vertex_buffer =
            self.graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&self.quad_verticies),
                    usage: BufferUsages::VERTEX,
                });

        // Slider
        let (slider_vertices, slider_index) = Vertex::cone(hit_circle_diameter / 2.0);

        self.slider_verticies = slider_vertices.into();

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.slider_vertex_buffer,
            &self.slider_verticies,
            Vertex
        );

        self.slider_indecies = slider_index.into();

        self.slider_index_buffer =
            self.graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&self.slider_indecies),
                    usage: BufferUsages::INDEX,
                });
    }

    pub fn on_resize(&mut self, new_size: &PhysicalSize<u32>) {
        self.graphics.resize(new_size);

        let (graphics_width, graphics_height) = self.graphics.get_surface_size();

        let (scale, offsets) = calc_playfield(new_size.width as f32, new_size.height as f32);

        self.scale = scale;
        self.offsets = offsets;

        self.camera.resize(new_size);
        self.camera.transform(self.scale, self.offsets);
        self.depth_texture = DepthTexture::new(
            &self.graphics,
            graphics_width,
            graphics_height,
            1,
        );


        self.graphics
            .queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&self.camera)); // TODO

        self.quad_debug.resize_camera(new_size);
        self.quad_debug.transform_camera(self.scale, self.offsets);

        // Slider to screen
        self.slider_to_screen_verticies = Vertex::quad_positional(
            0.0,
            0.0,
            graphics_width as f32,
            graphics_height as f32,
        );

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.slider_to_screen_vertex_buffer,
            &self.slider_to_screen_verticies,
            Vertex
        );
    }

    pub fn write_buffers(&mut self) {
        let _span = tracy_client::span!("osu_renderer write buffers");

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.hit_circle_instance_buffer,
            &self.hit_circle_instance_data,
            HitCircleInstance
        );

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.approach_circle_instance_buffer,
            &self.approach_circle_instance_data,
            ApproachCircleInstance
        );

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.slider_to_screen_instance_buffer,
            &self.slider_to_screen_instance_data,
            SliderInstance
        );

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.follow_points_instance_buffer,
            &self.follow_points_instance_data,
            SliderInstance
        );

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.quad_debug_buffer,
            &self.quad_debug_instance_data,
            QuadInstance
        );

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.quad_debug_buffer2,
            &self.quad_debug_instance_data2,
            QuadInstance
        );
    }

    /// Clears internal buffers
    pub fn clear_buffers(&mut self) {
        let _span = tracy_client::span!("osu_renderer clear_buffers");
        self.hit_circle_instance_data.clear();
        self.approach_circle_instance_data.clear();
        self.slider_to_screen_instance_data.clear();
        self.slider_to_screen_textures.clear();
        self.follow_points_instance_data.clear();
        self.quad_debug_instance_data.clear();
        self.quad_debug_instance_data2.clear();
        self.judgements_queue.clear();
        self.quad_debug.clear_atlas_buffers();
    }
    
    /// Responsible for managing and rendering judgments queue
    pub fn render_judgements(&mut self, atlas: &AtlasTexture, view: &TextureView) {
        for jdg in &self.judgements_queue {
            let image_index = match jdg.result {
                hit_objects::Hit::X300 => 0,
                hit_objects::Hit::X100 => 1,
                hit_objects::Hit::X50 => 2,
                hit_objects::Hit::MISS => 3,
            };

            self.quad_debug.add_atlas_quad(
                jdg.pos.x as f32, jdg.pos.y as f32,
                50.0, 50.0,
                image_index,
                jdg.alpha,
                &atlas
            )
        }

        self.quad_debug.render_atlas_test(view, atlas.bind_group());
    }

    /// Render all objects from internal buffers
    /// and clears used buffers afterwards
    pub fn render_objects(
        &mut self, 
        view: &TextureView,
        queue: &[usize],
        objects: &[Object],
        skin: &SkinManager,
    ) -> Result<(), wgpu::SurfaceError> {
        let _span = tracy_client::span!("osu_renderer render_objects");

        let mut encoder =
            self.graphics
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("HitObjects encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("slider render pass"),
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
            
            // Here we need to manually sync gpu buffers which is written after [`prepare_objects`] is done.
            // and visibility queue that's gets filled inside [`OsuState`] while preserving
            // hitobjects order
            let mut current_circle = 0;
            let mut current_slider = 0;

            for current_index in queue.iter() {
                let object = &objects[*current_index];

                match object.kind {
                    hit_objects::ObjectKind::Circle(_) => {
                        render_pass.set_pipeline(&self.quad_colored_pipeline);
                        
                        // hit circle itself
                        render_pass.set_bind_group(0, &skin.hit_circle.bind_group, &[]);
                        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
                        render_pass.set_vertex_buffer(0, self.hit_circle_vertex_buffer.slice(..));
                        render_pass.set_vertex_buffer(1, self.hit_circle_instance_buffer.slice(..));
                        render_pass.set_index_buffer(
                            self.hit_circle_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint16,
                        );
                        render_pass.draw_indexed(
                            0..QUAD_INDECIES.len() as u32,
                            0,
                            current_circle..current_circle + 1,
                        );

                        // overlay
                        render_pass.set_pipeline(&self.quad_colored_pipeline);
                        render_pass.set_bind_group(0, &skin.hit_circle_overlay.bind_group, &[]);
                        render_pass.set_vertex_buffer(0, self.hit_circle_vertex_buffer.slice(..));
                        render_pass.set_vertex_buffer(1, self.hit_circle_instance_buffer.slice(..));
                        render_pass.draw_indexed(
                            0..QUAD_INDECIES.len() as u32,
                            0,
                            current_circle..current_circle + 1,
                        );

                        current_circle += 1;
                    },
                    hit_objects::ObjectKind::Slider(_) => {
                        render_pass.set_pipeline(&self.slider_to_screen_render_pipeline);
                        render_pass.set_vertex_buffer(1, self.slider_to_screen_instance_buffer.slice(..));
                        render_pass.set_index_buffer(
                            self.hit_circle_index_buffer.slice(..), // DOCS
                            wgpu::IndexFormat::Uint16,
                        );

                        let instance = current_slider as u32..current_slider as u32 + 1;
                        let (texture, vertex_buffer, follow) = &self.slider_to_screen_textures[current_slider];

                        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
                        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

                        render_pass.set_bind_group(0, &texture.bind_group, &[]);

                        // First draw a slider body
                        render_pass.draw_indexed(0..QUAD_INDECIES.len() as u32, 0, instance.clone());

                        // follow circle
                        if let Some(follow) = follow {
                            render_pass.set_pipeline(&self.hit_circle_pipeline);
                            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
                            render_pass.set_bind_group(0, &skin.sliderb0.bind_group, &[]);
                            render_pass.set_vertex_buffer(0, self.hit_circle_vertex_buffer.slice(..));
                            render_pass.set_vertex_buffer(1, self.follow_points_instance_buffer.slice(..));
                            render_pass.set_index_buffer(
                                self.hit_circle_index_buffer.slice(..),
                                wgpu::IndexFormat::Uint16,
                            );
                            render_pass.draw_indexed(
                                0..QUAD_INDECIES.len() as u32,
                                0,
                                *follow - 1 as u32..*follow as u32,
                            );
                        }

                        // Hit circle on top of everything
                        render_pass.set_pipeline(&self.quad_colored_pipeline);
                        render_pass.set_bind_group(0, &skin.hit_circle.bind_group, &[]);
                        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);

                        render_pass.set_vertex_buffer(0, self.hit_circle_vertex_buffer.slice(..));

                        render_pass.set_vertex_buffer(1, self.hit_circle_instance_buffer.slice(..));

                        render_pass.set_index_buffer(
                            self.hit_circle_index_buffer.slice(..),
                            wgpu::IndexFormat::Uint16,
                        );

                        render_pass.draw_indexed(
                            0..QUAD_INDECIES.len() as u32,
                            0,
                            current_circle..current_circle + 1,
                        );

                        render_pass.set_pipeline(&self.hit_circle_pipeline);
                        render_pass.set_bind_group(0, &skin.hit_circle_overlay.bind_group, &[]);
                        render_pass.set_vertex_buffer(0, self.hit_circle_vertex_buffer.slice(..));
                        render_pass.set_vertex_buffer(1, self.hit_circle_instance_buffer.slice(..));
                        render_pass.draw_indexed(
                            0..QUAD_INDECIES.len() as u32,
                            0,
                            current_circle..current_circle + 1,
                        );

                        current_slider += 1;
                        current_circle += 1;
                    },
                }
            }

            // Approach circles should be always on top
            render_pass.set_pipeline(&self.approach_circle_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.hit_circle_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.approach_circle_instance_buffer.slice(..));
            render_pass.set_index_buffer(
                self.hit_circle_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );

            render_pass.draw_indexed(
                0..QUAD_INDECIES.len() as u32,
                0,
                0..self.approach_circle_instance_data.len() as u32,
            );
            
            /*
            self.quad_debug.render_on_view_instanced(
                &view,
                &skin.judgments.hit_100.bind_group,
                &self.quad_debug_buffer,
                self.quad_debug_instance_data.len() as u32
            );
            */
        }

        self.render_judgements(&skin.judgments_atlas, &view);

        let span = tracy_client::span!("osu_renderer render_objects::queue::submit");
        self.graphics
            .queue
            .submit([encoder.finish()]);
        drop(span);

        self.clear_buffers();

        Ok(())
    }
}
