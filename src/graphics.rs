use wgpu::{Instance, InstanceDescriptor, PresentMode, RequestAdapterOptions, SurfaceTexture};
use winit::window::Window;
use futures::executor::block_on;

pub struct Graphics {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl Graphics {
    pub fn new(window: &Window) -> Self {
        let _span = tracy_client::span!("wgpu init");

        let supported_backend = wgpu::Backends::VULKAN;
        let device_descriptor = wgpu::DeviceDescriptor {
            label: None,
            features: wgpu::Features::default(),
            limits: wgpu::Limits::default(),
        };
        let power_preferences = wgpu::PowerPreference::HighPerformance;
        //let present_mode = PresentMode::Fifo;
        let present_mode = PresentMode::Immediate;

        let size = window.inner_size();

        let instance = Instance::new(
            InstanceDescriptor {
                backends: supported_backend,
                dx12_shader_compiler: Default::default(),
                flags: wgpu::InstanceFlags::empty(),
                gles_minor_version: wgpu::Gles3MinorVersion::Automatic
            }
        );

        let surface = unsafe { 
            instance.create_surface(&window) 
        }.unwrap();

        let adapter_options = RequestAdapterOptions {
            power_preference: power_preferences,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        };

        let adapter = block_on(instance.request_adapter(
            &adapter_options,
        )).unwrap();

        let (device, queue) = block_on(adapter.request_device(
            &device_descriptor,
            None
        )).unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| {
                f.is_srgb()
            })            
            .unwrap_or(surface_caps.formats[0]);

        let surf_features = adapter.get_texture_format_features(
            surface_format
        );

        let surf_flags = surf_features.flags;

        println!(
            "{surface_format:?}: 1x: {}, 2x: {}, 4x: {}, 8x: {}",
            surf_flags.sample_count_supported(1),
            surf_flags.sample_count_supported(2),
            surf_flags.sample_count_supported(4),
            surf_flags.sample_count_supported(8)
        );

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);


        return Graphics {
            config,
            device,
            queue,
            size,
            surface,
        }
    }

    pub fn resize(
        &mut self, 
        new_size: &winit::dpi::PhysicalSize<u32>
    ) {
        let _span = tracy_client::span!("wgpu resize");
        if new_size.width > 0 && new_size.height > 0 {
            self.size = *new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn get_current_texture(
        &self
    ) -> Result<SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }
}
