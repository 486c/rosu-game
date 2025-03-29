use osu_replay_parser::replay::Replay;

#[derive(Copy, Clone)]
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

#[derive(Clone)]
pub struct ReplayFrame {
    pub ts: f64,
    pub pos: (f64, f64),
    pub keys: ReplayKeys,
}

#[derive(Default)]
pub struct ReplayLog {
    pub frames: Vec<ReplayFrame>,
}

impl From<Replay> for ReplayLog {
    fn from(value: Replay) -> Self {
        let mut ts = 0;
        let mut inputs = Vec::new();

        for frame in value.replay_data.frames {
            ts += frame.w;

            let input = ReplayFrame {
                ts: ts as f64,
                pos: (frame.x as f64, frame.y as f64),
                keys: ReplayKeys {
                    k1: frame.z.contains(osu_replay_parser::replay::replay_data::Keys::K1),
                    k2: frame.z.contains(osu_replay_parser::replay::replay_data::Keys::K2),
                    m1: frame.z.contains(osu_replay_parser::replay::replay_data::Keys::M1),
                    m2: frame.z.contains(osu_replay_parser::replay::replay_data::Keys::M2),
                },
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
                    continue
                }
            }

            current_frame = Some(legacy_frame.clone());
            new_inputs.push(legacy_frame.clone());
        }

        Self {
            frames: new_inputs,
        }
    }
}

