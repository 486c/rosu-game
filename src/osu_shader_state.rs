#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct OsuShaderState {
    pub time: f32,
    pub preempt: f32,
    pub fadein: f32,
}

impl OsuShaderState {
    pub fn new(time: f32, preempt: f32, fadein: f32) -> Self {
        Self {
            time,
            preempt,
            fadein
        }
    }
}
