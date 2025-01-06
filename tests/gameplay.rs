use std::{fs::read_to_string, path::{Path, PathBuf}};

use osu_replay_parser::replay::Replay;
use rosu::{hit_objects::{hit_window::HitWindow, Object}, math::get_hitcircle_diameter, processor::OsuProcessor};
use rosu_map::Beatmap;
use test_case::case;

/// Comparing gameplay process with replays

#[derive(Debug, Eq, PartialEq)]
struct Expected {
    x300: u16,
    x100: u16,
    x50: u16,
    xkatu: u16,
    xgeki: u16,
    xmiss: u16,
}

fn test_gameplay<T: AsRef<Path>>(replay_file: T, beatmap: T, expected: Expected) {
    let mut processor: OsuProcessor = Replay::open(replay_file.as_ref()).unwrap().into();
    let beatmap = Beatmap::from_path(beatmap.as_ref()).unwrap();
    let mut beatmap_objects = Object::from_rosu(&beatmap);

    let hit_window = HitWindow::from_od(beatmap.overall_difficulty);

    let circle_diameter = get_hitcircle_diameter(beatmap.circle_size);

    processor.process_all(&mut beatmap_objects, &hit_window, circle_diameter);

    let mut out = Expected {
        x300: 0,
        x100: 0,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };

    beatmap_objects.iter().for_each(|x| {
        match &x.kind {
            rosu::hit_objects::ObjectKind::Circle(circle) => {
                if let Some(result) = &circle.hit_result {
                    match result.result {
                        rosu::hit_objects::Hit::X300 => out.x300 += 1,
                        rosu::hit_objects::Hit::X100 => out.x100 += 1,
                        rosu::hit_objects::Hit::X50 => out.x50 += 1,
                        rosu::hit_objects::Hit::MISS => out.xmiss += 1,
                    }
                }

            },
            rosu::hit_objects::ObjectKind::Slider(_slider) => {},
        }
    });

    assert_eq!(out, expected, "Left - Result from processor, Right - expected");
}

#[case(
    "single_hit_circle1.osr", 
    "single_hit_circle.osu",
    Expected {
        x300: 1,
        x100: 0,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "1 x300"
)]
#[case(
"single_hit_circle2.osr", 
"single_hit_circle.osu",
Expected {
    x300: 0,
    x100: 1,
    x50: 0,
    xkatu: 0,
    xgeki: 0,
    xmiss: 0,
};
"1 x100"
)]
#[case(
"single_hit_circle3.osr", 
"single_hit_circle.osu",
Expected {
    x300: 0,
    x100: 0,
    x50: 1,
    xkatu: 0,
    xgeki: 0,
    xmiss: 0,
};
"1 x50"
)]
fn test_single_hit_circle(replay: &str, beatmap: &str, expected: Expected) {

    let base = PathBuf::from("tests/data");

    let replay_file = base.join(replay);
    let beatmap_file = base.join(beatmap);

    test_gameplay(
        replay_file, 
        beatmap_file, 
        expected
    );
}

#[case(
    "jumps_simple1.osr", 
    "jumps_simple.osu",
    Expected {
        x300: 5,
        x100: 1,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "5 x300 1 x100 part1"
)]
#[case(
    "jumps_simple2.osr", 
    "jumps_simple.osu",
    Expected {
        x300: 4,
        x100: 2,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "4 x300 2 x100"
)]
#[case(
    "jumps_simple3.osr", 
    "jumps_simple.osu",
    Expected {
        x300: 5,
        x100: 1,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "5 x300 1 x100 part2"
)]
#[case(
    "jumps_simple4.osr", 
    "jumps_simple.osu",
    Expected {
        x300: 6,
        x100: 0,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "6 x300"
)]
fn test_simple_jumps(replay: &str, beatmap: &str, expected: Expected) {

    let base = PathBuf::from("tests/data");

    let replay_file = base.join(replay);
    let beatmap_file = base.join(beatmap);

    test_gameplay(
        replay_file, 
        beatmap_file, 
        expected
    );
}
