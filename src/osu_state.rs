use std::path::Path;

use egui::{Slider, style::HandleShape};
use rosu_pp::Beatmap;
use wgpu::{ShaderStages, BindingType, TextureSampleType, TextureViewDimension, RenderPipeline, BindGroup, BufferUsages, util::DeviceExt};
use winit::window::Window;

use crate::{graphics::Graphics, egui_state::EguiState, texture::Texture, vertex::Vertex};

const VERTICES: &[Vertex] = &[
    //Vertex {pos: [0.0, 0.5]},
    //Vertex {pos: [-0.5, -0.5]},
    //Vertex {pos: [0.5, -0.5]},
    Vertex {pos: [-1.0, 1.0], uv: [0.0, 0.0]}, // 0
    Vertex {pos: [-1.0, 0.0], uv: [0.0, 1.0]}, // 1
    Vertex {pos: [0.0, 0.0], uv: [1.0, 1.0]}, // 2
    Vertex {pos: [0.0, 1.0], uv: [1.0, 0.0]}, // 3

    // 0 1 2 | 2 1 3
];

const INDECIES: &[u16] = &[0, 2, 3, 0, 1, 2];

pub struct OsuState {
    pub window: Window,
    pub state: Graphics,
    pub egui: EguiState,


    current_beatmap: Option<Beatmap>,

    current_time: f64,
    
    hit_circle_bind_group: BindGroup,
    hit_circle_texture: Texture,
    hit_circle_pipeline: RenderPipeline,
    hit_circle_vertex_buffer: wgpu::Buffer,
    hit_circle_index_buffer: wgpu::Buffer,
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
                    bind_group_layouts: &[&hit_circle_bind_group_layout],
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
                    buffers: &[Vertex::desc()],
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
            current_time: 0.0,
            hit_circle_texture,
            hit_circle_pipeline,
            hit_circle_bind_group,
            hit_circle_vertex_buffer,
            hit_circle_index_buffer,
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
        

        self.current_beatmap = Some(map);
    }

    pub fn update_egui(&mut self) {
        let input = self.egui.state.take_egui_input(&self.window);

        self.egui.context.begin_frame(input);

        egui::Window::new("Window").show(&self.egui.context, |ui| {
            if let Some(beatmap) = &self.current_beatmap {
                ui.add(
                    Slider::new(
                        &mut self.current_time, 
                        0.0..=beatmap.hit_objects.last()
                            .unwrap().start_time
                    )
                    .handle_shape(HandleShape::Rect{
                        aspect_ratio: 0.30
                    })
                    .step_by(1.0)
                    .text("Time")
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
        // Other stuff that need's to be updated
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
            render_pass.set_vertex_buffer(
                0, self.hit_circle_vertex_buffer.slice(..)
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
                0..1,
            );
        }

        // TODO errors
        let _ = self.egui.render(&self.state, &mut encoder, &view);

        self.state.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
