use std::path::PathBuf;

use approx::assert_relative_eq;
use rosu::hit_objects::{Object, ObjectKind};
use rosu_map::Beatmap;

fn get_other_tests_path() -> PathBuf {
    PathBuf::from("tests/data/other/")
}

#[test]
fn test_slider_slides_stuff() {
    let base = PathBuf::from("tests/data/gameplay/")
        .join("slider_with_ticks_and_reverse.osu");

    let beatmap = Beatmap::from_path(base).unwrap();
    let beatmap_objects = Object::from_rosu(&beatmap);

    assert_eq!(beatmap_objects.len(), 1);
    assert!(matches!(beatmap_objects[0].kind, ObjectKind::Slider(_)));

    if let ObjectKind::Slider(slider) = &beatmap_objects[0].kind {
        assert_eq!(slider.repeats, 2);

        assert_eq!(slider.slide(286.0), 1);
        assert_eq!(slider.slide(799.0), 2);

        assert_relative_eq!(slider.get_slider_progress(28.0), 0.0, max_relative = 0.001);
        assert_relative_eq!(slider.get_slider_progress(545.0), 1.0, max_relative = 0.001);
        assert_relative_eq!(
            0.0, 
            slider.get_slider_progress(1062.0).round(),
        );
    }
}

#[test]
fn test_slider_ticks() {
    let base = get_other_tests_path().join("slider_with_ticks.osu");

    println!("{}", base.display());

    let beatmap = Beatmap::from_path(base).unwrap();
    let beatmap_objects = Object::from_rosu(&beatmap);

    assert_eq!(beatmap_objects.len(), 1);
    assert!(matches!(beatmap_objects[0].kind, ObjectKind::Slider(_)));

    match &beatmap_objects[0].kind {
        ObjectKind::Circle(_) => panic!("should be slider"),
        ObjectKind::Slider(slider) => {
            assert_eq!(slider.ticks.len(), 3)
        },
    }
}
