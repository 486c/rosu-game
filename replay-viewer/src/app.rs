use std::sync::Arc;

use egui_wgpu::wgpu::RequestAdapterOptionsBase;
use rosu::{egui_state::EguiState, graphics::{Graphics, GraphicsInitialized}};
use wgpu::{InstanceDescriptor, PowerPreference, Surface};
use winit::{application::ApplicationHandler, dpi::PhysicalSize, event_loop::EventLoopProxy, window::{Theme, Window}};

use crate::state::ReplayViewerState;


pub enum AppEvents {
    GraphicsInitialized(Arc<Graphics<'static>>),
    Resize(PhysicalSize<u32>),
}

pub struct App<'app> {
    window: Option<Arc<Window>>,
    // Used as oneshot channel to initialize graphics in async manner
    proxy: Option<EventLoopProxy<AppEvents>>,
    graphics: Option<Arc<Graphics<'app>>>,
    replay_state: Option<ReplayViewerState<'app>>,
    egui_state: Option<EguiState>,
}

impl<'app> App<'app> {
    pub fn new(proxy: EventLoopProxy<AppEvents>) -> Self {
        Self {
            window: None,
            proxy: Some(proxy),
            graphics: None,
            egui_state: None,
            replay_state: None,
        }
    }
}

impl<'app> ApplicationHandler<AppEvents> for App<'app> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let attrs = Window::default_attributes();

        let window = Arc::new(event_loop.create_window(attrs).unwrap());

        self.window = Some(window.clone());
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let _span = tracy_client::span!("app::window_event");
        let Some(window) = self.window.as_ref() else {
            println!("Window is not initialazed");
            return;
        };

        if let Some(egui_state) = &mut self.egui_state {
            egui_state.on_window_event(&event, &window);
        }

        match event {
            winit::event::WindowEvent::Resized(physical_size) => {
                // If graphics is still uninitialized
                if self.graphics.is_none() {
                    let window = self.window.as_ref().unwrap().clone();

                    // Workaround when event is not received yet
                    let proxy = match self.proxy.take() {
                        Some(proxy) => proxy,
                        None => return,
                    };

                    pollster::block_on(async move {
                        let instance = wgpu::Instance::new(&InstanceDescriptor::default());

                        let size = window.inner_size();
                        let surface = instance.create_surface(window).unwrap();

                        let mut request_adapter_options = 
                            wgpu::RequestAdapterOptions {
                                power_preference: PowerPreference::HighPerformance,
                                force_fallback_adapter: false,
                                compatible_surface: None,
                            };

                        request_adapter_options.compatible_surface = Some(&surface);

                        let adapter = 
                            instance.request_adapter(&request_adapter_options)
                            .await
                            .expect("Failed to request adapter");


                        let (device, queue) = adapter.request_device(
                            &wgpu::DeviceDescriptor {
                                label: Some("WGPU Device"),
                                memory_hints: wgpu::MemoryHints::default(),
                                required_features: wgpu::Features::default(),
                                required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                            },
                            None,
)
                        .await
                        .expect("Failed to request a device!");


                        let graphics_initialized = GraphicsInitialized {
                            surface,
                            device,
                            queue,
                            adapter,
                            size,
                        };

                        let graphics = Arc::new(Graphics::from_initialized(graphics_initialized));

                        if proxy.send_event(AppEvents::GraphicsInitialized(graphics)).is_err() {
                            println!("user event is not send");
                        };

                        if proxy.send_event(AppEvents::Resize(size)).is_err() {
                            println!("user event is not send");
                        };
                    });
                }

                if let Some(graphics) = &self.graphics {
                    graphics.resize(&physical_size)
                }

                if let Some(replay_state) = &mut self.replay_state {
                    replay_state.on_resize(&physical_size)
                }
            },
            winit::event::WindowEvent::RedrawRequested => {
                let (Some(graphics), Some(egui_state), Some(replay_state)) = (
                    self.graphics.as_ref(),
                    self.egui_state.as_mut(),
                    self.replay_state.as_mut(),
                ) else {
                    println!("Graphics is not initialazed, can't draw");
                    return;
                };

                // Egui should be on top

                // 1. Non egui-stuff
                let output = match graphics.get_current_texture() {
                    Ok(texture) => texture,
                    Err(_) => {
                        //println!("{e}");
                        return
                    },
                };

                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                replay_state.render(&view);
                
                // 2. Egui stuff
                let gui_input = egui_state.state.take_egui_input(window);
                let ctx = egui_state.state.egui_ctx();
                ctx.begin_pass(gui_input);
                replay_state.render_ui(&ctx);
                let out = egui_state.state.egui_ctx().end_pass();

                egui_state.output = Some(out);

                egui_state.render(graphics, &view).unwrap();

                output.present();
            },
            winit::event::WindowEvent::DroppedFile(path) => {
                if let Some(state) = &mut self.replay_state {
                    state.open_replay(path)
                }
            },
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                let _span = tracy_client::span!("app::keyboard_input");
                if let Some(state) = &mut self.replay_state {
                    match event.physical_key {
                        winit::keyboard::PhysicalKey::Code(key_code) => {
                            match event.state {
                                winit::event::ElementState::Pressed => {
                                    state.on_pressed_down(key_code);
                                },
                                winit::event::ElementState::Released => {
                                    //state.on_pressed_release(key_code);
                                },
                            }
                        },
                        winit::keyboard::PhysicalKey::Unidentified(_) => {}
                            //tracing::warn!("Got undefined keyboard input"),
                    }
                }
            },
            winit::event::WindowEvent::MouseWheel { delta, .. } => {
                let _span = tracy_client::span!("app::mouse_wheel");
                if let Some(state) = &mut self.replay_state {
                    match delta {
                        winit::event::MouseScrollDelta::LineDelta(x, y) => {
                            println!("({}, {})", x ,y);
                            if y > 0.0 {
                                println!("Zoom in");
                                state.zoom_in();
                            } else {
                                println!("Zoom out");
                                state.zoom_out();
                            }
                        },
                        winit::event::MouseScrollDelta::PixelDelta(_physical_position) => println!("pixel delta"),
                    }
                }
            },
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                let _span = tracy_client::span!("app::cursor_moved");
                if let Some(state) = &mut self.replay_state {
                    state.on_mouse_moved(&position);
                }
            },
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                let _span = tracy_client::span!("app::mouse_input");
                if let Some(replay_state) = &mut self.replay_state {
                    match state {
                        winit::event::ElementState::Pressed => {
                            replay_state.on_mouse_pressed(button)
                        },
                        winit::event::ElementState::Released => {
                            replay_state.on_mouse_released(button)
                        },
                    }
                }

            },
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _span = tracy_client::span!("app::about_to_wait");
        let window = self.window.as_ref().unwrap();
        window.request_redraw();
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: AppEvents) {
        match event {
            AppEvents::GraphicsInitialized(graphics) => {
                self.egui_state = Some(EguiState::new(&graphics, self.window.as_ref().unwrap()));
                self.replay_state = Some(ReplayViewerState::new(graphics.clone()));
                self.graphics = Some(graphics);
            },
            AppEvents::Resize(new_size) => {
                //if let Some(ref mut state) = self.osu_state {
                    //state.osu_renderer.on_resize(&new_size);
                //}
            },
        }
    }
}
