#![cfg(windows)]

use s4im_engine::*;
use std::fs;

#[test]
fn junction_points_to_instance_and_removal_keeps_target() {
    let tmp = tempfile::tempdir().unwrap();
    let inst = tmp.path().join("instancias").join("Cheesies");
    fs::create_dir_all(inst.join("saves")).unwrap();
    fs::write(inst.join("saves").join("Slot_00000001.save"), "v1").unwrap();
    let link = tmp.path().join("Los Sims 4");

    create_junction(&link, &inst).unwrap();

    assert!(matches!(active_state(&link), ActiveState::Junction(_)));
    assert_eq!(
        fs::canonicalize(junction_target(&link).unwrap()).unwrap(),
        fs::canonicalize(&inst).unwrap()
    );
    assert_eq!(fs::read_to_string(link.join("saves/Slot_00000001.save")).unwrap(), "v1");
    fs::write(link.join("saves").join("new.save"), "via-link").unwrap();
    assert_eq!(fs::read_to_string(inst.join("saves/new.save")).unwrap(), "via-link");

    remove_junction(&link).unwrap();
    assert_eq!(active_state(&link), ActiveState::Missing);
    assert_eq!(fs::read_to_string(inst.join("saves/new.save")).unwrap(), "via-link");
    assert_eq!(fs::read_to_string(inst.join("saves/Slot_00000001.save")).unwrap(), "v1");
}

#[test]
fn junction_refuses_existing_path_and_real_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let inst = tmp.path().join("inst");
    let real = tmp.path().join("Los Sims 4");
    fs::create_dir_all(&inst).unwrap();
    fs::create_dir_all(&real).unwrap();
    fs::write(real.join("keep.txt"), "x").unwrap();

    assert!(create_junction(&real, &inst).is_err());
    assert!(remove_junction(&real).is_err());
    assert!(real.join("keep.txt").exists());
    assert!(create_junction(&tmp.path().join("l"), &tmp.path().join("no existe")).is_err());
}

#[test]
fn snapshots_and_listing_do_not_follow_a_junction_into_a_loop() {
    let tmp = tempfile::tempdir().unwrap();
    let inst = tmp.path().join("inst");
    fs::create_dir_all(inst.join("saves")).unwrap();
    fs::write(inst.join("saves").join("a.save"), "a").unwrap();
    create_junction(&inst.join("saves").join("bucle"), &inst).unwrap();
    let snaps = tmp.path().join("snaps");

    let info = create_snapshot(&inst, &snaps, "i", "x", SnapshotKind::Manual, false).unwrap();
    assert_eq!(info.size_bytes, 1);

    remove_junction(&inst.join("saves").join("bucle")).unwrap();
}
