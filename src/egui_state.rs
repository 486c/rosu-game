use egui_demo_lib::DemoWindows;
use egui_wgpu::Renderer;
use wgpu::{Device, CommandEncoder};
use winit::{window::Window, event::WindowEvent};

use crate::graphics::Graphics;

pub struct EguiState {
    pub context: egui::Context,
    pub state: egui_winit::State,
    pub renderer: Renderer,
    //pub render_pass: RenderPass,

    pub demo_app: DemoWindows,

    pub output: Option<egui::FullOutput>,
}

impl EguiState {
    pub fn new(device: &Device, window: &Window) -> Self {

        let context = egui::Context::default();

        // context.set_zoom_factor(0.5);

        //context.set_pixels_per_point(window.scale_factor() as f32);

        let winit_state = egui_winit::State::new(
            Default::default(),
            window,
            None,
            None,
        );


        let egui_renderer = Renderer::new(
            &device, 
            wgpu::TextureFormat::Bgra8UnormSrgb,
            None, 
            1
        );

        let demo_app = egui_demo_lib::DemoWindows::default();

        EguiState {
            renderer: egui_renderer,
            context,
            state: winit_state,
            //render_pass,
            demo_app,

            output: None,
        }
    }

    pub fn on_window_event(
        &mut self,
        event: &WindowEvent,
    ) {
        // TODO handle
        let _ = self.state.on_window_event(
            &self.context, event
        );
    }

    pub fn render(
        &mut self,
        graphics: &Graphics,
        encoder: &mut CommandEncoder,
        view: &wgpu::TextureView,
    ) -> Result<(), wgpu::SurfaceError> {
        if self.output.is_none() {
            println!("None");
            return Ok(());
        }

        let egui_output = self.output.take().unwrap();

        for (id, image_delta) in &egui_output.textures_delta.set {
            self.renderer
                .update_texture(
                    &graphics.device, &graphics.queue, *id, &image_delta
                );
        }

        let shapes = egui_output.shapes.as_slice();
        // TODO -1 alloc
        let paint_jobs = self.context.tessellate(
            shapes.to_vec(), 
            self.context.pixels_per_point(),
        );

        let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
            size_in_pixels: [graphics.config.width, graphics.config.height],
            pixels_per_point: self.context.pixels_per_point()
        };

        self.renderer.update_buffers(
            &graphics.device,
            &graphics.queue,
            encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
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


            self.renderer
                .render(&mut render_pass, &paint_jobs, &screen_descriptor);

        }

        for id in &egui_output.textures_delta.free {
            self.renderer.free_texture(&id);
        }

        Ok(())
    }
}
