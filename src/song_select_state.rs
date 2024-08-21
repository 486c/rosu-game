use std::{sync::{mpsc::{Receiver, Sender}, Arc}, time::Duration};

use egui::{scroll_area::ScrollBarVisibility, Align, Color32, Label, Margin, RichText, Stroke};
use egui_extras::{Size, StripBuilder};
use image::DynamicImage;
use rosu_map::Beatmap;
use wgpu::{util::DeviceExt, BufferUsages, TextureView};
use winit::dpi::PhysicalSize;

use crate::{camera::Camera, graphics::Graphics, hit_circle_instance::HitCircleInstance, osu_db::{BeatmapEntry, OsuDatabase}, osu_renderer::QUAD_INDECIES, osu_state::OsuStateEvent, rgb::Rgb, texture::Texture, vertex::Vertex};


const CARD_INNER_MARGIN: Margin = Margin {
    left: 5.0,
    right: 0.0,
    top: 8.0,
    bottom: 0.0,
};


enum SongSelectionEvents {
    SelectBeatmap(BeatmapEntry),
    LoadedBeatmap{ beatmap: Beatmap, image: DynamicImage },
    StartBeatmap(BeatmapEntry),
}

pub struct SongSelectionState<'ss> {
    db: OsuDatabase,
    graphics: Arc<Graphics<'ss>>,

    // Min & Max row that we currently need to draw
    min: usize,
    max: usize,

    // Current selected row
    current: usize,

    current_beatmap: Option<Beatmap>,
    current_background_image: Option<Texture>,

    inner_tx: Sender<SongSelectionEvents>,
    inner_rx: Receiver<SongSelectionEvents>,

    state_tx: Sender<OsuStateEvent>,

    // wgpu stuff
    camera: Camera,
    camera_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    quad_vertex_buffer: wgpu::Buffer,
    quad_index_buffer: wgpu::Buffer,
    quad_pipeline: wgpu::RenderPipeline,
    quad_instance_data: Vec<HitCircleInstance>,
    quad_instance_buffer: wgpu::Buffer,
}

impl<'ss> SongSelectionState<'ss> {
    pub fn new(graphics: Arc<Graphics<'ss>>, state_tx: Sender<OsuStateEvent>) -> Self {
        let (inner_tx, inner_rx) = std::sync::mpsc::channel();

        let quad_shader = graphics
            .device
            .create_shader_module(wgpu::include_wgsl!("shaders/hit_circle.wgsl"));

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
                    label: Some("hit_circle_index_buffer"),
                    contents: bytemuck::cast_slice(QUAD_INDECIES),
                    usage: BufferUsages::INDEX,
                });

        let quad_vertex_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("hit_circle_buffer"),
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

        let quad_instance_data = vec![
            HitCircleInstance::new(
                surface_config.width as f32 / 2.0,
                surface_config.height as f32 / 2.0,
                1.0,
                1.0,
                &Rgb::new(0, 0, 0),
            )
        ];

        let quad_instance_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("quad Instance Buffer"),
                    contents: bytemuck::cast_slice(&quad_instance_data),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
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
                    label: Some("hitcircle pipeline Layout"),
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
                    label: Some("hit_circle render pipeline"),
                    layout: Some(&quad_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &quad_shader,
                        entry_point: "vs_main",
                        buffers: &[Vertex::desc(), HitCircleInstance::desc()],
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

        Self {
            db: OsuDatabase::new().unwrap(), // TODO: REMOVE UNRAP
            min: 0,
            max: 0,
            current: 0,
            inner_tx,
            inner_rx,
            current_beatmap: None,
            graphics,
            current_background_image: None,
            camera,
            camera_bind_group,
            quad_pipeline,
            quad_instance_data,
            quad_instance_buffer,
            quad_vertex_buffer,
            quad_index_buffer,
            camera_buffer,
            state_tx,
        }
    }
    
    // Spawns a thread to parse a beatmap
    fn open_beatmap(&self, beatmap: &BeatmapEntry) {
        let tx = self.inner_tx.clone();
        let path = beatmap.path.clone();

        std::thread::spawn(move || {
            let parsed_beatmap = Beatmap::from_path(&path).unwrap();
            let bg_filename = parsed_beatmap.background_file.clone();
            let bg_path = path.parent()
                .unwrap()
                .join(&bg_filename);
        
            let img = image::open(bg_path).unwrap();
            let img = img.blur(5.0);

            tx.send(SongSelectionEvents::LoadedBeatmap{
                beatmap: parsed_beatmap,
                image: img,
            })
        });
    }


    pub fn on_resize(&mut self, new_size: &PhysicalSize<u32>) {
        if let Some(bg_img) = &self.current_background_image {
            self.resize_background_vertex(bg_img.width, bg_img.height);
        }

        self.camera.resize(new_size);

        self.quad_instance_data.clear();
        self.quad_instance_data.push(
            HitCircleInstance::new(
                new_size.width as f32 / 2.0,
                new_size.height as f32 / 2.0,
                1.0,
                1.0,
                &Rgb::new(0, 0, 0),
            )
        );

        self.graphics
            .queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&self.camera));

        self.graphics
            .queue
            .write_buffer(&self.quad_instance_buffer, 0, bytemuck::cast_slice(&self.quad_instance_data));
    }
    
    fn resize_background_vertex(&self, width: f32, height: f32) {
        let image_width = width;
        let image_height = height;

        let (graphics_width, graphics_height) = self.graphics.get_surface_size();
        let (graphics_width, graphics_height) = (graphics_width as f32, graphics_height as f32);

        let (mut to_width, mut to_height) = (graphics_width, graphics_height);

        let image_ratio = image_width as f32 / image_height as f32;
        let surface_ratio = graphics_width as f32 / graphics_height as f32;

        let (width, height) = (graphics_height * image_ratio, graphics_width / image_ratio);

        if surface_ratio < image_ratio {
            to_width = width;
        } else {
            to_height = height
        };


        self.graphics
            .queue
            .write_buffer(&self.quad_vertex_buffer, 0, bytemuck::cast_slice(
                &Vertex::quad_centered(to_width, to_height)
            ));

        tracing::info!("Resized background image vertex, width: {}, height: {}", image_width, image_height);
    }

    fn load_background(&mut self, image: DynamicImage) {
        self.resize_background_vertex(image.width() as f32, image.height() as f32);

        let texture = Texture::from_image(
            image,
            &self.graphics
        );



        self.current_background_image = Some(texture);

    }

    pub fn update(&mut self) {
        match self.inner_rx.try_recv() {
            Ok(event) => {
                match event {
                    SongSelectionEvents::SelectBeatmap(entry) => {
                        self.open_beatmap(&entry);
                    },
                    SongSelectionEvents::LoadedBeatmap{ beatmap, image }  => {
                        self.load_background(image);
                        self.current_beatmap = Some(beatmap);
                    },
                    SongSelectionEvents::StartBeatmap(entry) => {
                        let _ = self.state_tx.send(OsuStateEvent::StartBeatmap(entry));
                    },
                }
            },
            Err(e) => match e {
                std::sync::mpsc::TryRecvError::Empty => {},
                std::sync::mpsc::TryRecvError::Disconnected => {
                    tracing::error!("Channel is closed!")
                },
            },
        }
    }

    pub fn render_background(&self, view: &TextureView) {
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

            if let Some(texture) = &self.current_background_image {
                render_pass.set_pipeline(&self.quad_pipeline);
                render_pass.set_bind_group(0, &texture.bind_group, &[]);
                render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, self.quad_instance_buffer.slice(..));
                render_pass.set_index_buffer(
                    self.quad_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint16,
                );

                render_pass.draw_indexed(
                    0..QUAD_INDECIES.len() as u32,
                    0,
                    0..1,
                );
            }
        }

        self.graphics
            .queue
            .submit([encoder.finish()]);
    }

    pub fn render_beatmap_card_info(&mut self, ui: &mut egui::Ui) {
        egui::Frame::default()
            .rounding(5.0)
            .outer_margin(10.0)
            .inner_margin(5.0)
            .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 255))
            .show(ui, |ui| {

                ui.set_width(ui.available_rect_before_wrap().width());
                ui.set_height(ui.available_rect_before_wrap().height());
                if let Some(b) = &mut self.current_beatmap {
                    ui.add(Label::new(RichText::new(format!("{} - {} [{}]", &b.artist, &b.title, &b.version)).heading()).selectable(false));
                    ui.add(Label::new(format!("Mapped by {}", &b.creator)).selectable(false));

                    let last_hitobject_time = if let Some(obj) = b.hit_objects.last_mut() {
                        obj.end_time() as u64
                    } else {
                        0
                    };

                    let length = 
                        Duration::from_millis(last_hitobject_time);

                    let length_str = format!(
                        "{:02}:{:02}",
                        length.as_secs() / 60,
                        length.as_secs() % 60
                    );

                    let (bpm_max, bpm_min) = {
                        let mut max: f64 = f64::MIN;
                        let mut min: f64 = f64::MAX;

                        for point in &b.control_points.timing_points {
                            let bpm = 1.0 / point.beat_len * 1000.0 * 60.0;

                            max = max.max(bpm);
                            min = max.min(bpm);
                        }

                        (max, min)
                    };

                    let text = format!(
                        "Length: {} BPM: {:.0}-{:.0} Objects: {}",
                        length_str, 
                        bpm_min, bpm_max,
                        b.hit_objects.len() 
                    );
                    ui.add(Label::new(RichText::new(&text).strong()).selectable(false));

                    let circles = b.hit_objects.iter().filter(|h| {
                        match h.kind {
                            rosu_map::section::hit_objects::HitObjectKind::Circle(_) => true,
                            _ => false,
                        }
                    }).count();

                    let sliders = b.hit_objects.iter().filter(|h| {
                        match h.kind {
                            rosu_map::section::hit_objects::HitObjectKind::Slider(_) => true,
                            _ => false,
                        }
                    }).count();

                    let spinners = b.hit_objects.iter().filter(|h| {
                        match h.kind {
                            rosu_map::section::hit_objects::HitObjectKind::Spinner(_) => true,
                            _ => false,
                        }
                    }).count();

                    ui.add(Label::new(format!("Circles: {} Slider: {} Spinners: {}", circles, sliders, spinners)).selectable(false));
                    ui.add(Label::new(format!(
                                "CS:{:.2} AR:{:.2} OD:{:.2} HP:{:.2} Stars:TODO", 
                                b.circle_size, b.approach_rate, b.overall_difficulty, b.hp_drain_rate
                    )).selectable(false));
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.spinner();
                    });
                }
            });
    }

    pub fn render(&mut self, input: egui::RawInput, ctx: &egui::Context, view: &TextureView) -> egui::FullOutput {
        self.render_background(view);

        ctx.begin_frame(input);

        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            StripBuilder::new(ui)
                .size(Size::relative(0.6))
                .size(Size::relative(0.4))
                .horizontal(|mut strip| {
                    strip.strip(|builder| {
                        builder
                            .size(Size::relative(0.2))
                            .size(Size::relative(0.8))
                            .vertical(|mut strip| {
                                // INFO ABOUT BEATMAP
                                strip.cell(|ui| {
                                    self.render_beatmap_card_info(ui);
                                });

                                strip.strip(|builder| {
                                    builder
                                        .size(Size::relative(0.9))
                                        .size(Size::relative(0.1))
                                        .vertical(|mut strip| {
                                            strip.cell(|_ui| {});

                                            strip.cell(|ui| {
                                                egui::Frame::default()
                                                    .rounding(5.0)
                                                    .outer_margin(10.0)
                                                    .inner_margin(5.0)
                                                    .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 255))
                                                    .show(ui, |ui| {
                                                        ui.set_width(ui.available_rect_before_wrap().width());
                                                        ui.set_height(ui.available_rect_before_wrap().height());
                                                        ui.centered_and_justified(|ui| {
                                                            let text = format!("Beatmaps: {}", self.db.beatmaps_amount());
                                                            ui.add(Label::new(RichText::new(text).heading())
                                                                .selectable(false));
                                                        })
                                                    });
                                            })
                                        });
                                })
                            });
                    });

                    strip.cell(|ui| {
                        let row_height = 72.0;

                        egui::ScrollArea::vertical()
                        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                        .show_viewport(ui, |ui, rect| {
                            let min_row = (rect.min.y / row_height).floor() as usize;
                            let max_row = (rect.max.y / row_height).floor() as usize;
                            let total_height = 64.0 * self.db.beatmaps_amount() as f32;

                            ui.set_height(total_height);

                            let fill_top = (min_row - 0) as f32 * (row_height);
                            egui::Frame::none()
                                .show(ui, |ui| {
                                    ui.set_height(fill_top);
                                });

                            if max_row != self.max || min_row != self.min {
                                self.db.load_beatmaps_range(min_row, max_row);
                            }

                            let current = min_row;
                            
                            for (i, beatmap) in self.db.cache.iter().enumerate() {
                                let id = current + i;
                                let res = egui::Frame::default()
                                    .inner_margin(CARD_INNER_MARGIN)
                                    .outer_margin(0.0)
                                    .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 160))
                                    .stroke({
                                        if self.current == id {
                                            Stroke::new(1.0, Color32::RED)
                                        } else {
                                            Stroke::new(1.0, Color32::BLACK)
                                        }
                                    })
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_rect_before_wrap().width());
                                        ui.set_height(64.0);
                                        ui.set_max_height(64.0);


                                        ui.add(Label::new(RichText::new(&beatmap.title).heading()).selectable(false));
                                        ui.add(Label::new(format!("{} // {}", &beatmap.artist, &beatmap.creator)).selectable(false));
                                        ui.add(Label::new(&beatmap.version).selectable(false));
                                    });

                                let sense = res.response.interact(egui::Sense::click());

                                if sense.clicked() {
                                    self.current = id;

                                    let _ = 
                                        self.inner_tx.send(
                                            SongSelectionEvents::SelectBeatmap(beatmap.clone())
                                        ); // TODO handle this shit

                                    res.response.scroll_to_me(Some(Align::Center));
                                }

                                if sense.double_clicked() {
                                    self.current = id;
                                    let _ = self.inner_tx.send(
                                        SongSelectionEvents::StartBeatmap(beatmap.clone())
                                    );
                                    res.response.scroll_to_me(Some(Align::Center));
                                }
                            };
                            
                            self.min = min_row;
                            self.max = max_row;
                        });

                    })
                })
        });


        ctx.end_frame()
    }
}
