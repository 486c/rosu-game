use log::{info, warn};
use rosu::graphics::GraphicsInitialized;
use url::Url;
use wasm_bindgen::prelude::wasm_bindgen;
use rosu::timer::Timer;
use wgpu::{MemoryHints, RequestAdapterOptions};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event_loop::{ControlFlow, EventLoopProxy};
use winit::window::Window;
use winit::{event_loop::EventLoop, platform::web::WindowAttributesExtWebSys};
use rosu::hit_objects::ObjectKind;
use rosu::{math::calculate_preempt_fadein, config::Config, graphics::Graphics, osu_renderer::OsuRenderer};
use std::sync::Arc;
use rosu::skin_manager::SkinManager;
use rosu::hit_objects::Object;
use rosu::hit_objects::hit_window::HitWindow;
use winit::platform::web::WindowExtWebSys;
use wasm_bindgen_futures::spawn_local;

static TEST_BEATMAP_BYTES: &[u8] = include_bytes!("../1.osu");

struct OsuWasmState<'ows> {
    osu_renderer: OsuRenderer<'ows>,
    skin: SkinManager,
    osu_config: Config,

    clock: Timer,
    objects: Vec<Object>,
    objects_render_queue: Vec<usize>,
    objects_jedgments_render_qeue: Vec<usize>,
    current_preempt: f32,
    current_fadein: f32,
    current_hit_window: HitWindow,
}

impl<'ows> OsuWasmState<'ows> {
    pub fn open_beatmap_from_bytes(&mut self, bytes: &[u8]) {
        let beatmap: rosu_map::Beatmap = rosu_map::from_bytes(&bytes).unwrap();
        info!("Read beatmap from bytes");

        let hit_window = HitWindow::from_od(beatmap.overall_difficulty);
        let cs = beatmap.circle_size;
        let (preempt, fadein) = calculate_preempt_fadein(beatmap.approach_rate);
        self.objects = Object::from_rosu(&beatmap);
        self.osu_renderer.on_cs_change(cs);
        self.current_preempt = preempt;
        self.current_fadein = fadein;
        self.current_hit_window = hit_window;

        self.clock.reset_time();
        self.objects_render_queue.clear();
        self.objects_jedgments_render_qeue.clear();
    }

    pub fn on_resize(&mut self, new_size: &PhysicalSize<u32>) {
        self.osu_renderer.on_resize(&new_size);
    }

    pub fn on_draw(&mut self) {
        self.objects_render_queue.clear();
        self.objects_jedgments_render_qeue.clear();

        let time = self.clock.update();

        // TODO: For now i'm just copied it from
        // OsuState, for the future i probably 
        // needed to keep them in sync :)
        for (i, obj) in self.objects.iter_mut().enumerate().rev() {
            if obj.is_judgements_visible(time, self.current_preempt) {
                //self.objects_judgments_render_queue.push(i);
            };

            if !obj.is_visible(time, self.current_preempt, &self.current_hit_window) {
                continue;
            }

            match &mut obj.kind {
                ObjectKind::Slider(slider) => {
                    self.osu_renderer.prepare_and_render_slider_texture(
                        slider, 
                        &self.skin, 
                        &self.osu_config
                    );
                }
                _ => {},
            }


            self.objects_render_queue.push(i);
        }


        self.osu_renderer.prepare_objects(
            time, self.current_preempt, self.current_fadein,
            &self.objects_render_queue, &self.objects,
            &self.skin
        );

        self.osu_renderer.prepare(
            &self.osu_config
        );

        self.osu_renderer.write_buffers();

        // Render thingy
        let output = self.osu_renderer.get_graphics().get_current_texture().unwrap();

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.osu_renderer.render_objects(
            &view,
            &self.objects_render_queue, &self.objects,
            &self.skin,
        ).unwrap();
    }
}

struct App<'app> {
    window: Option<Arc<Window>>,
    // Used as oneshot channel to initialize graphics in async manner
    proxy: Option<EventLoopProxy<AppEvents>>,
    graphics: Option<Arc<Graphics<'app>>>,
    osu_state: Option<OsuWasmState<'app>>
}

enum AppEvents {
    GraphicsInitialized(Arc<Graphics<'static>>),
}

impl<'app> App<'app> {
    fn new(proxy: EventLoopProxy<AppEvents>) -> Self {
        Self {
            window: None,
            proxy: Some(proxy),
            graphics: None,
            osu_state: None,
        }
    }
}

async fn initialize_graphics<'a>(window: Arc<Window>) -> GraphicsInitialized<'a> {
    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::GL,
        ..Default::default()
    });

    let limits = wgpu::Limits::downlevel_webgl2_defaults();

    let device_descriptor = wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::default(),
        required_limits: limits,
        memory_hints: MemoryHints::default() 
    };

    let power_preferences = wgpu::PowerPreference::None;
    let surface = instance.create_surface(window).unwrap();
    info!("Initialized surface");

    let adapter_options = RequestAdapterOptions {
        power_preference: power_preferences,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    };

    let adapter = instance.request_adapter(&adapter_options).await.unwrap();
    info!("Initialized adapter");


    let (device, queue) = adapter.request_device(&device_descriptor, None).await.unwrap();
    info!("Initialized device and queue");
    
    GraphicsInitialized {
        surface,
        adapter,
        device,
        queue,
        size,
    }

}

impl<'app> ApplicationHandler<AppEvents> for App<'app> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_canvas(None);

        let window = Arc::new(event_loop.create_window(attrs).unwrap());

        self.window = Some(window.clone());
        
        // Appending canvas to the the page
        let canvas = self.window.as_ref().unwrap().canvas().unwrap();
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                doc.get_element_by_id("app")?.append_child(&canvas).ok()?;
                Some(())
            })
        .unwrap();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::RedrawRequested => {
                let window = self.window.as_ref().unwrap();

                if let Some(ref mut state) = self.osu_state {
                    state.on_draw()
                }

                window.request_redraw();

            },
            winit::event::WindowEvent::Resized(new_size) => {
                if self.graphics.is_none() {
                    let window = self.window.as_ref().unwrap().clone();
                    let proxy = self.proxy.take().unwrap();
                    spawn_local(async move {
                        let initialized_graphics = initialize_graphics(window.clone()).await;
                        let graphics = Arc::new(Graphics::from_initialized(initialized_graphics));

                        if proxy.send_event(AppEvents::GraphicsInitialized(graphics)).is_err() {
                            info!("user event is not send");
                        };
                    });
                } else {
                    if let Some(ref mut osu_state) = self.osu_state {
                        osu_state.on_resize(&new_size);
                    }
                }
            },
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: AppEvents) {
        match event {
            AppEvents::GraphicsInitialized(graphics) => {
                let osu_config = Config::default();
                let osu_renderer = OsuRenderer::new(graphics.clone(), &osu_config);
                info!("Initialized osu! Renderer");
                let skin = SkinManager::from_static(&graphics);
                info!("Initialized static osu! skin");

                let mut state = OsuWasmState {
                    osu_renderer,
                    skin,
                    clock: Timer::new(),
                    objects: Vec::new(),
                    objects_render_queue: Vec::new(),
                    objects_jedgments_render_qeue: Vec::new(),
                    current_preempt: 0.0,
                    current_fadein: 0.0,
                    current_hit_window: HitWindow::from_od(5.0),
                    osu_config,
                };

                state.open_beatmap_from_bytes(&TEST_BEATMAP_BYTES);

                self.graphics = Some(graphics);
                self.osu_state = Some(state);
            },
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
async fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).expect("Couldn't initialize logger");

    //let event_loop = EventLoop::new().expect("Failed to initialize event loop");
    let event_loop = EventLoop::<AppEvents>::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(proxy);
    event_loop.run_app(&mut app).unwrap();

    /*


    let window_attrs = Window::default_attributes();
    event_loop.create_window(window_attrs).unwrap();

    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| {
            let canvas = web_sys::Element::from(window.canvas().unwrap());
            doc.get_element_by_id("app")?.append_child(&canvas).ok()?;
            Some(())
        })
    .unwrap();


    // Getting url path
    let current_url = {
        web_sys::window().unwrap().location().href()
    }.unwrap();

    let url = Url::parse(&current_url)
        .expect("failed to parse current url");
    
    // TODO: some magical fuckery to extract path
    // in the future protect this using some proper routing lmao
    let beatmap_id = if let Some(path) = url.path().strip_prefix("/b/") {
        let id = path.trim_matches('/').to_string();
        Some(id)
    } else {
        None
    }.expect("no beatmap id is provided. bye!");
    
    let window = Arc::new(window);

    info!("Initialized graphics");

    let osu_config = Config::default();

    let mut osu_renderer = OsuRenderer::new(graphics.clone(), &osu_config);
    info!("Initialized OsuRenderer");

    let skin = SkinManager::from_static(&graphics);
    info!("Initialized skin");
    
    let download_link = Url::parse(&format!("https://osu.direct/api/osu/{}", beatmap_id)).unwrap();

    let client = reqwest_wasm::Client::new();
    let downloaded_beatmap = client.get(download_link)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, PATCH, PUT, DELETE, OPTIONS")
        .header("Access-Control-Allow-Headers", "Origin, Content-Type, X-Auth-Token")
        .send().await.unwrap()
        .bytes().await.expect("failed to extract bytes from request sended");

    info!("Downloaded beatmap: {}", downloaded_beatmap.len());

    let beatmap: rosu_map::Beatmap = rosu_map::from_bytes(&downloaded_beatmap).unwrap();
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

    */
}
