use std::sync::{Arc, Mutex};

use wgpu::{Instance, InstanceDescriptor, MemoryHints, PresentMode, RequestAdapterOptions, SurfaceTexture};
use winit::window::Window;

pub struct GraphicsInitialized<'gi> {
    pub surface: wgpu::Surface<'gi>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,
    pub size: winit::dpi::PhysicalSize<u32>,
}

pub struct Graphics<'g> {
    pub surface: wgpu::Surface<'g>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: Mutex<wgpu::SurfaceConfiguration>,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl<'g> Graphics<'g> {
    pub async fn new(window: Arc<Window>) -> Self {
        let _span = tracy_client::span!("wgpu init");

        let size = window.inner_size();

        let supported_backend = wgpu::Backends::PRIMARY;
        let device_descriptor = wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::default(),
            required_limits: wgpu::Limits::default(), 
            memory_hints: MemoryHints::default() 
        };

        let instance = Instance::new(InstanceDescriptor {
            backends: supported_backend,
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::empty(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });


        let power_preferences = wgpu::PowerPreference::None;
        let surface = instance.create_surface(window).unwrap();

        let adapter_options = RequestAdapterOptions {
            power_preference: power_preferences,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        };

        let adapter = instance.request_adapter(&adapter_options).await.unwrap();

        let (device, queue) = adapter.request_device(&device_descriptor, None).await.unwrap();

        let graphics = GraphicsInitialized {
            surface,
            device,
            queue,
            adapter,
            size,
        };

        Self::from_initialized(graphics)
    }

    pub fn from_initialized(graphics: GraphicsInitialized<'g>) -> Self {
        let present_mode = PresentMode::AutoVsync;
        let surface_caps = graphics.surface.get_capabilities(&graphics.adapter);

        tracing::info!("Surface caps: {:#?}", &surface_caps);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surf_features = graphics.adapter.get_texture_format_features(
            surface_format
        );

        let surf_flags = surf_features.flags;

        tracing::info!(
            "{surface_format:?}: 1x: {}, 2x: {}, 4x: {}, 8x: {}",
            surf_flags.sample_count_supported(1),
            surf_flags.sample_count_supported(2),
            surf_flags.sample_count_supported(4),
            surf_flags.sample_count_supported(8)
        );

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: graphics.size.width,
            height: graphics.size.height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };

        dbg!(&config);

        graphics.surface.configure(&graphics.device, &config);

        return Graphics {
            config: Mutex::new(config),
            device: graphics.device,
            queue: graphics.queue,
            size: graphics.size,
            surface: graphics.surface,
        };
    }

    pub fn resize(&self, new_size: &winit::dpi::PhysicalSize<u32>) {
        let _span = tracy_client::span!("wgpu resize");
        if new_size.width > 0 && new_size.height > 0 {
            let mut lock = self.config.lock().unwrap();

            lock.width = new_size.width;
            lock.height = new_size.height;

            self.surface.configure(&self.device, &lock);
        }
    }

    pub fn get_surface_size(&self) -> (u32, u32) {
        let lock = self.config.lock().unwrap();

        (lock.width, lock.height)
    }

    pub fn get_surface_config(&self) -> wgpu::SurfaceConfiguration {
        let lock = self.config.lock().unwrap();

        lock.clone()
    }

    pub fn get_current_texture(&self) -> Result<SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }
}
