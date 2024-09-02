use std::{collections::VecDeque, sync::Arc, time::{Duration, Instant}};

use wgpu::{util::DeviceExt, BufferUsages, TextureView};
use winit::dpi::PhysicalPosition;

use crate::{buffer_write_or_init, graphics::Graphics, quad_instance::QuadInstance, quad_renderer::QuadRenderer, skin_manager::SkinManager};

const TRAIL_KEEP_LEN: usize = 10;
const TRAIL_KEEP_MS: u64 = 35;

pub struct CursorRenderer<'cr> {
    graphics: Arc<Graphics<'cr>>,
    quad_renderer: QuadRenderer<'cr>,

    trail_instance_data: VecDeque<(Instant, QuadInstance)>,
    trail_buffer: wgpu::Buffer,
    
    cursor_instance: QuadInstance,
    cursor_buffer: wgpu::Buffer,
}

impl<'cr> CursorRenderer<'cr> {
    pub fn new(graphics: Arc<Graphics<'cr>>) -> Self {
        let quad_renderer = QuadRenderer::new(graphics.clone());
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
        }
    }

    pub fn update(&mut self) {
        self.trail_instance_data.push_back((Instant::now(), self.cursor_instance));

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
        self.cursor_instance = instance;
    }

    pub fn on_resize(&mut self, new_size: &winit::dpi::PhysicalSize<u32>) {
        self.quad_renderer.resize_camera(new_size);
    }

    pub fn render_on_view(&mut self, view: &TextureView, skin: &SkinManager) {
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

        //self.graphics.queue.write_buffer(&self.cursor_buffer, 0, bytemuck::bytes_of(&self.cursor_instance));

        // 1. Trail
        self.quad_renderer.render_on_view(
            view,
            &skin.cursor_trail.bind_group,
            &self.trail_buffer,
            self.trail_instance_data.len() as u32
        );

        // 2. cursor itself
        self.quad_renderer.render_on_view(
            view, 
            &skin.cursor.bind_group, 
            &self.cursor_buffer, 
            1
        );
    }
}
