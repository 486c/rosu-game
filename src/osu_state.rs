use std::path::Path;

use egui::{Slider, style::HandleShape};
use rosu_pp::Beatmap;
use winit::window::Window;

use crate::{graphics::Graphics, egui_state::EguiState, texture::Texture};

pub struct OsuState {
    pub window: Window,
    pub state: Graphics,
    pub egui: EguiState,


    current_beatmap: Option<Beatmap>,

    current_time: f64,

    hit_circle_texture: Texture,

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
        
        Self {
            window,
            current_beatmap: None,
            egui,
            state: graphics,
            current_time: 0.0,
            hit_circle_texture,
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
                    Slider::new(&mut self.current_time, 0.0..=beatmap.hit_objects.last().unwrap().start_time)
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
            let _render_pass = encoder.begin_render_pass(
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
        }

        // TODO errors
        let _ = self.egui.render(&self.state, &mut encoder, &view);

        self.state.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
