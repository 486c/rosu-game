use std::{fs::File, io::BufReader};

use graphics::Graphics;
use osu_state::OsuState;
use rodio::{Decoder, OutputStream, Sink};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};
pub mod math;

mod camera;
mod egui_state;
mod graphics;
mod hit_circle_instance;
mod hit_objects;
mod osu_renderer;
mod osu_state;
mod slider_instance;
mod texture;
mod timer;
mod vertex;

fn main() {
    let _client = tracy_client::Client::start();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        //.with_resizable(false)
        .with_inner_size(LogicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();

    let state = Graphics::new(&window);

    let file = BufReader::new(File::open("tests/mayday/audio.mp3").unwrap());
    let source = Decoder::new(file).unwrap();

    let (stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.pause();
    sink.append(source);

    let mut osu_state = OsuState::new(window, state, sink);

    osu_state.open_beatmap("tests/mayday/mayday.osu");

    //osu_state.set_time(194046.5);
    //osu_state.set_time(30000.0);

    //osu_state.open_beatmap("tests/single_slider.osu");
    //osu_state.open_beatmap("tests/linear_sliders.osu");

    let _ = event_loop.run(move |event, _, elwf| {
        let _span = tracy_client::span!("event_loop");

        match event {
            Event::RedrawRequested(window_id) => {
                if window_id == osu_state.window.id() {
                    'blk: loop {
                        match osu_state.render() {
                            Ok(_) => break 'blk,
                            Err(wgpu::SurfaceError::Lost) => println!("Surface Lost"),
                            Err(wgpu::SurfaceError::OutOfMemory) => elwf.set_exit(),
                            Err(e) => eprintln!("{:?}", e),
                        }
                    }
                }
            }
            Event::MainEventsCleared => {
                osu_state.update();
                osu_state.window.request_redraw();
            }
            Event::WindowEvent {
                event,
                window_id: _,
            } => {
                osu_state.egui.on_window_event(&event);

                match event {
                    WindowEvent::CloseRequested => {
                        elwf.set_exit();
                    }
                    WindowEvent::Resized(physical_size) => {
                        osu_state.resize(&physical_size);
                        //osu_state.state.resize(physical_size)
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        osu_state.resize(new_inner_size);
                        //osu_state.state.resize(*new_inner_size);
                    }
                    //WindowEvent::RedrawRequested => {
                    //}
                    _ => {}
                }
            }
            _ => {}
        };
    });
}
