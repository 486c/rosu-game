use log::{info, warn};
use wasm_bindgen::prelude::wasm_bindgen;
use rosu::timer::Timer;
use winit::{event_loop::EventLoop, platform::web::WindowExtWebSys, window::WindowBuilder};
use rosu::hit_objects::ObjectKind;
use rosu::{math::calculate_preempt_fadein, config::Config, graphics::Graphics, osu_renderer::OsuRenderer};
use std::sync::Arc;
use rosu::skin_manager::SkinManager;
use rosu::hit_objects::Object;
use rosu::hit_objects::hit_window::HitWindow;

static TEST_BEATMAP_BYTES: &[u8] = include_bytes!("../1.osu");

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
async fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).expect("Couldn't initialize logger");

    let event_loop = EventLoop::new().expect("Failed to initialize event loop");

    let window = WindowBuilder::new()
        .build(&event_loop)
        .unwrap();

    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| {
            let canvas = web_sys::Element::from(window.canvas().unwrap());
            doc.get_element_by_id("app")?.append_child(&canvas).ok()?;
            Some(())
        })
    .unwrap();
    
    let window = Arc::new(window);

    let graphics = Arc::new(Graphics::new(window.clone()).await);
    info!("Initialized graphics");

    let osu_config = Config::default();

    let mut osu_renderer = OsuRenderer::new(graphics.clone(), &osu_config);
    info!("Initialized OsuRenderer");

    let skin = SkinManager::from_static(&graphics);
    info!("Initialized skin");

    let beatmap: rosu_map::Beatmap = rosu_map::from_bytes(TEST_BEATMAP_BYTES).unwrap();
    info!("Initialized test static beatmap!");
    
    let hit_window = HitWindow::from_od(beatmap.overall_difficulty);
    let cs = beatmap.circle_size;
    let (preempt, fadein) = calculate_preempt_fadein(beatmap.approach_rate);
    let mut our_objects = Object::from_rosu(&beatmap);
    osu_renderer.on_cs_change(cs);

    let mut objects_render_queue: Vec<usize> = Vec::new();
    let mut objects_judgments_render_queue: Vec<usize> = Vec::new();
    info!("Initialized all beatmap info required for rendering");

    let mut osu_clock = Timer::new();
    osu_clock.unpause();

    let _ = event_loop.run(move |event, elwf| {
        match event {
            winit::event::Event::WindowEvent { window_id, event } => {
                match event {
                    winit::event::WindowEvent::Resized(new_size) => {
                        osu_renderer.on_resize(&new_size);
                    },
                    winit::event::WindowEvent::CursorMoved{ .. } => {
                    },
                    winit::event::WindowEvent::RedrawRequested => {
                        objects_render_queue.clear();
                        objects_judgments_render_queue.clear();

                        let time = osu_clock.update();

                        // TODO: For now i'm just copied it from
                        // OsuState, for the future i probably 
                        // needed to keep them in sync :)
                        for (i, obj) in our_objects.iter_mut().enumerate().rev() {
                            if obj.is_judgements_visible(time, preempt) {
                                //self.objects_judgments_render_queue.push(i);
                            };

                            if !obj.is_visible(time, preempt, &hit_window) {
                                continue;
                            }

                            match &mut obj.kind {
                                ObjectKind::Slider(slider) => {
                                    osu_renderer.prepare_and_render_slider_texture(slider, &skin, &osu_config);
                                }
                                _ => {},
                            }


                            objects_render_queue.push(i);
                        }


                        osu_renderer.prepare_objects(
                            time, preempt, fadein,
                            &objects_render_queue, &our_objects,
                            &skin
                        );

                        osu_renderer.prepare(
                            &osu_config
                        );

                        osu_renderer.write_buffers();

                        // Render thingy
                        let output = osu_renderer.get_graphics().get_current_texture().unwrap();

                        let view = output
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());

                        osu_renderer.render_objects(
                            &view,
                            &objects_render_queue, &our_objects,
                            &skin,
                        ).unwrap();
                    },
                    _ => {}
                }
            },
            winit::event::Event::AboutToWait => {
                window.request_redraw();
            },
            _ => {}
        }
    });
}
