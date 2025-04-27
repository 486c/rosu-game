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
    /// Toggle storing slider textures in the gpu for future reuse
    pub store_slider_textures: bool,
    /// Will use judgements colors instead of skin colors
    /// for drawing hit objects, useful for debugging
    pub debug_use_judgements_as_colors: bool,
    pub slider: SliderConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            store_slider_textures: true,
            slider: SliderConfig {
                border_feather: 0.1,
                border_size_multiplier: 0.65,
                body_color_saturation: 0.62,
                body_alpha_multiplier: 0.65,
            },
            debug_use_judgements_as_colors: false,
        }
    }
}
