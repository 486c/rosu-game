use std::path::Path;

use egui::Slider;
use rosu_map::Beatmap;
use winit::{window::Window, dpi::PhysicalSize};

use crate::{egui_state::EguiState, graphics::Graphics, hitobjects::{Object, ObjectKind}, osu_renderer::OsuRenderer, timer::Timer};

/// Return preempt and fadein based on AR
fn calculate_preempt_fadein(ar: f32) -> (f32, f32) {
    if ar > 5.0 {
        (
            1200.0 - 750.0 * (ar - 5.0) / 5.0, 
            800.0 - 500.0 * (ar - 5.0) / 5.0
        )
    } else if ar < 5.0 {
        (
            1200.0 + 600.0 * (5.0 - ar) / 5.0, 
            800.0 + 400.0 * (5.0 - ar) / 5.0
        )
    } else {
        (1200.0, 800.0)
    }
}

fn calculate_hit_window(od: f32) -> (f32, f32, f32) {
    (
        80.0 - 6.0 * od,
        140.0 - 8.0 * od,
        200.0 - 10.0 * od
    )
}

pub struct OsuState {
    pub window: Window,
    pub egui: EguiState,

    osu_renderer: OsuRenderer,


    current_beatmap: Option<Beatmap>,
    preempt: f32,
    fadein: f32,
    hit_offset: f32,

    hit_objects: Vec<Object>,

    objects_queue: Vec<usize>,

    osu_clock: Timer,

    // SLIDERS

    // TODO remove
}

impl OsuState {
    pub fn new(
        window: Window,
        graphics: Graphics
    ) -> Self {

        let egui = EguiState::new(&graphics, &window);
        let osu_renderer = OsuRenderer::new(graphics);

        
        Self {
            preempt: 0.0,
            hit_offset: 0.0,
            fadein: 0.0,
            osu_renderer,
            window,
            current_beatmap: None,
            egui,
            osu_clock: Timer::new(),
            //hit_circle_texture,
            //hit_circle_pipeline,
            //hit_circle_vertex_buffer,
            //hit_circle_index_buffer,
            //osu_camera: camera,
            //camera_bind_group,
            //camera_buffer,
            //hit_circle_instance_buffer,
            //hit_circle_instance_data,
            //shader_state: OsuShaderState::default(),
            //approach_circle_texture,
            //approach_circle_pipeline,
            //approach_circle_instance_data,
            //approach_circle_instance_buffer,
            //slider_control_point_texture,
            //slider_vertex_buffer,
            //slider_index_buffer,
            //slider_instance_buffer,
            //slider_instance_data,
            //slider_pipeline,
            //slider_verticies: slider_vertices,
            //slider_indecies: index,
            //depth_texture,
            objects_queue: Vec::with_capacity(20),
            hit_objects: Vec::new(),
        }
    }

    pub fn open_beatmap<P: AsRef<Path>>(&mut self, path: P) {
        let map = match Beatmap::from_path(path) {
            Ok(m) => m,
            Err(e) => {
                println!("Failed to parse beatmap");
                println!("{}", e);
                return;
            },
        };

        let (preempt, fadein) = calculate_preempt_fadein(
            map.approach_rate
        );
        let (_x300, _x100, x50) = calculate_hit_window(
            map.overall_difficulty
        );

        self.preempt = preempt;
        self.fadein = fadein;
        self.hit_offset = x50;

        // Convert rosu_map object to our objects
        let mut out_objects = Vec::with_capacity(map.hit_objects.len());
        for obj in &map.hit_objects {
            if let Some(cobj) = Object::from_rosu(obj) {
                out_objects.push(cobj)
            }
        }
        self.hit_objects = out_objects;

        self.current_beatmap = Some(map);
        self.apply_beatmap_transformations();

    }

    pub fn apply_beatmap_transformations(&mut self) {
        //let hit_circle_multiplier = OSU_COORDS_WIDTH * self.scale / OSU_COORDS_WIDTH;

        let cs = match &self.current_beatmap {
            Some(beatmap) => beatmap.circle_size,
            None => 4.0,
        };

        self.osu_renderer.on_cs_change(cs);

    }

    pub fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        self.osu_renderer.on_resize(new_size);

    }

    pub fn update_egui(&mut self) {
        let _span = tracy_client::span!("osu_state update egui");

        let input = self.egui.state.take_egui_input(&self.window);

        self.egui.context.begin_frame(input);

        egui::Window::new("Window")
            .show(&self.egui.context, |ui| {
            if let Some(beatmap) = &self.current_beatmap {
                ui.add(
                    egui::Label::new(
                        format!("{}", self.osu_clock.get_time())
                    )
                );

                ui.add(
                    Slider::new(
                        &mut self.osu_clock.last_time,
                        1.0..=(beatmap.hit_objects.last().unwrap().start_time)
                    ).step_by(1.0)
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
    
    // Going through every object on beatmap and preparing it to
    // assigned buffers
    pub fn prepare_objects(&mut self, time: f64) {
        let _span = tracy_client::span!("osu_state prepare objects");

        self.objects_queue.clear();

        for (_, obj) in self.hit_objects.iter_mut().enumerate() {
            if !obj.is_visible(time, self.preempt) {
                continue
            }

            // TODO circles
            match &mut obj.kind {
                ObjectKind::Circle(_) => {},
                ObjectKind::Slider(slider) => {
                    self.osu_renderer.prepare_and_render_slider_texture(slider);
                },
            }

            self.osu_renderer.prepare_object_for_render(
                obj,
                time,
                self.preempt,
                self.fadein
            );
        }
        
        // When we are done preparing all objects for rendering
        // we should not forget to upload all that to gpu
        self.osu_renderer.write_buffers();
    }

    pub fn update(&mut self) {
        let _span = tracy_client::span!("osu_state update");

        self.update_egui();
        let time = self.osu_clock.update();

        self.prepare_objects(time);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let _span = tracy_client::span!("osu_state render");

        let output = self.osu_renderer.get_graphics().get_current_texture()?;

        let view = output.texture.create_view(
            &wgpu::TextureViewDescriptor::default()
        );

        self.osu_renderer.render_objects(&view)?;

        let graphics = self.osu_renderer.get_graphics();

        let mut encoder = graphics.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
        });

        self.egui.render(graphics, &mut encoder, &view)?;

        let span = tracy_client::span!("osu_state queue::submit");
        graphics.queue.submit(std::iter::once(encoder.finish()));
        drop(span);

        let span = tracy_client::span!("osu_state render::present");
        output.present();
        drop(span);

        Ok(())
    }
}
