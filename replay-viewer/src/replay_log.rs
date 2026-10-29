use std::{fs::File, io::Write, path::Path};

use rosu_replay::{GameMode, Key, Mod, Replay, ReplayEvent, ReplayEventOsu};

#[derive(Copy, Clone, PartialEq)]
pub struct ReplayKeys {
    pub k1: bool,
    pub k2: bool,
    pub m1: bool,
    pub m2: bool,
}

impl Default for ReplayKeys {
    fn default() -> Self {
        Self {
            k1: false,
            k2: false,
            m1: false,
            m2: false,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct ReplayFrame {
    pub ts: f64,
    pub delta: i32,
    pub pos: (f64, f64),
    pub keys: ReplayKeys,
}

pub struct ReplayLog {
    pub beatmap_hash: String,
    pub username: String,
    pub game_version: u32,
    pub frames: Vec<ReplayFrame>,
    pub mods: Mod,
}

impl ReplayLog {
    pub fn toggle_k1(&mut self, idx: usize) -> bool {
        let Some(frame) = self.frames.get_mut(idx) else {
            return false;
        };

        if frame.keys.k1 && frame.keys.m1 {
            frame.keys.k1 = false;
            frame.keys.m1 = false;
        } else {
            frame.keys.k1 = true;
            frame.keys.m1 = true;
        };

        true
    }

    pub fn toggle_k2(&mut self, idx: usize) -> bool {
        let Some(frame) = self.frames.get_mut(idx) else {
            return false;
        };

        if frame.keys.k2 && frame.keys.m2 {
            frame.keys.k2 = false;
            frame.keys.m2 = false;
        } else {
            frame.keys.k2 = true;
            frame.keys.m2 = true;
        };

        true
    }

    pub fn toggle_m1(&mut self, idx: usize) -> bool {
        let Some(frame) = self.frames.get_mut(idx) else {
            return false;
        };

        if frame.keys.m1 {
            frame.keys.m1 = false;
        } else {
            frame.keys.m1 = true;
        };

        true
    }

    pub fn toggle_m2(&mut self, idx: usize) -> bool {
        let Some(frame) = self.frames.get_mut(idx) else {
            return false;
        };

        if frame.keys.m2 {
            frame.keys.m2 = false;
        } else {
            frame.keys.m2 = true;
        };

        true
    }

    pub fn save_replay<P: AsRef<Path>>(&self, path: P) {
        let mut replay_data = Vec::new();

        for frame in &self.frames {
            let mut keys_bitflag: u32 = 0;

            if frame.keys.k1 {
                keys_bitflag |= Key::K1.value();
            } else if frame.keys.k2 {
                keys_bitflag |= Key::K2.value();
            } else if frame.keys.m1 {
                keys_bitflag |= Key::M1.value();
            } else if frame.keys.m2 {
                keys_bitflag |= Key::M2.value();
            }

            let event = ReplayEventOsu {
                time_delta: frame.delta,
                x: frame.pos.0 as f32,
                y: frame.pos.1 as f32,
                keys: Key(keys_bitflag),
            };

            replay_data.push(ReplayEvent::Osu(event));
        }

        let replay = Replay {
            mode: GameMode::Std,
            game_version: self.game_version,
            beatmap_hash: self.beatmap_hash.clone(),
            username: self.username.clone(),
            replay_hash: String::new(),
            count_300: 1,
            count_100: 1,
            count_50: 1,
            count_geki: 1,
            count_katu: 1,
            count_miss: 1,
            score: 133337,
            max_combo: 228,
            perfect: false,
            mods: self.mods,
            life_bar_graph: None,
            timestamp: Default::default(),
            replay_data,
            replay_id: 0,
            rng_seed: None,
            lazer_score_info: None, // TODO: don't forget to pass lazer data
        };

        let buffer = replay.pack().unwrap();

        let mut file = File::create(path).unwrap();
        file.write_all(&buffer).unwrap();
    }
}

impl From<Replay> for ReplayLog {
    fn from(value: Replay) -> Self {
        let mut ts = 0;
        let mut inputs = Vec::new();


        for frame in value.replay_data.iter() {
            let ReplayEvent::Osu(frame) = frame else {
                continue;
            };
            
            // skip frames but account for skip
            if frame.x == 256.0 && frame.y == -500.0 {
                ts += frame.time_delta;
                continue;
            };

            ts += frame.time_delta;

            let input = ReplayFrame {
                ts: ts as f64,
                pos: (frame.x as f64, frame.y as f64),
                keys: ReplayKeys {
                    k1: frame.keys.value() & Key::K1.value() == Key::K1.value(),
                    k2: frame.keys.value() & Key::K2.value() == Key::K2.value(),
                    m1: frame.keys.value() & Key::M1.value() == Key::M1.value(),
                    m2: frame.keys.value() & Key::M2.value() == Key::M2.value(),
                },
                delta: frame.time_delta,
            };

            inputs.push(input);
        }

        // Post proccesing frames
        // stole from osu!lazer
        if inputs.len() >= 2 && inputs[1].ts < inputs[0].ts {
            inputs[1].ts = inputs[0].ts;
            inputs[0].ts = 0.0;
        }

        if inputs.len() >= 3 && inputs[0].ts > inputs[2].ts {
            inputs[0].ts = inputs[2].ts;
            inputs[1].ts = inputs[2].ts;
        }

        if inputs.len() >= 2 && inputs[1].pos == (256.0, -500.0).into() {
            inputs.remove(1);
        }

        if inputs.len() >= 1 && inputs[0].pos == (256.0, -500.0).into() {
            inputs.remove(0);
        }

        let mut current_frame: Option<ReplayFrame> = None;
        let mut new_inputs = Vec::new();

        for legacy_frame in &inputs {
            if let Some(current_frame) = &current_frame {
                if legacy_frame.ts < current_frame.ts {
                    continue;
                }
            }

            current_frame = Some(legacy_frame.clone());
            new_inputs.push(legacy_frame.clone());
        }

        Self {
            frames: new_inputs,
            beatmap_hash: value.beatmap_hash,
            username: value.username,
            game_version: value.game_version,
            mods: value.mods,
        }
    }
}
