use crate::osu_input::OsuInput;

#[derive(Default)]
pub struct ReplayLog {
    frames: Vec<OsuInput>
}

impl ReplayLog {
    pub fn store_input(&mut self, input: OsuInput) {
        self.frames.push(input);
    }

    pub fn last_input(&self) -> Option<OsuInput> {
        self.frames.last().cloned() // TODO remove unwrap lol
    }
}
