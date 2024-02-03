use osu_state::OsuState;
use graphics::Graphics;
use winit::{
    event_loop::EventLoop,
    window::WindowBuilder, 
    event::{WindowEvent, Event}, 
};

mod graphics;
mod texture;
mod vertex;
mod egui_state;
mod osu_state;
mod camera;
mod hit_circle_instance;
mod timer;

fn main() {
    let _client = tracy_client::Client::start();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .build(&event_loop)
        .unwrap();

    let state = Graphics::new(&window);

    let mut osu_state = OsuState::new(
        window,
        state
    );

    osu_state.open_beatmap("tests/test.osu");

    let _ = event_loop.run(move |event, _, elwf| {
        let _span = tracy_client::span!("event_loop");

        match event {
            Event::RedrawRequested(window_id) => {
                if window_id == osu_state.window.id() {
                    match osu_state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => 
                            osu_state.resize(&osu_state.state.size.clone()),
                            //osu_state.state.resize(osu_state.state.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => 
                            elwf.set_exit(),
                        Err(e) => eprintln!("{:?}", e),
                    }
                }

            },
            Event::MainEventsCleared => {
                osu_state.update();
                osu_state.window.request_redraw();
            },
            Event::WindowEvent{
                event,
                window_id: _,
            } => {
                osu_state.egui.on_window_event(
                    &event
                );

                match event {
                    WindowEvent::CloseRequested => {
                        elwf.set_exit();
                    },
                    WindowEvent::Resized(physical_size) => {
                        osu_state.resize(&physical_size);
                        //osu_state.state.resize(physical_size)
                    },
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        osu_state.resize(new_inner_size);
                        //osu_state.state.resize(*new_inner_size);
                    }
                    //WindowEvent::RedrawRequested => {
                    //}
                    _ => {},
                }
            },
            _ => {},
        };
    });
}
