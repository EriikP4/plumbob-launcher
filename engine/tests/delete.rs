#![cfg(windows)]

use s4im_engine::*;
use std::cell::RefCell;
use std::fs;
use std::path::Path;

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn instance(root: &Path, code: &str) {
    let dir = root.join(format!("Los Sims 4_{code}"));
    write(&dir.join("saves/Slot_00000001.save"), "save");
    write(&dir.join("saves/Slot_00000002.save"), "save2");
    write(&dir.join("Mods/a.package"), "aa");
    write(&dir.join("Mods/sub/b.ts4script"), "bbb");
    write(&dir.join("Tray/house.trayitem"), "t");
    write(&dir.join("Capturas de pantalla/c.png"), "png");
    adopt_instance(root, code, "My instance", "", InstanceKind::Solo).unwrap();
}

fn silent(_: u64, _: u64) {}

#[test]
fn deletes_everything_and_reports_the_stats() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let snaps = tmp.path().join("snaps");
    instance(&root, "a");
    instance(&root, "b");
    mark_played(&root, "a", 7200).unwrap();
    create_snapshot(&root.join("Los Sims 4_a"), &snaps, "a", "x", SnapshotKind::Manual, false).unwrap();

    let stats = delete_instance(&root, &snaps, "a", false, &silent).unwrap();

    assert_eq!(stats.name, "My instance");
    assert_eq!(stats.mods, 2);
    assert_eq!(stats.saves, 2);
    assert_eq!(stats.play_seconds, 7200);
    assert_eq!(stats.kept_snapshots, 1);
    assert!(stats.bytes >= 4 + 5 + 2 + 3 + 1 + 3);
    assert!(stats.files >= 7);
    assert!(!root.join("Los Sims 4_a").exists());
    assert!(root.join("Los Sims 4_b/saves/Slot_00000001.save").exists());
    assert_eq!(list_snapshots(&snaps, "a").unwrap().len(), 1);
    assert_eq!(list_instances(&root).unwrap().len(), 1);
}

#[test]
fn progress_reaches_the_total_in_order() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    instance(&root, "a");
    let seen = RefCell::new(Vec::new());
    let track = |done: u64, total: u64| seen.borrow_mut().push((done, total));

    delete_instance(&root, &tmp.path().join("snaps"), "a", false, &track).unwrap();

    let values = seen.borrow();
    assert_eq!(values.first().unwrap().0, 0);
    let last = values.last().unwrap();
    assert_eq!(last.0, last.1);
    assert!(values.windows(2).all(|p| p[0].0 <= p[1].0));
}

#[test]
fn refuses_unregistered_missing_and_unsafe_codes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let snaps = tmp.path().join("snaps");
    fs::create_dir_all(root.join("Los Sims 4_old/saves")).unwrap();
    write(&root.join("Los Sims 4_old/saves/s.save"), "x");
    write(&tmp.path().join("fuera/importante.txt"), "no tocar");

    assert!(delete_instance(&root, &snaps, "old", false, &silent).is_err());
    assert!(delete_instance(&root, &snaps, "nope", false, &silent).is_err());
    assert!(delete_instance(&root, &snaps, "..", false, &silent).is_err());
    assert!(delete_instance(&root, &snaps, "../fuera", false, &silent).is_err());
    assert!(delete_instance(&root, &snaps, "", false, &silent).is_err());
    assert!(root.join("Los Sims 4_old/saves/s.save").exists());
    assert!(tmp.path().join("fuera/importante.txt").exists());
}

#[test]
fn links_inside_the_instance_are_removed_but_never_followed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    instance(&root, "a");
    let outside = tmp.path().join("fuera");
    write(&outside.join("importante.txt"), "no tocar");
    create_junction(&root.join("Los Sims 4_a/Mods/enlace"), &outside).unwrap();

    delete_instance(&root, &tmp.path().join("snaps"), "a", false, &silent).unwrap();

    assert!(!root.join("Los Sims 4_a").exists());
    assert_eq!(fs::read_to_string(outside.join("importante.txt")).unwrap(), "no tocar");
}

#[test]
fn read_only_files_do_not_block_the_deletion() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    instance(&root, "a");
    let file = root.join("Los Sims 4_a/saves/Slot_00000001.save");
    let mut perms = fs::metadata(&file).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&file, perms).unwrap();

    delete_instance(&root, &tmp.path().join("snaps"), "a", false, &silent).unwrap();

    assert!(!root.join("Los Sims 4_a").exists());
}

#[test]
fn deleting_the_active_instance_removes_only_the_link() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let snaps = tmp.path().join("snaps");
    instance(&root, "a");
    instance(&root, "b");
    activate_instance(&root, &snaps, "a", false).unwrap();
    assert_eq!(active_instance_id(&root).as_deref(), Some("a"));

    delete_instance(&root, &snaps, "a", false, &silent).unwrap();

    assert!(!root.join("Los Sims 4_a").exists());
    assert_eq!(active_state(&game_dir(&root)), ActiveState::Missing);
    assert!(root.join("Los Sims 4_b/saves/Slot_00000001.save").exists());
    activate_instance(&root, &snaps, "b", false).unwrap();
    assert_eq!(active_instance_id(&root).as_deref(), Some("b"));
}
