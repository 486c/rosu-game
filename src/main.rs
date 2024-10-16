use std::sync::Arc;

use graphics::Graphics;
use osu_state::OsuState;
use rodio::{OutputStream, Sink};
pub mod math;
mod camera;
mod rgb;
mod egui_state;
mod graphics;
mod skin_ini;
mod hit_circle_instance;
mod hit_objects;
mod osu_renderer;
mod osu_state;
mod config;
mod slider_instance;
mod skin_manager;
mod texture;
mod timer;
mod ui;
mod vertex;
mod screen;
mod song_select_state;
mod osu_db;
mod quad_renderer;
mod quad_instance;
mod song_importer_ui;
mod osu_cursor_renderer;
mod frameless_source;
mod osu_input;
/*

fn main() {
    let _client = tracy_client::Client::start();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_names(true)
        .init();

    tracing::info!("Starting rosu-game!");

    let event_loop = EventLoop::new().expect("Failed to initialize event loop");

    let window = WindowBuilder::new()
        //.with_resizable(false)
        .with_inner_size(LogicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();


    let window = Arc::new(window);

    let state = Graphics::new(window.clone());

    //let file = BufReader::new(File::open("tests/mayday/audio.mp3").unwrap());
    //let source = Decoder::new(file).unwrap();

    let (stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.pause();

    let mut osu_state = OsuState::new(window.clone(), state, sink);

    let _ = event_loop.run(move |event, elwf| {
        let _span = tracy_client::span!("event_loop");

        match event {
            Event::AboutToWait => {
                osu_state.update();
                osu_state.window.request_redraw();
            }
            Event::WindowEvent {
                event,
                window_id: _,
            } => {
                osu_state.egui.on_window_event(&event, &window);

                match event {
                    WindowEvent::CloseRequested => {
                        elwf.exit();
                    }
                    WindowEvent::Resized(physical_size) => {
                        osu_state.resize(&physical_size);
                    }
                    WindowEvent::RedrawRequested => {
                        'blk: loop {
                            match osu_state.render() {
                                Ok(_) => break 'blk,
                                Err(wgpu::SurfaceError::Lost) => println!("Surface Lost"),
                                Err(wgpu::SurfaceError::OutOfMemory) => elwf.exit(),
                                Err(e) => eprintln!("{:?}", e),
                            }
                        }
                    }
                    WindowEvent::KeyboardInput{ event, ..} => {
                        match event.physical_key {
                            winit::keyboard::PhysicalKey::Code(key_code) => {
                                match event.state {
                                    winit::event::ElementState::Pressed => {
                                        osu_state.on_pressed_down(key_code);
                                    },
                                    winit::event::ElementState::Released => {
                                        osu_state.on_pressed_release(key_code);
                                    },
                                }
                            },
                            winit::keyboard::PhysicalKey::Unidentified(_) => 
                                tracing::warn!("Got undefined keyboard input"),
                        }
                    },
                    WindowEvent::CursorMoved{ device_id, position } => {
                        osu_state.on_cursor_moved(position);
                    }
                    _ => {
                    }
                }
            }
            Event::NewEvents{..} => {},
            _ => {
            }
        };
    });
}
*/
