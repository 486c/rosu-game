use std::sync::Arc;

use rosu::{graphics::Graphics, osu_state::OsuState};
use soloud::Soloud;
use winit::{application::ApplicationHandler, event_loop::{ControlFlow, EventLoop}, keyboard::KeyCode, window::Window};

pub struct OsuApp<'a> {
    window: Option<Arc<Window>>,
    state: Option<OsuState<'a>>,

    is_cntrl_pressed: bool,
}

impl<'a> ApplicationHandler for OsuApp<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let attrs = Window::default_attributes();
        let window_orig = Arc::new(event_loop.create_window(attrs).unwrap());

        self.window = Some(window_orig.clone());

        let window = window_orig.clone();
        let graphics = pollster::block_on(async move {
            Graphics::new(window.clone()).await
        });

        let window = window_orig.clone();

        let sl = Soloud::default().unwrap();

        let state = pollster::block_on(async move {
            OsuState::new(window, graphics, sl)
        });

        self.state = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match &event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            winit::event::WindowEvent::Resized(new_size) => {
                if let Some(state) = &mut self.state {
                    state.resize(&new_size);
                }
            },
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                if let Some(state) = &mut self.state {
                    match event.physical_key {
                        winit::keyboard::PhysicalKey::Code(key_code) => {
                            match event.state {
                                winit::event::ElementState::Pressed => {
                                    if key_code == KeyCode::ControlLeft {
                                        self.is_cntrl_pressed = true;
                                    }

                                    state.on_pressed_down(key_code, self.is_cntrl_pressed);
                                },
                                winit::event::ElementState::Released => {
                                    if key_code == KeyCode::ControlLeft {
                                        self.is_cntrl_pressed = false;
                                    }

                                    state.on_pressed_release(key_code);
                                },
                            }
                        },
                        winit::keyboard::PhysicalKey::Unidentified(_) => 
                            tracing::warn!("Got undefined keyboard input"),
                    }
                }
            },
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                if let Some(state) = &mut self.state {
                    state.on_cursor_moved(*position);
                }
            },
            winit::event::WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state {
                    'blk: loop {
                        match state.render() {
                            Ok(_) => break 'blk,
                            Err(wgpu::SurfaceError::Lost) => tracing::warn!("Surface Lost"),
                            Err(wgpu::SurfaceError::OutOfMemory) => tracing::error!("Render out of memory!"),
                            Err(e) => tracing::error!("Error during render: {e}"),
                        }
                    }
                }


                if let Some(window) = &mut self.window {
                    window.request_redraw();
                }
            },
            _ => {},
        }

        if let (Some(state), Some(window)) = (&mut self.state, &self.window) {
            let _ = state.egui.on_window_event(&event, &window);
        };
    }


    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(state) = &mut self.state {
            state.update();
        }
    }
}

fn main() {
    let _client = tracy_client::Client::start();
    
    /*
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_names(true)
        .init();
    */

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = OsuApp {
        window: None,
        state: None,
        is_cntrl_pressed: false,
    };

    event_loop.run_app(&mut app).unwrap();
}
