mod app;
mod state;
mod analyze_cursor_renderer;
mod replay_log;
mod lines_vertex;

use app::{App, AppEvents};
use winit::event_loop::{ControlFlow, EventLoop};

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
