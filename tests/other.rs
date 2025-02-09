use std::path::PathBuf;

use rosu::hit_objects::{Object, ObjectKind};
use rosu_map::Beatmap;

fn get_other_tests_path() -> PathBuf {
    PathBuf::from("tests/data/other/")
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
