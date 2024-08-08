#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SliderConfig {
    pub border_feather: f32,
    pub border_size_multiplier: f32,
    pub body_color_saturation: f32,
    pub body_alpha_multiplier: f32,
}

#[derive(Debug)]
pub struct Config {
    pub store_slider_textures: bool,
    pub slider: SliderConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            store_slider_textures: true,
            slider: SliderConfig {
                border_feather: 0.1,
                border_size_multiplier: 1.0,
                body_color_saturation: 1.0,
                body_alpha_multiplier: 1.0,
            },
        }
    }
}
