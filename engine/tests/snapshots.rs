use s4im_engine::*;
use std::fs;
use std::path::Path;

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

fn setup() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let inst = tmp.path().join("Los Sims 4_solo");
    write(&inst.join("saves").join("Slot_00000001.save"), "v1");
    write(&inst.join("saves").join("nested").join("x.bin"), "n1");
    write(&inst.join("Tray").join("house.trayitem"), "t1");
    write(&inst.join("Mods").join("a.package"), "m1");
    write(&inst.join("Options.ini"), "opt");
    let snaps = tmp.path().join("snapshots");
    (tmp, inst, snaps)
}

#[test]
fn creates_snapshot_without_touching_source() {
    let (_t, inst, snaps) = setup();
    let info = create_snapshot(&inst, &snaps, "solo", "manual", SnapshotKind::Manual, false).unwrap();
    assert_eq!(info.items, ["saves", "Tray"]);
    assert!(!info.includes_mods);
    assert_eq!(info.size_bytes, 2 + 2 + 2);
    assert_eq!(read(&inst.join("saves/Slot_00000001.save")), "v1");
    let data = snaps.join("solo").join(&info.id).join("data");
    assert_eq!(read(&data.join("saves/nested/x.bin")), "n1");
    assert!(!data.join("Mods").exists());
    assert!(!snaps.join("solo").join(format!(".tmp-{}", info.id)).exists());
}

#[test]
fn optional_mods_are_included() {
    let (_t, inst, snaps) = setup();
    let info = create_snapshot(&inst, &snaps, "solo", "with mods", SnapshotKind::Manual, true).unwrap();
    assert!(info.includes_mods);
    assert_eq!(info.items, ["saves", "Tray", "Mods"]);
    assert_eq!(read(&snaps.join("solo").join(&info.id).join("data/Mods/a.package")), "m1");
}

#[test]
fn same_second_snapshots_get_distinct_ids_and_list_newest_first() {
    let (_t, inst, snaps) = setup();
    let a = create_snapshot(&inst, &snaps, "solo", "a", SnapshotKind::Manual, false).unwrap();
    let b = create_snapshot(&inst, &snaps, "solo", "b", SnapshotKind::Manual, false).unwrap();
    assert_ne!(a.id, b.id);
    let list = list_snapshots(&snaps, "solo").unwrap();
    assert_eq!(list.len(), 2);
    assert!(list[0].created >= list[1].created);
    assert!(list_snapshots(&snaps, "other").unwrap().is_empty());
}

#[test]
fn restore_brings_back_old_content_and_keeps_a_safety_snapshot() {
    let (_t, inst, snaps) = setup();
    let original = create_snapshot(&inst, &snaps, "solo", "base", SnapshotKind::Manual, false).unwrap();
    write(&inst.join("saves").join("Slot_00000001.save"), "v2-corrupta");
    write(&inst.join("saves").join("extra.save"), "new");
    fs::remove_file(inst.join("Tray/house.trayitem")).unwrap();

    let safety = restore_snapshot(&inst, &snaps, "solo", &original.id).unwrap();

    assert_eq!(read(&inst.join("saves/Slot_00000001.save")), "v1");
    assert_eq!(read(&inst.join("saves/nested/x.bin")), "n1");
    assert_eq!(read(&inst.join("Tray/house.trayitem")), "t1");
    assert!(!inst.join("saves/extra.save").exists());
    assert_eq!(read(&inst.join("Options.ini")), "opt");
    assert_eq!(read(&inst.join("Mods/a.package")), "m1");
    assert!(!inst.join(".restore-tmp").exists());

    assert_eq!(safety.kind, SnapshotKind::Auto);
    let safety_data = snaps.join("solo").join(&safety.id).join("data");
    assert_eq!(read(&safety_data.join("saves/Slot_00000001.save")), "v2-corrupta");
    assert_eq!(read(&safety_data.join("saves/extra.save")), "new");
}

#[test]
fn restore_with_mods_snapshot_replaces_mods() {
    let (_t, inst, snaps) = setup();
    let snap = create_snapshot(&inst, &snaps, "solo", "m", SnapshotKind::Manual, true).unwrap();
    write(&inst.join("Mods/a.package"), "roto");
    restore_snapshot(&inst, &snaps, "solo", &snap.id).unwrap();
    assert_eq!(read(&inst.join("Mods/a.package")), "m1");
}

#[test]
fn restore_of_unknown_snapshot_changes_nothing() {
    let (_t, inst, snaps) = setup();
    let err = restore_snapshot(&inst, &snaps, "solo", "20200101-000000");
    assert!(matches!(err, Err(EngineError::NotFound(_))));
    assert_eq!(read(&inst.join("saves/Slot_00000001.save")), "v1");
    assert!(list_snapshots(&snaps, "solo").unwrap().is_empty());
}

#[test]
fn rejects_path_traversal_ids() {
    let (_t, inst, snaps) = setup();
    assert!(create_snapshot(&inst, &snaps, "..", "x", SnapshotKind::Manual, false).is_err());
    assert!(create_snapshot(&inst, &snaps, "a/b", "x", SnapshotKind::Manual, false).is_err());
    assert!(delete_snapshot(&snaps, "solo", "..\\..").is_err());
    assert!(restore_snapshot(&inst, &snaps, "solo", "../x").is_err());
}

#[test]
fn missing_instance_or_nothing_to_back_up_is_an_error() {
    let (t, inst, snaps) = setup();
    let ghost = t.path().join("no existe");
    assert!(matches!(
        create_snapshot(&ghost, &snaps, "solo", "x", SnapshotKind::Manual, false),
        Err(EngineError::NotFound(_))
    ));
    let empty = t.path().join("vacia");
    fs::create_dir_all(&empty).unwrap();
    assert!(create_snapshot(&empty, &snaps, "solo", "x", SnapshotKind::Manual, false).is_err());
    assert!(list_snapshots(&snaps, "solo").unwrap().is_empty());
    assert!(inst.exists());
}

#[test]
fn prune_removes_only_old_auto_snapshots() {
    let (_t, inst, snaps) = setup();
    let manual = create_snapshot(&inst, &snaps, "solo", "m", SnapshotKind::Manual, false).unwrap();
    let mut autos = Vec::new();
    for i in 0..4 {
        autos.push(
            create_snapshot(&inst, &snaps, "solo", &format!("a{i}"), SnapshotKind::Auto, false).unwrap(),
        );
    }
    let removed = prune_auto_snapshots(&snaps, "solo", 2).unwrap();
    assert_eq!(removed.len(), 2);
    let left = list_snapshots(&snaps, "solo").unwrap();
    assert_eq!(left.len(), 3);
    assert!(left.iter().any(|s| s.id == manual.id));
    assert_eq!(left.iter().filter(|s| s.kind == SnapshotKind::Auto).count(), 2);
}

#[test]
fn stale_tmp_folders_are_removed_but_snapshots_stay() {
    let (_t, inst, snaps) = setup();
    let keep = create_snapshot(&inst, &snaps, "solo", "x", SnapshotKind::Manual, false).unwrap();
    let stale = snaps.join("solo").join(".tmp-20200101-000000");
    write(&stale.join("data/a.bin"), "x");

    assert_eq!(clean_stale_tmp(&snaps.join("solo"), std::time::Duration::from_secs(3600)), 0);
    assert!(stale.exists());
    assert_eq!(clean_stale_tmp(&snaps.join("solo"), std::time::Duration::ZERO), 1);
    assert!(!stale.exists());
    assert_eq!(list_snapshots(&snaps, "solo").unwrap()[0].id, keep.id);
    assert_eq!(clean_stale_tmp(&snaps.join("nada"), std::time::Duration::ZERO), 0);
}

#[test]
fn delete_removes_the_snapshot() {
    let (_t, inst, snaps) = setup();
    let s = create_snapshot(&inst, &snaps, "solo", "x", SnapshotKind::Manual, false).unwrap();
    delete_snapshot(&snaps, "solo", &s.id).unwrap();
    assert!(list_snapshots(&snaps, "solo").unwrap().is_empty());
    assert!(matches!(
        delete_snapshot(&snaps, "solo", &s.id),
        Err(EngineError::NotFound(_))
    ));
}
