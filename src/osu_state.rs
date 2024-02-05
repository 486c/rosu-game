use std::path::Path;

use cgmath::{Vector2, Matrix4, Vector3};
use egui::{Slider, style::HandleShape};
use rosu_pp::{Beatmap, parse::HitObjectKind};
use wgpu::{ShaderStages, BindingType, TextureSampleType, TextureViewDimension, RenderPipeline, BindGroup, BufferUsages, util::DeviceExt};
use winit::{window::Window, dpi::PhysicalSize};

use crate::{graphics::Graphics, egui_state::EguiState, texture::Texture, vertex::Vertex, camera::Camera, hit_circle_instance::HitCircleInstance, timer::Timer};

static OSU_COORDS_WIDTH: f32 = 512.0;
static OSU_COORDS_HEIGHT: f32 = 384.0;

const VERTICES: &[Vertex] = &[
    Vertex {pos: [0.0, 0.0], uv:[0.0, 0.0]},
    Vertex {pos: [0.0, 20.0], uv:[0.0, 1.0]},
    Vertex {pos: [20.0, 20.0], uv:[1.0, 1.0]},
    Vertex {pos: [20.0, 0.0], uv:[1.0, 0.0]},
    //Vertex {pos: [-1.0, 1.0], uv: [0.0, 0.0]},
    //Vertex {pos: [-1.0, 0.0], uv: [0.0, 1.0]},
    //Vertex {pos: [0.0, 0.0], uv: [1.0, 1.0]},
    //Vertex {pos: [0.0, 1.0], uv: [1.0, 0.0]}, 
];


//const INDECIES: &[u16] = &[0, 2, 3, 0, 1, 2];
const INDECIES: &[u16] = &[0, 1, 2, 0, 2, 3];

pub struct OsuState {
    pub window: Window,
    pub state: Graphics,
    pub egui: EguiState,

    current_beatmap: Option<Beatmap>,

    osu_clock: Timer,

    hit_circle_bind_group: BindGroup,
    hit_circle_texture: Texture,
    hit_circle_pipeline: RenderPipeline,
    hit_circle_vertex_buffer: wgpu::Buffer,
    hit_circle_index_buffer: wgpu::Buffer,

    hit_circle_instance_data: Vec<HitCircleInstance>,
    hit_circle_instance_buffer: wgpu::Buffer,

    osu_camera: Camera,
    camera_bind_group: BindGroup,
    camera_buffer: wgpu::Buffer,

    // TODO remove
    scale: f32,
    playfield_scale: (f32, f32),
    playfield_offsets: (f32, f32)
}

impl OsuState {
    pub fn new(
        window: Window,
        graphics: Graphics
    ) -> Self {

        let egui = EguiState::new(&graphics.device, &window);

        let hit_circle_texture = Texture::from_path(
            "skin/hitcircle.png",
            &graphics
        );

        let shader = graphics.device.create_shader_module(
            wgpu::include_wgsl!("shaders/hit_circle.wgsl")
        );


        let hit_circle_vertex_buffer = graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
                    contents: bytemuck::cast_slice(VERTICES),
                    usage: BufferUsages::VERTEX,
                }
            );

        let hit_circle_index_buffer = graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_index_buffer"),
                    contents: bytemuck::cast_slice(INDECIES),
                    usage: BufferUsages::INDEX,
                }
            );

        let hit_circle_instance_data: Vec<HitCircleInstance> = Vec::new();

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
        
        /* Camera stuff */
        let camera = Camera::new(
            graphics.config.width as f32, 
            graphics.config.height as f32, 
        );

        let camera_buffer = graphics.device
            .create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("uniform_buffer"),
                    contents: bytemuck::bytes_of(&camera.mat),
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

        let hit_circle_bind_group_layout = graphics.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("hitcircles bind"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float {filterable: true},
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler (wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });


        let hit_circle_bind_group = graphics.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &hit_circle_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&hit_circle_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&hit_circle_texture.sampler),
                    }
                ],
                label: Some("hit_circle_bind"),
            }
        );

        let hit_circle_pipeline_layout = graphics.device
            .create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[
                        &hit_circle_bind_group_layout,
                        &camera_bind_group_layout
                    ],
                    push_constant_ranges: &[],
                }
            );

        let hit_circle_pipeline = graphics.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("hit_circle render pipeline"),
                layout: Some(&hit_circle_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[
                        Vertex::desc(), 
                        HitCircleInstance::desc(),
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
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
                multiview: None,
            }
        );

        
        Self {
            window,
            current_beatmap: None,
            egui,
            state: graphics,
            osu_clock: Timer::new(),
            hit_circle_texture,
            hit_circle_pipeline,
            hit_circle_bind_group,
            hit_circle_vertex_buffer,
            hit_circle_index_buffer,
            osu_camera: camera,
            camera_bind_group,
            camera_buffer,
            hit_circle_instance_buffer,
            hit_circle_instance_data,
            scale: 1.0,
            playfield_scale: (1.0, 1.0),
            playfield_offsets: (0.0, 0.0),
        }
    }

    pub fn open_beatmap<P: AsRef<Path>>(&mut self, path: P) {
        let map = match Beatmap::from_path(path) {
            Ok(m) => m,
            Err(_) => {
                println!("Failed to parse beatmap");
                return;
            },
        };

        /*

        for obj in &map.hit_objects {
            if obj.kind == HitObjectKind::Circle {
                self.hit_circle_instance_data.push(
                    HitCircleInstance::new(
                        obj.pos.x,
                        obj.pos.y
                    )
                )
            }
        }

        self.hit_circle_instance_buffer = self.state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Hit Instance Buffer"),
                contents: bytemuck::cast_slice(
                    &self.hit_circle_instance_data
                ),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            }
        );
        */

        self.current_beatmap = Some(map);
    }

    pub fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        self.state.resize(new_size);
        self.osu_camera.resize(new_size);
        
        // TODO Recreate buffers
        self.state.queue
            .write_buffer(
                &self.camera_buffer, 
                0, 
                bytemuck::bytes_of(&self.osu_camera.mat) // TODO
        );
    }

    pub fn update_egui(&mut self) {
        let input = self.egui.state.take_egui_input(&self.window);

        self.egui.context.begin_frame(input);

        egui::Window::new("Window")
            .show(&self.egui.context, |ui| {
            if let Some(beatmap) = &self.current_beatmap {
                ui.add(
                    egui::Label::new(
                        format!(
                            "Playfield scale: {} - {}", 
                            self.playfield_scale.0,
                            self.playfield_scale.1,
                        )
                    )
                );

                ui.add(
                    egui::Label::new(
                        format!(
                            "Playfield offsets: {} - {}", 
                            self.playfield_offsets.0,
                            self.playfield_offsets.1,
                        )
                    )
                );

                ui.add(
                    egui::Label::new(
                        format!("{}", self.osu_clock.get_time())
                    )
                );
            
                ui.add(
                    Slider::new(
                        &mut self.osu_clock.last_time,
                        1.0..=(beatmap.hit_objects.last().unwrap().start_time)
                    )
                );

                if !self.osu_clock.is_paused() {
                    if ui.add(egui::Button::new("pause")).clicked() {
                        self.osu_clock.pause();
                    }
                } else {
                    if ui.add(egui::Button::new("unpause")).clicked() {
                        self.osu_clock.unpause();
                    }
                }

                ui.add(
                    Slider::new(
                        &mut self.scale,
                        0.0..=100.0
                    ).text("Scale")
                );
            }
        });

        let output = self.egui.context.end_frame();

        self.egui.state.handle_platform_output(
            &self.window,
            &self.egui.context,
            output.platform_output.to_owned(),
        );

        self.egui.output = Some(output);
    }

    pub fn update(&mut self) {
        self.update_egui();
        self.osu_clock.update();

        self.hit_circle_instance_data.clear();
        
        // TODO refactor
        if let Some(beatmap) = &self.current_beatmap {
            for obj in &beatmap.hit_objects {
                if obj.kind != HitObjectKind::Circle {
                    continue;
                }

                const PREEMPT: f64 = 480.0;
                const FADEIN: f64 = 320.0;
                
                if obj.start_time < self.osu_clock.get_time() + FADEIN 
                && obj.start_time > self.osu_clock.get_time() - PREEMPT {
                    self.hit_circle_instance_data.push(
                        HitCircleInstance::new(
                            obj.pos.x,
                            obj.pos.y,
                            Vector2::new(
                                1.0, 1.0
                            ),
                        )
                    )
                }
            }

            self.hit_circle_instance_buffer = self.state.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Hit Instance Buffer"),
                    contents: bytemuck::cast_slice(
                        &self.hit_circle_instance_data
                        ),
                        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                    }
                );
        }

        // Other stuff that needs to be updated
        // TODO
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let _span = tracy_client::span!("wgpu render");

        let output = self.state.surface.get_current_texture()?;
        let view = output.texture.create_view(
            &wgpu::TextureViewDescriptor::default()
        );

        let mut encoder = self.state.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: 
                    &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(
                                wgpu::Color {
                                    r: 0.1,
                                    g: 0.2,
                                    b: 0.3,
                                    a: 1.0,
                                }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.hit_circle_pipeline);
            render_pass.set_bind_group(0, &self.hit_circle_bind_group, &[]);
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

            //render_pass.draw(0..VERTICES.len() as u32, 0..1);
            //render_pass.draw(0..4, 0..1);
            render_pass.draw_indexed(
                0..INDECIES.len() as u32,
                0,
                0..self.hit_circle_instance_data.len() as u32,
            );
        }

        // TODO errors
        let _ = self.egui.render(&self.state, &mut encoder, &view);

        self.state.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
