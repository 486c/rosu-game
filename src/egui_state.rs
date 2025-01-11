use egui_wgpu::{Renderer, ScreenDescriptor};
use winit::{window::Window, event::WindowEvent};

use crate::graphics::Graphics;

pub struct EguiState {
    pub state: egui_winit::State,
    pub renderer: Renderer,
    //pub render_pass: RenderPass,

    pub output: Option<egui::FullOutput>,
}

impl EguiState {
    pub fn new(graphics: &Graphics, window: &Window) -> Self {

        let context = egui::Context::default();

        let winit_state = egui_winit::State::new(
            context,
            Default::default(),
            window,
            None,
            None,
            None,
        );

        let surface_config = graphics.get_surface_config();


        let egui_renderer = Renderer::new(
            &graphics.device, 
            surface_config.format,
            None, 
            1,
            false
        );

        EguiState {
            renderer: egui_renderer,
            state: winit_state,
            output: None,
        }
    }

    pub fn on_window_event(
        &mut self,
        event: &WindowEvent,
        window: &Window,
    ) {
        let _ = self.state.on_window_event(
            window, event
        );
    }

    pub fn render(
        &mut self,
        graphics: &Graphics,
        view: &wgpu::TextureView,
    ) -> Result<(), wgpu::SurfaceError> {
        let _span = tracy_client::span!("egui_state render");

        let (graphics_width, graphics_height) = graphics.get_surface_size();

        if self.output.is_none() {
            //println!("None");
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
        let paint_jobs = self.state.egui_ctx().tessellate(
            shapes.to_vec(), 
            self.state.egui_ctx().pixels_per_point(),
        );

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [graphics_width, graphics_height],
            pixels_per_point: self.state.egui_ctx().pixels_per_point()
        };


        let mut encoder =
            graphics
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("egui encoder"),
                });

        self.renderer.update_buffers(
            &graphics.device,
            &graphics.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let descriptor = wgpu::RenderPassDescriptor {
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
            };

            let render_pass = encoder.begin_render_pass(&descriptor).forget_lifetime();
            let mut render_pass = render_pass.forget_lifetime();

            self.renderer
                .render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }


        for id in &egui_output.textures_delta.free {
            self.renderer.free_texture(&id);
        }

        graphics.queue.submit([encoder.finish()]);

        Ok(())
    }
}
