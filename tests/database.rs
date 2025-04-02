use std::{path::PathBuf, thread::sleep, time::Duration};

use rosu::osu_db::OsuDatabase;
use testdir::testdir;

#[test]
fn test_osu_database_creation() {
    let tmp_dir = testdir!();
    let db_path = tmp_dir.join("rosu.db");

    let database = OsuDatabase::new_from_path(&db_path).unwrap();

    assert!(db_path.exists());
    assert_eq!(database.beatmaps_amount(), 0);
}


#[test]
fn test_osu_database_scanning() {
    let tmp_dir = testdir!();
    let db_path = tmp_dir.join("rosu.db");
    let songs_path = PathBuf::from("tests/data/songs_folder");

    println!("db path: {:?}", db_path);

    let database = OsuDatabase::new_from_path(&db_path).unwrap();

    assert!(db_path.exists());
    assert_eq!(database.beatmaps_amount(), 0);


    let (_tx, rx) = oneshot::channel();

    database.scan_beatmaps(&songs_path, rx);

    sleep(Duration::from_secs(2));
    assert_eq!(database.beatmaps_amount(), 1);

    let expected_hash = "e2f3e496b1014c84c998be738887e315";

    assert_eq!(&database.get_beatmap_by_hash(expected_hash).unwrap().hash, expected_hash);
}
