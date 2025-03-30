mod app;
mod state;
mod analyze_cursor_renderer;
mod replay_log;

use std::num::NonZeroU64;

use app::{App, AppEvents};
use egui::Rect;
use rosu::graphics::GraphicsInitialized;
use winit::event_loop::{ControlFlow, EventLoop};

use {
    egui_wgpu::wgpu::util::DeviceExt,
    egui_wgpu::{self, wgpu},
};

fn main() {
    env_logger::init();
    log::info!("Started");

    let _client = tracy_client::Client::start();
    let event_loop = EventLoop::<AppEvents>::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(proxy);
    event_loop.run_app(&mut app).unwrap();
}

/*

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<MyApp>::new(MyApp::new(&cc)))
        }),
    )
}

struct MyApp {
    pub tick: f32
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            tick: 0.0
        }
    }
}

impl MyApp {
    pub fn new<'a>(cc: &'a eframe::CreationContext<'a>) -> Self {
        let wgpu_render_state = cc.wgpu_render_state.as_ref().unwrap();

        let graphics_initialzed = GraphicsInitialized {
            surface: todo!(),
            device: todo!(),
            queue: todo!(),
            adapter: todo!(),
            size: todo!(),
        };
        
        /*
        let device = &wgpu_render_state.device;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("custom3d"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./custom3d_wgpu_shader.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("custom3d"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(16),
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("custom3d"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("custom3d"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: None,
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu_render_state.target_format.into())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("custom3d"),
            contents: bytemuck::cast_slice(&[0.0_f32; 4]), // 16 bytes aligned!
            // Mapping at creation (as done by the create_buffer_init utility) doesn't require us to to add the MAP_WRITE usage
            // (this *happens* to workaround this bug )
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("custom3d"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(TriangleRenderResources {
                pipeline,
                bind_group,
                uniform_buffer,
            });
        */

        Self::default()
    }

    fn custom_painting(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let (rect, response) =
            ui.allocate_exact_size(egui::Vec2::new(ui.available_width(), ui.available_height()), egui::Sense::drag());

        self.tick += response.drag_motion().x * 0.01;
        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            CustomTriangleCallback { angle: self.tick },
        ));
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.options_mut(|options| {
            options.max_passes = std::num::NonZeroUsize::new(3).unwrap();
        });

        // Disable text wrapping
        //
        // egui text layouting tries to utilize minimal width possible
        ctx.style_mut(|style| {
            style.wrap_mode = Some(egui::TextWrapMode::Extend);
        });


        let align_flex_content_in_center = |style: &mut Style| {
            // Align content in center in flexbox layout
            style.justify_content = Some(taffy::JustifyContent::Center);
            style.align_items = Some(taffy::AlignItems::Center);
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            tui(ui, ui.id().with("demo"))
                .reserve_available_space()
                .style(egui_taffy::taffy::Style {
                    display: Display::Grid,
                    grid_template_columns: vec![fr(0.2), fr(1.)],
                    grid_template_rows: vec![fr(1.0), fr(0.05)],
                    size: percent(1.0),
                    align_items: Some(taffy::AlignItems::Stretch),
                    justify_items: Some(taffy::AlignItems::Stretch),

                    ..Default::default()
                })
            .show(|tui| {
                tui
                .mut_style(align_flex_content_in_center)
                .add_with_border(|tui| {
                    tui.label("Test1")
                });

                tui
                .style(Style {
                    align_items: Some(taffy::AlignItems::Start),
                    justify_content: Some(taffy::JustifyContent::Start),
                    grid_column: style_helpers::line(2),
                    grid_row: style_helpers::line(1),
                    ..Default::default()
                })
                .add_with_border(|tui| {
                    let container = tui.taffy_container();
                    let rect = container.full_container();
                    //let rect = tui.current_viewport_content();
                    let egui_ui = tui.egui_ui_mut();
                    egui::Frame::canvas(egui_ui.style()).show(egui_ui, |ui| {
                      self.custom_painting(ui, rect);
                    });
                });

                tui
                .style(Style {
                    align_items: Some(taffy::AlignItems::Center),
                    justify_content: Some(taffy::JustifyContent::Center),
                    grid_column: span(2),
                    ..Default::default()
                })
                .add_with_border(|tui| {
                    let _ = tui.button(|tui| {
                        tui.label("Button");
                    });

                    let slider_width = tui.current_viewport_content().width();
                    let egui_ui = tui.egui_ui_mut();
                    egui_ui.spacing_mut().slider_width = slider_width;
                    egui_ui.add(egui::Slider::new(&mut self.tick, 0.0..=100.0).show_value(false));
                });

                //tui
                //.mut_style(align_flex_content_in_center)
                //.add_with_border(|tui| {
                    //tui.label("Test4");
                //});
            });
        });
    }
}

struct CustomTriangleCallback {
    angle: f32,
}

impl egui_wgpu::CallbackTrait for CustomTriangleCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let resources: &TriangleRenderResources = resources.get().unwrap();
        resources.prepare(device, queue, self.angle);
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let resources: &TriangleRenderResources = resources.get().unwrap();
        resources.paint(render_pass);
    }
}

struct TriangleRenderResources {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl TriangleRenderResources {
    fn prepare(&self, _device: &wgpu::Device, queue: &wgpu::Queue, angle: f32) {
        // Update our uniform buffer with the angle from the UI
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[angle, 0.0, 0.0, 0.0]),
        );
    }

    fn paint(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        // Draw our triangle!
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

*/
