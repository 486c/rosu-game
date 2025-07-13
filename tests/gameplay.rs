use std::path::{Path, PathBuf};

use osu_replay_parser::replay::Replay;
use rosu::{hit_objects::{hit_window::HitWindow, slider::SliderResultState, Hit, Object}, math::calc_hitcircle_diameter, processor::OsuProcessor};
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

    let circle_diameter = calc_hitcircle_diameter(beatmap.circle_size);

    processor.process_all(&mut beatmap_objects, &hit_window, circle_diameter);

    let mut out = Expected {
        x300: 0,
        x100: 0,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };

    let mut proccessed_sliders = 0;
    let mut proccessed_circles = 0;

    let mut sliders_with_result = 0;
    let mut circles_with_result = 0;

    beatmap_objects.iter().for_each(|x| {
        match &x.kind {
            rosu::hit_objects::ObjectKind::Circle(circle) => {
                proccessed_circles += 1;

                if let Some(result) = &circle.hit_result {

                    if result.result != Hit::X300 {
                        println!("Circle at {}, result: {:?} at {}", circle.start_time, result.result, result.at);
                    }

                    match result.result {
                        rosu::hit_objects::Hit::X300 => out.x300 += 1,
                        rosu::hit_objects::Hit::X100 => out.x100 += 1,
                        rosu::hit_objects::Hit::X50 => out.x50 += 1,
                        rosu::hit_objects::Hit::MISS => out.xmiss += 1,
                    }
                    circles_with_result += 1;
                }

            },
            rosu::hit_objects::ObjectKind::Slider(slider) => {
                proccessed_sliders += 1;


                if let Some(hit_result) = &slider.hit_result {

                    match hit_result.state {
                        SliderResultState::Passed(hit) => {
                            sliders_with_result += 1;

                            match hit {
                                rosu::hit_objects::Hit::X300 => out.x300 += 1,
                                rosu::hit_objects::Hit::X100 => out.x100 += 1,
                                rosu::hit_objects::Hit::X50 => out.x50 += 1,
                                rosu::hit_objects::Hit::MISS => {}, //out.xmiss += 1,
                            }
                        },
                        _ => { panic!("super bad") }
                    }
                    //println!("=============");
                    //dbg!(slider.start_time);
                    //dbg!(hit_result);
                    //dbg!(&slider.checkpoints);
                    //println!("=============");

                    /*

                    let max_possible_hits = 1 + slider.checkpoints.len() + 1;
                    let mut actual_hits = 0;

                    if hit_result.head.result != Hit::MISS {
                        actual_hits += 1;
                    }

                    if hit_result.lenience_passed {
                        actual_hits += 1;
                    }

                    actual_hits += hit_result.passed_checkpoints.len();

                    let percent = actual_hits as f32 / max_possible_hits as f32;

                    let allow300 = true;
                    let allow100 = true;

                    sliders_with_result += 1;

                    if percent >= 0.999 && allow300 {
                        println!("9 | Slider at {}, result: x300", slider.start_time);
                        out.x300 += 1;
                    }
                    else if percent >= 0.5 && allow100 {
                        println!("9 | Slider at {}, result: x100", slider.start_time);
                        out.x100 += 1;
                    }
                    else if percent > 0.0 {
                        println!("9 | Slider at {}, result: x50", slider.start_time);
                        out.x50 += 1;
                    }
                    else {
                        println!("9 | Slider at {}, result: Miss", slider.start_time);
                        //out.xmiss += 1;
                    }

                    return;
                    */
                } else {
                    panic!("uncovered slider");
                }

            },
        }
    });
    
    println!("Processed Sliders: {proccessed_sliders}");
    println!("Processed Circles: {proccessed_circles}");
    println!("Sliders with result: {sliders_with_result}");
    println!("Circles with result: {circles_with_result}");

    beatmap_objects.iter().for_each(|x| {
        match &x.kind {
            rosu::hit_objects::ObjectKind::Circle(circle) => {
                if circle.hit_result.is_none() {
                    println!("Circle without result at {}", circle.start_time);
                }
            },
            rosu::hit_objects::ObjectKind::Slider(slider) => return,
        }
    });

    assert_eq!(out, expected, "Left - Result from processor, Right - expected");
}

fn get_gameplay_tests_path() -> PathBuf {
    PathBuf::from("tests/data/gameplay")
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
    let base = get_gameplay_tests_path();

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
    let base = get_gameplay_tests_path();

    let replay_file = base.join(replay);
    let beatmap_file = base.join(beatmap);

    test_gameplay(
        replay_file, 
        beatmap_file, 
        expected
    );
}

#[case(
    "slider.osr", 
    "slider.osu",
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
    "slider2.osr", 
    "slider.osu",
    Expected {
        x300: 0,
        x100: 0,
        x50: 1,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "skipped slider tick and end, 1 x50"
)]
#[case(
    "slider3.osr", 
    "slider.osu",
    Expected {
        x300: 0,
        x100: 1,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "skipped slider tick, hit end, 1 x100"
)]
fn test_slider(replay: &str, beatmap: &str, expected: Expected) {
    let base = get_gameplay_tests_path();

    let replay_file = base.join(replay);
    let beatmap_file = base.join(beatmap);

    test_gameplay(
        replay_file, 
        beatmap_file, 
        expected
    );
}

#[case(
    "slider_two_ticks.osr", 
    "slider_two_ticks.osu",
    Expected {
        x300: 0,
        x100: 1,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "hit head and first tick, skipped rest, 1 x100"
)]
#[case(
    "slider_two_ticks2.osr", 
    "slider_two_ticks.osu",
    Expected {
        x300: 1,
        x100: 0,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "hit slider completly, 1 x300"
)]
#[case(
    "slider_two_ticks3.osr", 
    "slider_two_ticks.osu",
    Expected {
        x300: 0,
        x100: 1,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "skipped just last slider tick, 1 x100"
)]
#[case(
    "slider_two_ticks4.osr", 
    "slider_two_ticks.osu",
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
fn test_slider_two_ticks(replay: &str, beatmap: &str, expected: Expected) {
    let base = get_gameplay_tests_path();

    let replay_file = base.join(replay);
    let beatmap_file = base.join(beatmap);

    test_gameplay(
        replay_file, 
        beatmap_file, 
        expected
    );
}

#[case(
    "two_sliders.osr", 
    "two_sliders.osu",
    Expected {
        x300: 2,
        x100: 0,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "perfect sliders hit, 2 x300"
)]
#[case(
    "two_sliders2.osr", 
    "two_sliders.osu",
    Expected {
        x300: 1,
        x100: 1,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "missed slider ticks, hit perfectly second slider, 1 x300 1 x100"
)]
fn test_two_sliders(replay: &str, beatmap: &str, expected: Expected) {
    let base = get_gameplay_tests_path();

    let replay_file = base.join(replay);
    let beatmap_file = base.join(beatmap);

    test_gameplay(
        replay_file, 
        beatmap_file, 
        expected
    );
}


#[case(
    "sliders_and_jumps.osr", 
    "sliders_and_jumps.osu",
    Expected {
        x300: 6,
        x100: 1,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "1 x100, 6x300"
)]
fn test_sliders_and_jumps(replay: &str, beatmap: &str, expected: Expected) {
    let base = get_gameplay_tests_path();

    let replay_file = base.join(replay);
    let beatmap_file = base.join(beatmap);

    test_gameplay(
        replay_file, 
        beatmap_file, 
        expected
    );
}

#[case(
    "slider_with_stack.osr", 
    "slider_with_stack.osu",
    Expected {
        x300: 4,
        x100: 0,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "4 x300"
)]
#[case(
    "slider_with_stack2.osr", 
    "slider_with_stack2.osu",
    Expected {
        x300: 3,
        x100: 6,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "3 x300, 6 x100"
)]
fn test_sliders_with_stacks(replay: &str, beatmap: &str, expected: Expected) {
    let base = get_gameplay_tests_path();

    let replay_file = base.join(replay);
    let beatmap_file = base.join(beatmap);

    test_gameplay(
        replay_file, 
        beatmap_file, 
        expected
    );
}

#[case(
    "slider_with_ticks_and_reverse.osr", 
    "slider_with_ticks_and_reverse.osu",
    Expected {
        x300: 1,
        x100: 0,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "perfect hit, 1 x300"
)]
#[case(
    "slider_with_ticks_and_reverse2.osr", 
    "slider_with_ticks_and_reverse.osu",
    Expected {
        x300: 0,
        x100: 1,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "missed some ticks, 1 x100"
)]
fn test_slider_with_ticks_and_reverse(replay: &str, beatmap: &str, expected: Expected) {
    let base = get_gameplay_tests_path();

    let replay_file = base.join(replay);
    let beatmap_file = base.join(beatmap);

    test_gameplay(
        replay_file, 
        beatmap_file, 
        expected
    );
}

#[case(
    "koise.osr", 
    "koise.osu",
    Expected {
        x300: 46,
        x100: 0,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "koise normal diff, 46 x300 an SS"
)]
#[case(
    "koise2.osr", 
    "koise.osu",
    Expected {
        x300: 41,
        x100: 5,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "koise normal diff, 41 x300 5 x100 an A"
)]
#[case(
    "aozora_hard.osr", 
    "aozora_hard.osu",
    Expected {
        x300: 65,
        x100: 2,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "aozora hard diff, 65 x300 2 x100 an S"
)]
#[case(
    "getta_banban.osr", 
    "getta_banban.osu",
    Expected {
        x300: 236,
        x100: 6,
        x50: 1,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "getta banban hard diff, 236 x300 6 x100 x50 an A"
)]
#[case(
    "gin_no_kaze.osr", 
    "gin_no_kaze.osu",
    Expected {
        x300: 332,
        x100: 21,
        x50: 1,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "gin no kaze, an A"
)]
fn test_actual_ranked_map(replay: &str, beatmap: &str, expected: Expected) {
    let base = get_gameplay_tests_path();

    let replay_file = base.join(replay);
    let beatmap_file = base.join(beatmap);

    test_gameplay(
        replay_file, 
        beatmap_file, 
        expected
    );
}

#[case(
    "stacks.osr", 
    "stacks.osu",
    Expected {
        x300: 5,
        x100: 0,
        x50: 0,
        xkatu: 0,
        xgeki: 0,
        xmiss: 0,
    };
    "stack of five, with one key beign held"
)]
fn test_stacks_one_key_hold(replay: &str, beatmap: &str, expected: Expected) {
    let base = get_gameplay_tests_path();

    let replay_file = base.join(replay);
    let beatmap_file = base.join(beatmap);

    test_gameplay(
        replay_file, 
        beatmap_file, 
        expected
    );
}
