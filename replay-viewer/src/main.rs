mod analyze_cursor_renderer;
mod app;
mod click_detector;
mod judgements_list;
mod lines_vertex;
mod playfield;
mod replay_log;
mod state;
mod ui;
mod tools;

use app::{App, AppEvents};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_names(true)
        .init();

    log::info!("Started");

    let _client = tracy_client::Client::start();
    let event_loop = EventLoop::<AppEvents>::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(proxy);
    event_loop.run_app(&mut app).unwrap();
}
