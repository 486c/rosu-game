use std::sync::{Arc, Mutex};

use log::warn;
use pollster::block_on;
use wgpu::{Instance, InstanceDescriptor, PresentMode, RequestAdapterOptions, SurfaceTexture};
use winit::window::Window;

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

        /*
cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
            } else {
                let size = window.inner_size();
                let instance = wgpu::Instance::default();
                let limits = wgpu::Limits::default();
            }
        }
        */




        let present_mode = PresentMode::Fifo;
        let size = window.inner_size();

        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
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
                };

            } else {

                let supported_backend = wgpu::Backends::VULKAN;
                let device_descriptor = wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::default(),
                    required_limits: wgpu::Limits::default(),
                };


                let instance = Instance::new(InstanceDescriptor {
                    backends: supported_backend,
                    dx12_shader_compiler: Default::default(),
                    flags: wgpu::InstanceFlags::empty(),
                    gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
                });
            }
        }


        let power_preferences = wgpu::PowerPreference::None;
        let surface = instance.create_surface(window).unwrap();


        warn!("Create surface");

        let adapter_options = RequestAdapterOptions {
            power_preference: power_preferences,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        };

        let adapter = instance.request_adapter(&adapter_options).await.unwrap();

        warn!("adapter");

        let (device, queue) = adapter.request_device(&device_descriptor, None).await.unwrap();

        warn!("device queue");

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surf_features = adapter.get_texture_format_features(surface_format);

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
            desired_maximum_frame_latency: 1,
        };

        surface.configure(&device, &config);

        return Graphics {
            config: Mutex::new(config),
            device,
            queue,
            size,
            surface,
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
