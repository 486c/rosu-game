use std::{collections::VecDeque, sync::{Arc, RwLock}, time::{Duration, Instant}};

use wgpu::{util::DeviceExt, BufferUsages, TextureView};
use winit::dpi::PhysicalPosition;

use crate::{buffer_write_or_init, graphics::Graphics, quad_instance::QuadInstance, quad_renderer::QuadRenderer, skin_manager::SkinManager};

// TODO: control this through settings
const TRAIL_KEEP_MS: u64 = 55;
const TARGET_TRAIL_UPDATE_RATE: f64 = 120.0; // Per sec

pub struct CursorRenderer<'cr> {
    graphics: Arc<Graphics<'cr>>,
    quad_renderer: QuadRenderer<'cr>,
    skin_manager: Arc<RwLock<SkinManager>>,

    trail_instance_data: VecDeque<(Instant, QuadInstance)>,
    trail_buffer: wgpu::Buffer,
    
    cursor_instance: QuadInstance,
    cursor_buffer: wgpu::Buffer,

    last_update: Instant,
}

impl<'cr> CursorRenderer<'cr> {
    pub fn new(
        graphics: Arc<Graphics<'cr>>,
        skin_manager: Arc<RwLock<SkinManager>>,
    ) -> Self {
        let quad_renderer = QuadRenderer::new(graphics.clone(), false);
        quad_renderer.resize_vertex_centered(50.0, 50.0);
        let trail_instance_data = VecDeque::with_capacity(10);
        let trail_buffer = quad_renderer.create_instance_buffer();
        let cursor_buffer = quad_renderer.create_instance_buffer();

        let cursor_instance = QuadInstance::from_xy_pos(0.0, 0.0);

        Self {
            graphics,
            quad_renderer,
            trail_instance_data,
            trail_buffer,
            cursor_instance,
            cursor_buffer,
            last_update: Instant::now(),
            skin_manager,
        }
    }

    pub fn update(&mut self) {
        self.trail_instance_data.retain(|(last, _)| {
            if *last < Instant::now() - Duration::from_millis(TRAIL_KEEP_MS) {
                false
            } else {
                true
            }
        });
    }

    pub fn on_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let instance = QuadInstance::from_xy_pos(position.x as f32, position.y as f32);
        
        // Weird logic required to keep cursor trail updated at the same rate
        let now = Instant::now();
        let frame_duration: Duration = Duration::from_secs_f64(1.0 / TARGET_TRAIL_UPDATE_RATE as f64);
        let update_time = now.duration_since(self.last_update);
        if update_time > frame_duration {
            self.trail_instance_data.push_back((Instant::now(), self.cursor_instance));
            self.last_update = now;
        }

        self.cursor_instance = instance;
    }

    pub fn on_resize(&mut self, new_size: &winit::dpi::PhysicalSize<u32>) {
        self.quad_renderer.resize_camera(new_size);
    }

    pub fn render_on_view(&mut self, view: &TextureView) {
        let skin = self.skin_manager.read().expect("failed");

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.cursor_buffer,
            &[self.cursor_instance],
            QuadInstance
        );

        let trail: Vec<QuadInstance> = self.trail_instance_data.iter().map(|(_, instance)| *instance).collect();

        buffer_write_or_init!(
            self.graphics.queue,
            self.graphics.device,
            self.trail_buffer,
            &trail,
            QuadInstance
        );

        // 1. Trail
        self.quad_renderer.render_on_view_instanced(
            view,
            &skin.cursor_trail.bind_group,
            &self.trail_buffer,
            0..self.trail_instance_data.len() as u32
        );

        // 2. cursor itself
        self.quad_renderer.render_on_view_instanced(
            view, 
            &skin.cursor.bind_group, 
            &self.cursor_buffer, 
            0..1
        );
    }
}
