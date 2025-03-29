use std::path::PathBuf;

use rosu::{hit_objects::{hit_window::HitWindow, Object}, math::calc_hitcircle_diameter, osu_input::{KeyboardState, OsuInput}, processor::OsuProcessor};
use rosu_map::Beatmap;

#[test]
fn test_stable_slider_leniency() {
    /*
    // Leniency -36 ms spot for this slider is (21.387608, -237.04048)
    let beatmap_path = PathBuf::from("tests/data/gameplay/slider_leniency.osu");
    let beatmap = Beatmap::from_path(beatmap_path).unwrap();

    let mut beatmap_objects = Object::from_rosu(&beatmap);
    let hit_window = HitWindow::from_od(beatmap.overall_difficulty);
    let circle_diameter = calc_hitcircle_diameter(beatmap.circle_size);

    let mut processor = OsuProcessor::default();
    
    // Hitting slider head
    processor.store_input(
        OsuInput {
            ts: 344.0,
            pos: (199.0, 298.0).into(),
            keys: KeyboardState {
                k1: true,
                k2: false,
                m1: false,
                m2: false,
            },
            hold: false,
        }
    );

    processor.store_input(
        OsuInput {
            ts: 345.0,
            pos: (199.0, 298.0).into(),
            keys: KeyboardState {
                k1: false,
                k2: false,
                m1: false,
                m2: false,
            },
            hold: false,
        }
    );

    // Going to slider linency spot
    processor.store_input(
        OsuInput {
            ts: 536.8850574712644,
            pos: (21.387608, -237.04048).into(),
            keys: KeyboardState {
                k1: true,
                k2: false,
                m1: false,
                m2: false,
            },
            hold: false,
        }
    );
    processor.store_input(
        OsuInput {
            ts: 537.8850574712644,
            pos: (21.387608, -237.04048).into(),
            keys: KeyboardState {
                k1: true,
                k2: false,
                m1: false,
                m2: false,
            },
            hold: false,
        }
    );

    processor.store_input(
        OsuInput {
            ts: 538.8850574712644,
            pos: (21.387608, -237.04048).into(),
            keys: KeyboardState {
                k1: true,
                k2: false,
                m1: false,
                m2: false,
            },
            hold: false,
        }
    );

    // Going away from linecy spot
    processor.store_input(
        OsuInput {
            ts: 539.8850574712644,
            pos: (100.0, 100.0).into(),
            keys: KeyboardState {
                k1: false,
                k2: false,
                m1: false,
                m2: false,
            },
            hold: false,
        }
    );

    processor.process_all(&mut beatmap_objects, &hit_window, circle_diameter);

    match &beatmap_objects[0].kind {
        rosu::hit_objects::ObjectKind::Circle(_) => unimplemented!(),
        rosu::hit_objects::ObjectKind::Slider(slider) => {
            let progress = slider.get_slider_progress(slider.end_time() - 36.0);
            dbg!(slider.curve.position_at(progress));
            dbg!(&slider.hit_result);

        },
    }

    assert!(1 == 1);
    */

}
