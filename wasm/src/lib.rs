use log::{info, warn};
use wasm_bindgen::prelude::wasm_bindgen;
use winit::{event_loop::EventLoop, platform::web::WindowExtWebSys, window::WindowBuilder};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
fn main() {
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


    let _ = event_loop.run(move |event, elwf| {
        match event {
            winit::event::Event::WindowEvent { window_id, event } => {
                match event {
                    winit::event::WindowEvent::Resized(_) => {},
                    winit::event::WindowEvent::CursorMoved{ .. } => {
                        info!("Cursor moved");
                    },
                    winit::event::WindowEvent::RedrawRequested => {},
                    _ => {}
                }
            },
            winit::event::Event::AboutToWait => {},
            _ => {}
        }
    });
}
