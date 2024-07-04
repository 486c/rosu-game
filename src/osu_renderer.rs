use std::{num::NonZeroU32, sync::Arc};

use cgmath::{Matrix4, SquareMatrix, Vector2, Vector3};
use rosu_map::Beatmap;
use wgpu::{util::DeviceExt, BindGroup, BufferUsages, CommandEncoder, Extent3d, RenderPipeline, TextureDescriptor, TextureDimension, TextureUsages, TextureView};
use winit::dpi::PhysicalSize;

use crate::{camera::Camera, graphics::Graphics, hit_circle_instance::{ApproachCircleInstance, HitCircleInstance}, hitobjects::{self, Object, SLIDER_FADEOUT_TIME}, math::{self, lerp}, slider_instance::SliderInstance, texture::{DepthTexture, Texture}, vertex::Vertex};


const QUAD_INDECIES: &[u16] = &[0, 1, 2, 0, 2, 3];

const OSU_COORDS_WIDTH: f32 = 512.0;
const OSU_COORDS_HEIGHT: f32 = 384.0;

const OSU_PLAYFIELD_BORDER_TOP_PERCENT: f32 = 0.117;
const OSU_PLAYFIELD_BORDER_BOTTOM_PERCENT: f32 = 0.0834;

fn get_hitcircle_diameter(cs: f32) -> f32 {
	((1.0 - 0.7*(cs - 5.0) / 5.0) / 2.0) * 128.0 * 1.00041
}

fn calc_playfield_scale_factor(screen_w: f32, screen_h: f32) -> f32 {
    let top_border_size = OSU_PLAYFIELD_BORDER_TOP_PERCENT * screen_h;
    let bottom_border_size = OSU_PLAYFIELD_BORDER_BOTTOM_PERCENT * screen_h;
    
    let engine_screen_w = screen_w;
    let engine_screen_h = screen_h - bottom_border_size - top_border_size;

    let scale_factor = if screen_w / OSU_COORDS_WIDTH > engine_screen_h / OSU_COORDS_HEIGHT {
        engine_screen_h / OSU_COORDS_HEIGHT
    } else {
        engine_screen_w / OSU_COORDS_WIDTH
    };

    return scale_factor;
}

pub struct OsuRenderer {
    // Graphics State
    graphics: Graphics,

    // State
    scale: f32,
    offsets: Vector2::<f32>,
    hit_circle_diameter: f32,


    // Quad verticies
    quad_verticies: [Vertex; 4],


    // Camera
    camera: Camera,
    camera_bind_group: BindGroup,
    camera_buffer: wgpu::Buffer,

    // Approach circle
    approach_circle_pipeline: RenderPipeline,
    approach_circle_texture: Texture,
    approach_circle_instance_buffer: wgpu::Buffer,
    approach_circle_instance_data: Vec<ApproachCircleInstance>,

    // Hit Circle
    hit_circle_texture: Texture,
    hit_circle_pipeline: RenderPipeline,
    hit_circle_vertex_buffer: wgpu::Buffer,
    hit_circle_index_buffer: wgpu::Buffer,
    hit_circle_instance_data: Vec<HitCircleInstance>,
    hit_circle_instance_buffer: wgpu::Buffer,

    // Slider to texture
    slider_instance_buffer: wgpu::Buffer,
    slider_instance_data: Vec<SliderInstance>,
    slider_pipeline: RenderPipeline,
    slider_indecies: Vec<u16>,

    slider_vertex_buffer: wgpu::Buffer,
    slider_index_buffer: wgpu::Buffer,
    slider_verticies: Vec<Vertex>,

    // Slider texture to screen
    slider_to_screen_verticies: [Vertex; 4],
    slider_to_screen_vertex_buffer: wgpu::Buffer,
    slider_to_screen_render_pipeline: RenderPipeline,
    slider_to_screen_instance_buffer: wgpu::Buffer,
    slider_to_screen_instance_data: Vec<SliderInstance>,
    slider_to_screen_textures: Vec<(Arc<Texture>, Arc<wgpu::Buffer>)>,

    // Slider follow point
    follow_point_texture: Texture,
    follow_points_instance_data: Vec<HitCircleInstance>,
    follow_points_instance_buffer: wgpu::Buffer,


    depth_texture: DepthTexture,
}

impl OsuRenderer {
    pub fn new(graphics: Graphics) -> Self {
        let hit_circle_texture = Texture::from_path(
            "skin/hitcircle.png",
            &graphics
        );

        let approach_circle_texture = Texture::from_path(
            "skin/approachcircle.png",
            &graphics
        );

        let slider_control_point_texture = Texture::from_path(
            "skin/slider_control_point.png",
            &graphics
        );

        let follow_point_texture = Texture::from_path(
            "skin/sliderb0.png",
            &graphics
        );

        let hit_circle_shader = graphics.device.create_shader_module(
            wgpu::include_wgsl!("shaders/hit_circle.wgsl")
        );

        let approach_circle_shader = graphics.device.create_shader_module(
            wgpu::include_wgsl!("shaders/approach_circle.wgsl")
        );

        let slider_shader = graphics.device.create_shader_module(
            wgpu::include_wgsl!("shaders/slider2.wgsl")
        );

        let slider_to_screen_shader = graphics.device.create_shader_module(
            wgpu::include_wgsl!("shaders/slider_to_screen.wgsl")
        );

        let depth_texture = DepthTexture::new(&graphics, graphics.config.width, graphics.config.height);

        let quad_verticies = Vertex::quad_centered(1.0, 1.0);

        let hit_circle_vertex_buffer = graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&quad_verticies),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
            );

        let hit_circle_index_buffer = graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_index_buffer"),
                    contents: bytemuck::cast_slice(QUAD_INDECIES),
                    usage: BufferUsages::INDEX,
                }
            );

        let hit_circle_instance_data: Vec<HitCircleInstance> = Vec::with_capacity(10);

        let hit_circle_instance_buffer = graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Hit Instance Buffer"),
                    contents: bytemuck::cast_slice(
                        &hit_circle_instance_data
                    ),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
            );
        
        let approach_circle_instance_data: Vec<ApproachCircleInstance> =
            Vec::with_capacity(10);

        let approach_circle_instance_buffer = graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Hit Instance Buffer"),
                    contents: bytemuck::cast_slice(
                        &approach_circle_instance_data
                    ),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
            );
        
        /* Camera stuff */
        let camera = Camera::new(
            graphics.config.width as f32, 
            graphics.config.height as f32,
            1.0,
        );

        let camera_buffer = graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("uniform_buffer"),
                    contents: bytemuck::bytes_of(&camera),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                }
            );


        let camera_bind_group_layout = graphics.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        let camera_bind_group = graphics.device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &camera_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_buffer.as_entire_binding(),
                    }
                ],
                label: Some("camera_bind_group"),
        });
        
        let approach_circle_pipeline_layout = graphics.device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("approach circle pipeline Layout"),
                    bind_group_layouts: &[
                        &approach_circle_texture.bind_group_layout,
                        //&approach_circle_texture.bind_group_layout,
                        &camera_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                }
        );

        let approach_circle_pipeline = graphics.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("approach circle render pipeline"),
                layout: Some(&approach_circle_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &approach_circle_shader,
                    entry_point: "vs_main",
                    buffers: &[
                        Vertex::desc(), 
                        ApproachCircleInstance::desc(),
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &approach_circle_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: graphics.config.format,
                        blend: Some(wgpu::BlendState{
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent::OVER,
                        }
                        ),
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
                multiview: None,//Some(NonZeroU32::new(4).unwrap()),
            }
        );

        let hit_circle_pipeline_layout = graphics.device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("hitcircle pipeline Layout"),
                    bind_group_layouts: &[
                        &hit_circle_texture.bind_group_layout,
                        //&approach_circle_texture.bind_group_layout,
                        &camera_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                }
            );

        let hit_circle_pipeline = graphics.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("hit_circle render pipeline"),
                layout: Some(&hit_circle_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &hit_circle_shader,
                    entry_point: "vs_main",
                    buffers: &[
                        Vertex::desc(), 
                        HitCircleInstance::desc(),
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &hit_circle_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: graphics.config.format,
                        blend: Some(wgpu::BlendState{
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
            }
        );

        let (slider_verticies, slider_indecies) = Vertex::cone(5.0);
        let slider_instance_data: Vec<SliderInstance> = Vec::with_capacity(10);

        let slider_vertex_buffer = graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&slider_verticies),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
            );

        let slider_instance_buffer = graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("linear instance buffer"),
                    contents: bytemuck::cast_slice(&slider_instance_data),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
            );

        let slider_index_buffer = graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_index_buffer"),
                    contents: bytemuck::cast_slice(&slider_indecies),
                    usage: BufferUsages::INDEX,
                }
            );

        let slider_pipeline_layout = graphics.device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("slider test pipeline Layout"),
                    bind_group_layouts: &[
                        //&approach_circle_texture.bind_group_layout,
                        &camera_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                }
            );

        let slider_pipeline = graphics.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("slider test pipeline"),
                layout: Some(&slider_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &slider_shader,
                    entry_point: "vs_main",
                    buffers: &[
                        Vertex::desc(), 
                        SliderInstance::desc(),
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &slider_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: graphics.config.format,
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
                    stencil: wgpu::StencilState::default(), // 2.
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            }
        );

        let slider_to_screen_verticies = Vertex::quad_positional(0.0, 0.0, 1.0, 1.0);

        let slider_to_screen_vertex_buffer = graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&slider_to_screen_verticies),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
            );

        let slider_to_screen_pipeline_layout = graphics.device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("slider to screen pipeline Layout"),
                    bind_group_layouts: &[
                        &Texture::default_bind_group_layout(&graphics),
                        &camera_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                }
        );

        let slider_to_screen_render_pipeline = graphics.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("slider to screen render pipeline"),
                layout: Some(&slider_to_screen_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &slider_to_screen_shader,
                    entry_point: "vs_main",
                    buffers: &[
                        Vertex::desc(), 
                        SliderInstance::desc(),
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &slider_to_screen_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: graphics.config.format,
                        blend: Some(wgpu::BlendState{
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
            }
        );

        let slider_to_screen_instance_data = Vec::with_capacity(10);

        let slider_to_screen_instance_buffer = graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("slider to screen instance buffer"),
                    contents: bytemuck::cast_slice(&slider_instance_data),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
            );

        let follow_points_instance_data = Vec::with_capacity(10);

        let follow_points_instance_buffer = graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("slider to screen instance buffer"),
                    contents: bytemuck::cast_slice(&follow_points_instance_data),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
            );

        let scale = calc_playfield_scale_factor(
            graphics.size.width as f32,
            graphics.size.height as f32
        );

        Self {
            graphics,
            scale,
            quad_verticies,
            camera,
            camera_bind_group,
            camera_buffer,
            approach_circle_pipeline,
            approach_circle_texture,
            approach_circle_instance_buffer,
            approach_circle_instance_data,
            hit_circle_texture,
            hit_circle_pipeline,
            hit_circle_vertex_buffer,
            hit_circle_index_buffer,
            hit_circle_instance_data,
            hit_circle_instance_buffer,
            depth_texture,
            slider_instance_buffer,
            slider_instance_data,
            slider_pipeline,
            slider_indecies,
            slider_vertex_buffer,
            slider_index_buffer,
            slider_verticies,
            slider_to_screen_verticies,
            slider_to_screen_vertex_buffer,
            slider_to_screen_render_pipeline,
            slider_to_screen_instance_buffer,
            slider_to_screen_instance_data,
            slider_to_screen_textures: Vec::with_capacity(10),
            follow_point_texture,
            follow_points_instance_data,
            follow_points_instance_buffer,
            offsets: Vector2::new(0.0, 0.0),
            hit_circle_diameter: 1.0,
        }
    }

    pub fn get_graphics(&self) -> &Graphics {
        &self.graphics
    }
    
    /// Render slider to the **texture** not screen
    pub fn prepare_and_render_slider_texture(&mut self, slider: &mut hitobjects::Slider) {
        let _span = tracy_client::span!("osu_renderer prepare_and_render_slider_texture");
        if !slider.bounding_box.is_none() 
        && !slider.texture.is_none() {
            return
        }
        
        let bbox = slider.bounding_box(self.hit_circle_diameter / 2.0);
        slider.bounding_box = Some(bbox.clone());
        
        let bbox_width = bbox.width();
        let bbox_height = bbox.height();

        //width += 300.0;
        //height += 300.0;

        let depth_texture = DepthTexture::new(&self.graphics, bbox_width as u32, bbox_height as u32);

        //let mut ortho = Camera::ortho(top_left.x, width, height, top_left.y);
        // glOrtho(center_x - width / 2, center_x + width / 2, center_y - height / 2, center_y + height / 2, -1, 1);
        //glOrtho(top_left_x, top_right_x, bottom_left_y, top_left_y, -1, 1);
        //let mut ortho = Camera::ortho(center_x - width / 2.0, center_x + width / 2.0, center_y - height / 2.0, center_y + height / 2.0);
        let ortho = Camera::ortho(0.0, bbox_width, bbox_height, 0.0);
        
        // gluLookAt(center_x, center_y, 0, center_x, center_y, -1, 0, 1, 0);
        //ortho.scale(self.scale);
        
        /*
        ortho.view = Matrix4::look_at_rh(
            [center_x, center_y, 0.0].into(),
            [center_x, center_y, -1.0].into(),
            [0.0, 1.0, 0.0].into()
        );
        */

        //ortho.view = self.camera.view;

        let camera_buffer = self.graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("uniform_buffer"),
                    contents: bytemuck::bytes_of(&ortho),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                }
            );

        let camera_bind_group_layout = self.graphics.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });
        
        let camera_bind_group = self.graphics.device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &camera_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_buffer.as_entire_binding(),
                    }
                ],
                label: Some("camera_bind_group"),
        });

        self.slider_instance_data.clear();

        let slider_texture = self.graphics.device.create_texture(
            &TextureDescriptor {
                label: Some("SLIDER RENDER TEXTURE"),
                size: Extent3d {
                    width: bbox_width as u32,
                    height: bbox_height as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: self.graphics.config.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[self.graphics.config.format],
            });


        // Preparing instances
        let curve = &slider.curve;
        let n_segments = curve.dist() / 2.5;
        let step_by = (100.0 / n_segments as f64) / 100.0;

        let mut step = 0.0;

        while step <= 1.0 {
            let p = curve.position_at(step);

            // translating a bounding box coordinates to our coordinates that starts at (0,0)
            let x = p.x + slider.pos.x;
            let x = 0.0 + (x - bbox.top_left.x) * 1.0;

            let y = p.y + slider.pos.y;
            let y = 0.0 + (y - bbox.top_left.y) * 1.0;

            self.slider_instance_data.push(
                SliderInstance::new(
                    x, y,
                    1.0
                )
            );

            step += step_by;
        }

        let mut origin = Vector2::new(slider.pos.x, slider.pos.y);
        origin.x = 0.0 + (origin.x - bbox.top_left.x) * 1.0;
        origin.y = 0.0 + (origin.y - bbox.top_left.y) * 1.0;


        self.slider_instance_buffer = self.graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("linear vertex_buffer"),
                    contents: bytemuck::cast_slice(&self.slider_instance_data),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
            );

        // Drawing to the texture
        let view = slider_texture.create_view(
            &wgpu::TextureViewDescriptor::default()
        );

        let mut encoder = self.graphics.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("SLIDER TEXTURE ENCODER"),
        });
        
        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                label: Some("slider render pass"),
                color_attachments: 
                    &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,/*wgpu::LoadOp::Clear(
                                wgpu::Color {
                                    r: 0.0,
                                    g: 0.0,
                                    b: 0.0,
                                    a: 1.0,
                                })*/
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(
                    wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_texture.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }
                ),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.slider_pipeline);

            render_pass.set_bind_group(0, &camera_bind_group, &[]);

            render_pass.set_vertex_buffer(
                0, self.slider_vertex_buffer.slice(..)
            );

            render_pass.set_vertex_buffer(
                1, self.slider_instance_buffer.slice(..)
            );
            
            render_pass.set_index_buffer(
                self.slider_index_buffer.slice(..), 
                wgpu::IndexFormat::Uint16
            );
            
            render_pass.draw_indexed(
                0..self.slider_indecies.len() as u32,
                0,
                0..self.slider_instance_data.len() as u32,
            );
        }


        self.graphics.queue.submit(std::iter::once(encoder.finish()));

        slider.texture = Some(Arc::new(Texture::from_texture(slider_texture, &self.graphics)));

        // RENDERED SLIDER TEXTURE QUAD
        let verticies = Vertex::quad_origin(
            slider.pos.x - origin.x, 
            slider.pos.y - origin.y,
            bbox_width,
            bbox_height,
        );

        let buffer = self.graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("another slider to screen verticies buffer"),
                    contents: bytemuck::cast_slice(&verticies),
                    usage: BufferUsages::VERTEX,
                }
            );

        slider.quad = Some(buffer.into());

    }

    pub fn on_cs_change(&mut self, cs: f32) {
        println!("OsuRenderer -> on_cs_change()");
        let hit_circle_diameter = get_hitcircle_diameter(cs);

        self.hit_circle_diameter = hit_circle_diameter;

        self.quad_verticies = Vertex::quad_centered(hit_circle_diameter, hit_circle_diameter);

        self.hit_circle_vertex_buffer = self.graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&self.quad_verticies),
                    usage: BufferUsages::VERTEX,
                }
            );

        // Slider
        let (slider_vertices, slider_index) = Vertex::cone55(hit_circle_diameter / 2.0);

        self.slider_verticies = slider_vertices;

        self.slider_vertex_buffer = self.graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&self.slider_verticies),
                    usage: BufferUsages::VERTEX,
                }
            );

        self.slider_indecies = slider_index;
        

        self.slider_index_buffer = self.graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&self.slider_indecies),
                    usage: BufferUsages::INDEX,
                }
            );
    }

    pub fn on_resize(&mut self, new_size: &PhysicalSize<u32>) {
        self.graphics.resize(new_size);

        // Calculate playfield scale
        self.scale = calc_playfield_scale_factor(
            new_size.width as f32,
            new_size.height as f32
        );

        
        // Calculate playfield offsets
        let scaled_height = OSU_COORDS_HEIGHT as f32 * self.scale;
        let scaled_width = OSU_COORDS_WIDTH as f32 * self.scale;

        let bottom_border_size = 
            OSU_PLAYFIELD_BORDER_BOTTOM_PERCENT * new_size.height as f32;

        let y_offset = (new_size.height as f32 - scaled_height) / 2.0 
            + (new_size.height as f32 / 2.0 - (scaled_height / 2.0) - bottom_border_size);

        let x_offset = (new_size.width as f32 - scaled_width) / 2.0;

        let offsets = Vector2::new(x_offset, y_offset);
        self.offsets = offsets;

        self.camera.resize(new_size);
        self.camera.transform(self.scale, offsets);
        self.depth_texture = DepthTexture::new(
            &self.graphics,
            self.graphics.config.width,
            self.graphics.config.height,
        );
        
        // TODO Recreate buffers
        self.graphics.queue.write_buffer(
            &self.camera_buffer, 
            0, 
            bytemuck::bytes_of(&self.camera)
        ); // TODO

        // Slider to screen

        
        self.slider_to_screen_verticies = Vertex::quad_positional(
            0.0, 0.0,
            self.graphics.config.width as f32,
            self.graphics.config.height as f32,
        );


        self.slider_to_screen_vertex_buffer = self.graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(&self.slider_to_screen_verticies),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
            );
    }

    pub fn write_buffers(&mut self) {
        let _span = tracy_client::span!("osu_renderer write buffers");

        self.hit_circle_instance_buffer = self.graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Hit Instance Buffer"),
                    contents: bytemuck::cast_slice(
                        &self.hit_circle_instance_data
                    ),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
            );

        self.approach_circle_instance_buffer = self.graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Approach Instance Buffer"),
                    contents: bytemuck::cast_slice(
                        &self.approach_circle_instance_data
                    ),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
            );

        self.slider_to_screen_instance_buffer = self.graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Approach Instance Buffer"),
                    contents: bytemuck::cast_slice(
                        &self.slider_to_screen_instance_data
                    ),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
            );


        self.follow_points_instance_buffer = self.graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Instance Buffer"),
                    contents: bytemuck::cast_slice(
                        &self.follow_points_instance_data,
                    ),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                }
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
    }
    
    /// Prepares object for render
    /// HitCircle:
    ///     1. Prepare hit & approach circles instances
    /// Slider:
    ///
    pub fn prepare_object_for_render(
        &mut self, 
        obj: &Object,
        time: f64,
        preempt: f32,
        fadein: f32,
    ) {
        match &obj.kind {
            hitobjects::ObjectKind::Circle(circle) => {
                let _span = tracy_client::span!("osu_renderer prepare_object_for_render::circle");

                let start_time = 
                    obj.start_time - preempt as f64;
                let end_time = 
                    start_time + fadein as f64;
                let alpha = 
                    ((time-start_time)/(end_time-start_time))
                    .clamp(0.0, 1.0);

                let approach_progress = 
                    (time-start_time)/(obj.start_time-start_time); 

                let approach_scale = lerp(1.0, 4.0, 1.0 - approach_progress)
                    .clamp(1.0, 4.0);

                self.hit_circle_instance_data.push(
                    HitCircleInstance::new(
                        circle.pos.x,
                        circle.pos.y,
                        alpha as f32
                    )
                );

                self.approach_circle_instance_data.push(
                    ApproachCircleInstance::new(
                        circle.pos.x,
                        circle.pos.y,
                        alpha as f32,
                        approach_scale as f32
                    )
                );
            },
            hitobjects::ObjectKind::Slider(slider) => {
                let _span = tracy_client::span!("osu_renderer prepare_object_for_render::slider");

                let start_time = obj.start_time - preempt as f64;
                let end_time = start_time + fadein as f64;

                let mut alpha = 
                    ((time-start_time)/(end_time-start_time))
                    .clamp(0.0, 0.95);

                // FADEOUT
                if time >= obj.start_time + slider.duration 
                && time <= obj.start_time + slider.duration + SLIDER_FADEOUT_TIME {
                    let start = obj.start_time + slider.duration;
                    let end = obj.start_time + slider.duration + SLIDER_FADEOUT_TIME;

                    let min = start.min(end);
                    let max = start.max(end);

                    let percentage = 100.0 - (((time - min) * 100.0) / (max - min)); // TODO remove `* 100.0`

                    alpha = (percentage / 100.0).clamp(0.0, 0.95);
                }

                // BODY
                self.slider_to_screen_instance_data.push(
                    SliderInstance {
                        //pos: [x as f32, y as f32],
                        pos: [0.0, 0.0],
                        alpha: alpha as f32,
                });
                
                // APPROACH
                let approach_progress = 
                    (time-start_time)/(obj.start_time-start_time); 

                let approach_scale = lerp(1.0, 3.95, 1.0 - approach_progress)
                    .clamp(1.0, 4.0);

                let approach_alpha = if time >= obj.start_time {
                    0.0
                } else {
                    alpha
                };

                // FOLLOW POINTS STUFF
                // BLOCK IN WHICH SLIDER IS HITABLE
                if time >= obj.start_time && time <= obj.start_time + slider.duration {
                    // Calculating current slide
                    let v1 = time - obj.start_time;
                    let v2 = slider.duration / slider.repeats as f64;
                    let slide = (v1 / v2).floor() as i32 + 1;


                    let slide_start = obj.start_time + (v2 * (slide as f64 - 1.0));


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

                    self.follow_points_instance_data.push(
                        HitCircleInstance {
                            pos: [pos.x + slider.pos.x, pos.y + slider.pos.y],
                            alpha: 1.0,
                        }
                    )
                }

                let start_pos = slider.pos + slider.curve.position_at(0.0);

                assert!(slider.bounding_box.is_none() != true);

                if let Some(bbox) = &slider.bounding_box {
                    let x_top_left = slider.pos.x - (bbox.width() / 2.0);
                    let y_top_left = slider.pos.y + (bbox.height() / 2.0);
                    
                    self.approach_circle_instance_data.push(
                        ApproachCircleInstance::new(
                            slider.pos.x,
                            slider.pos.y,
                            //x_top_left + self.offsets.x,
                            //y_top_left + self.offsets.y,
                            approach_alpha as f32,
                            approach_scale as f32
                        )
                    );
                }

                if let (Some(texture), Some(quad)) = (&slider.texture, &slider.quad) {
                    self.slider_to_screen_textures.push(
                        (texture.clone(), quad.clone()), // TODO
                    )
                } else {
                    panic!("Texture and quad should be present");
                }

            },
        }
    }

    pub fn render_sliders(&mut self, view: &TextureView) -> Result<CommandEncoder, wgpu::SurfaceError> {
        let _span = tracy_client::span!("osu_renderer render_sliders");

        let mut encoder = self.graphics.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("sliders & followpoints encoder"),
        });
    
        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                label: Some("slider render pass"),
                color_attachments: 
                    &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(
                                wgpu::Color {
                                    r: 0.0,
                                    g: 0.0,
                                    b: 0.0,
                                    a: 0.0,
                                }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.slider_to_screen_render_pipeline);


            render_pass.set_vertex_buffer(
                1, self.slider_to_screen_instance_buffer.slice(..)
            );

            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);

            render_pass.set_index_buffer(
                self.hit_circle_index_buffer.slice(..),  // DOCS
                wgpu::IndexFormat::Uint16
            );
            
            // Sanity check
            assert_eq!(
                self.slider_to_screen_instance_data.len(),
                self.slider_to_screen_textures.len()
            );

            for (i, (texture, vertex_buffer)) in self.slider_to_screen_textures.iter().enumerate() {
                let instance = i as u32..i as u32 +1;

                render_pass.set_vertex_buffer(
                    0, vertex_buffer.slice(..)
                );

                render_pass.set_bind_group(
                    0, 
                    &texture.bind_group, 
                    &[]
                );

                render_pass.draw_indexed(
                    0..QUAD_INDECIES.len() as u32,
                    0,
                    instance,
                );
            }

            // Slider follow points
            render_pass.set_pipeline(&self.hit_circle_pipeline);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(
                0, 
                &self.follow_point_texture.bind_group, 
                &[]
            );

            render_pass.set_vertex_buffer(
                0, self.hit_circle_vertex_buffer.slice(..)
            );
            render_pass.set_vertex_buffer(
                1, self.follow_points_instance_buffer.slice(..)
            );
            render_pass.set_index_buffer(
                self.hit_circle_index_buffer.slice(..), 
                wgpu::IndexFormat::Uint16
            );

            render_pass.draw_indexed(
                0..QUAD_INDECIES.len() as u32,
                0,
                0..self.follow_points_instance_data.len() as u32,
            );

        }

        Ok(encoder)
    }

    pub fn render_hitcircles(&mut self, view: &TextureView) -> Result<CommandEncoder, wgpu::SurfaceError> {
        let _span = tracy_client::span!("osu_renderer render_hitcircles");
        let mut encoder = self.graphics.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("HitCircles encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("slider render pass"),
                    color_attachments: 
                        &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                /*
                                   wgpu::Color {
                                   r: 0.0,
                                   g: 0.0,
                                   b: 0.0,
                                   a: 0.0,
                                   }),
                                   */
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                });

            // Hit circles section
            render_pass.set_pipeline(&self.hit_circle_pipeline);
            render_pass.set_bind_group(
                0, 
                &self.hit_circle_texture.bind_group, 
                &[]
            );

            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);

            render_pass.set_vertex_buffer(
                0, self.hit_circle_vertex_buffer.slice(..)
            );

            render_pass.set_vertex_buffer(
                1, self.hit_circle_instance_buffer.slice(..)
            );

            render_pass.set_index_buffer(
                self.hit_circle_index_buffer.slice(..), 
                wgpu::IndexFormat::Uint16
            );

            render_pass.draw_indexed(
                0..QUAD_INDECIES.len() as u32,
                0,
                0..self.hit_circle_instance_data.len() as u32,
            );

            // Approach circles section
            render_pass.set_pipeline(&self.approach_circle_pipeline);
            render_pass.set_bind_group( // TODO ???
                0, 
                &self.approach_circle_texture.bind_group, 
                &[]
            );

            render_pass.set_bind_group(
                1, 
                &self.approach_circle_texture.bind_group, 
                &[]
            );

            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);

            render_pass.set_vertex_buffer(
                0, self.hit_circle_vertex_buffer.slice(..)
            );

            render_pass.set_vertex_buffer(
                1, self.approach_circle_instance_buffer.slice(..)
            );

            render_pass.set_index_buffer(
                self.hit_circle_index_buffer.slice(..), 
                wgpu::IndexFormat::Uint16
            );

            render_pass.draw_indexed(
                0..QUAD_INDECIES.len() as u32,
                0,
                0..self.approach_circle_instance_data.len() as u32,
            );
        }

        Ok(encoder)
    }
    
    /// Render all objects from internal buffers 
    /// and clears used buffers afterwards
    pub fn render_objects(&mut self, view: &TextureView) -> Result<(), wgpu::SurfaceError> {
        let _span = tracy_client::span!("osu_renderer render_objects");

        let hitcircles_encoder = self.render_hitcircles(&view)?;
        let sliders_encoder = self.render_sliders(&view)?;

        //println!("Sliders to render: {}", self.slider_to_screen_instance_data.len());
        

        let span = tracy_client::span!("osu_renderer render_objects::queue::submit");
        self.graphics.queue.submit(
            [sliders_encoder.finish(), hitcircles_encoder.finish()]
        );
        drop(span);

        self.clear_buffers();

        Ok(())
    }
}
