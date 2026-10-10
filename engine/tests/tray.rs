#![cfg(windows)]

use s4im_engine::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const EXTENSIONS: [&str; 9] = ["blueprint", "bpi", "hhi", "householdbinary", "room", "sgi", "trayitem", "rmi", "desconocida"];

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

fn set_age(path: &Path, seconds_ago: u64) {
    let file = fs::File::options().write(true).open(path).unwrap();
    file.set_modified(SystemTime::now() - Duration::from_secs(seconds_ago)).unwrap();
}

struct Env {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    snaps: PathBuf,
}

fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let snaps = tmp.path().join("snaps");
    fs::create_dir_all(&root).unwrap();
    Env { root, snaps, _tmp: tmp }
}

fn instance(e: &Env, code: &str, tag: &str) {
    let dir = e.root.join(format!("Los Sims 4_{code}"));
    for ext in EXTENSIONS {
        write(&dir.join(format!("Tray/0x{tag}00.{ext}")), &format!("{tag}-{ext}"));
    }
    write(&dir.join("Tray/sub/extra.bin"), "anidado");
    write(&dir.join("saves/Slot_00000001.save"), "save");
    adopt_instance(&e.root, code, code, "", InstanceKind::Solo).unwrap();
}

fn tray(e: &Env, code: &str) -> PathBuf {
    e.root.join(format!("Los Sims 4_{code}/Tray"))
}

fn old(e: &Env, code: &str) -> PathBuf {
    e.root.join(format!("Los Sims 4_{code}/TrayOld"))
}

fn summary(e: &Env, code: &str) -> InstanceSummary {
    list_instances(&e.root).unwrap().into_iter().find(|i| i.id == code).unwrap()
}

#[test]
fn enabling_moves_every_file_to_the_shared_tray_and_links_it() {
    let e = env();
    instance(&e, "a", "aa");
    assert!(!summary(&e, "a").tray_synced);

    let report = enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();

    assert_eq!(report.copied as usize, EXTENSIONS.len() + 1);
    let shared = shared_tray_dir(&e.root);
    for ext in EXTENSIONS {
        assert_eq!(read(&shared.join(format!("0xaa00.{ext}"))), format!("aa-{ext}"));
        assert_eq!(read(&old(&e, "a").join(format!("0xaa00.{ext}"))), format!("aa-{ext}"));
        assert_eq!(read(&tray(&e, "a").join(format!("0xaa00.{ext}"))), format!("aa-{ext}"));
    }
    assert_eq!(read(&shared.join("sub/extra.bin")), "anidado");
    let s = summary(&e, "a");
    assert!(s.tray_synced && s.tray_old);
    assert_eq!(s.tray_count, 1);
    assert_eq!(read(&e.root.join("Los Sims 4_a/saves/Slot_00000001.save")), "save");
    assert_eq!(list_snapshots(&e.snaps, "a").unwrap().len(), 1);

    write(&tray(&e, "a").join("new.trayitem"), "created by the sync");
    assert_eq!(read(&shared.join("new.trayitem")), "created by the sync");
}

#[test]
fn a_second_instance_merges_instead_of_replacing() {
    let e = env();
    instance(&e, "a", "aa");
    instance(&e, "b", "bb");
    write(&e.root.join("Los Sims 4_a/Tray/common.trayitem"), "old version of a");
    write(&e.root.join("Los Sims 4_b/Tray/common.trayitem"), "new version of b");
    set_age(&e.root.join("Los Sims 4_a/Tray/common.trayitem"), 5000);
    set_age(&e.root.join("Los Sims 4_b/Tray/common.trayitem"), 10);

    enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();
    let report = enable_tray_sync(&e.root, &e.snaps, "b", false).unwrap();

    let shared = shared_tray_dir(&e.root);
    assert!(shared.join("0xaa00.trayitem").is_file());
    assert!(shared.join("0xbb00.trayitem").is_file());
    assert_eq!(read(&shared.join("common.trayitem")), "new version of b");
    assert!(report.updated >= 1);
    assert_eq!(read(&old(&e, "a").join("common.trayitem")), "old version of a");
    assert_eq!(read(&old(&e, "b").join("common.trayitem")), "new version of b");
    assert!(tray(&e, "a").join("0xbb00.trayitem").is_file());
    assert!(tray(&e, "b").join("0xaa00.trayitem").is_file());
}

#[test]
fn an_older_copy_never_overwrites_a_newer_shared_file() {
    let e = env();
    instance(&e, "a", "aa");
    instance(&e, "b", "bb");
    write(&e.root.join("Los Sims 4_a/Tray/common.trayitem"), "reciente");
    write(&e.root.join("Los Sims 4_b/Tray/common.trayitem"), "antigua");
    set_age(&e.root.join("Los Sims 4_a/Tray/common.trayitem"), 10);
    set_age(&e.root.join("Los Sims 4_b/Tray/common.trayitem"), 9000);

    enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();
    enable_tray_sync(&e.root, &e.snaps, "b", false).unwrap();

    assert_eq!(read(&shared_tray_dir(&e.root).join("common.trayitem")), "reciente");
    assert_eq!(read(&old(&e, "b").join("common.trayitem")), "antigua");
}

#[test]
fn enabling_is_idempotent_and_keeps_an_existing_tray_old() {
    let e = env();
    instance(&e, "a", "aa");
    enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();
    let again = enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();
    assert_eq!(again.copied, 0);
    assert_eq!(read(&old(&e, "a").join("0xaa00.hhi")), "aa-hhi");

    disable_tray_sync(&e.root, "a", TrayDisable::Empty, false).unwrap();
    write(&tray(&e, "a").join("recent.trayitem"), "new library");
    enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();

    assert_eq!(read(&old(&e, "a").join("recent.trayitem")), "new library");
    assert_eq!(read(&e.root.join("Los Sims 4_a/TrayOld_2/0xaa00.hhi")), "aa-hhi");
}

#[test]
fn restore_brings_the_original_tray_back_and_leaves_the_shared_one_alone() {
    let e = env();
    instance(&e, "a", "aa");
    instance(&e, "b", "bb");
    enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();
    enable_tray_sync(&e.root, &e.snaps, "b", false).unwrap();
    write(&tray(&e, "a").join("during.trayitem"), "only in the sync");

    disable_tray_sync(&e.root, "a", TrayDisable::Restore, false).unwrap();

    let a = summary(&e, "a");
    assert!(!a.tray_synced && !a.tray_old);
    assert_eq!(read(&tray(&e, "a").join("0xaa00.sgi")), "aa-sgi");
    assert!(!tray(&e, "a").join("0xbb00.sgi").exists());
    assert!(!tray(&e, "a").join("during.trayitem").exists());
    assert!(shared_tray_dir(&e.root).join("during.trayitem").is_file());
    assert!(summary(&e, "b").tray_synced);
}

#[test]
fn restore_without_a_backup_is_refused_and_keeps_the_link() {
    let e = env();
    instance(&e, "a", "aa");
    enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();
    disable_tray_sync(&e.root, "a", TrayDisable::Empty, false).unwrap();
    enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();
    fs::remove_dir_all(old(&e, "a")).unwrap();
    fs::remove_dir_all(e.root.join("Los Sims 4_a/TrayOld_2")).ok();

    assert!(disable_tray_sync(&e.root, "a", TrayDisable::Restore, false).is_err());
    assert!(summary(&e, "a").tray_synced);
}

#[test]
fn merge_combines_the_backup_with_the_shared_items_and_keeps_the_backup() {
    let e = env();
    instance(&e, "a", "aa");
    instance(&e, "b", "bb");
    enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();
    enable_tray_sync(&e.root, &e.snaps, "b", false).unwrap();

    disable_tray_sync(&e.root, "a", TrayDisable::Merge, false).unwrap();

    let a = summary(&e, "a");
    assert!(!a.tray_synced && a.tray_old);
    for ext in EXTENSIONS {
        assert!(tray(&e, "a").join(format!("0xaa00.{ext}")).is_file());
        assert!(tray(&e, "a").join(format!("0xbb00.{ext}")).is_file());
    }
    assert_eq!(read(&tray(&e, "a").join("sub/extra.bin")), "anidado");
    assert!(old(&e, "a").join("0xaa00.trayitem").is_file());
    write(&tray(&e, "a").join("local.trayitem"), "ya no se comparte");
    assert!(!shared_tray_dir(&e.root).join("local.trayitem").exists());
}

#[test]
fn empty_and_unlink_leave_a_real_empty_tray() {
    let e = env();
    instance(&e, "a", "aa");
    enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();
    disable_tray_sync(&e.root, "a", TrayDisable::Empty, false).unwrap();
    assert!(tray(&e, "a").is_dir());
    assert_eq!(fs::read_dir(tray(&e, "a")).unwrap().count(), 0);
    assert!(summary(&e, "a").tray_old);
    assert!(shared_tray_dir(&e.root).join("0xaa00.trayitem").is_file());

    write(&e.root.join("Los Sims 4_c/saves/s.save"), "x");
    adopt_instance(&e.root, "c", "C", "", InstanceKind::Solo).unwrap();
    link_new_instance_tray(&e.root, "c").unwrap();
    assert!(summary(&e, "c").tray_synced);
    disable_tray_sync(&e.root, "c", TrayDisable::Empty, false).unwrap();
    assert_eq!(fs::read_dir(tray(&e, "c")).unwrap().count(), 0);
    assert!(!summary(&e, "c").tray_old);
}

#[test]
fn an_instance_without_tray_folder_just_gets_the_link() {
    let e = env();
    write(&e.root.join("Los Sims 4_a/saves/s.save"), "x");
    adopt_instance(&e.root, "a", "A", "", InstanceKind::Solo).unwrap();

    enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();

    let a = summary(&e, "a");
    assert!(a.tray_synced && !a.tray_old);
}

#[test]
fn disabling_when_not_synced_or_for_unregistered_instances_fails() {
    let e = env();
    instance(&e, "a", "aa");
    assert!(disable_tray_sync(&e.root, "a", TrayDisable::Empty, false).is_err());
    assert!(tray(&e, "a").join("0xaa00.trayitem").is_file());

    write(&e.root.join("Los Sims 4_old/Tray/x.trayitem"), "x");
    assert!(enable_tray_sync(&e.root, &e.snaps, "old", false).is_err());
    assert!(e.root.join("Los Sims 4_old/Tray/x.trayitem").is_file());
    assert!(!e.root.join("Los Sims 4_old/TrayOld").exists());
}

#[test]
fn size_ignores_the_shared_tray_but_the_counter_sees_it() {
    let e = env();
    instance(&e, "a", "aa");
    instance(&e, "b", "bb");
    let before = summary(&e, "a").size_bytes;
    enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();
    enable_tray_sync(&e.root, &e.snaps, "b", false).unwrap();
    let a = summary(&e, "a");
    assert_eq!(a.tray_count, 2);
    assert!(a.size_bytes < before);
}

#[test]
fn snapshots_skip_a_linked_tray_and_restore_leaves_it_alone() {
    let e = env();
    instance(&e, "a", "aa");
    enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();
    let snap = create_snapshot(&e.root.join("Los Sims 4_a"), &e.snaps, "a", "x", SnapshotKind::Manual, false).unwrap();
    assert_eq!(snap.items, ["saves"]);

    write(&e.root.join("Los Sims 4_a/saves/Slot_00000001.save"), "roto");
    restore_snapshot(&e.root.join("Los Sims 4_a"), &e.snaps, "a", &snap.id).unwrap();

    assert_eq!(read(&e.root.join("Los Sims 4_a/saves/Slot_00000001.save")), "save");
    assert!(summary(&e, "a").tray_synced);
    assert!(shared_tray_dir(&e.root).join("0xaa00.trayitem").is_file());
}

#[test]
fn deleting_a_synced_instance_never_touches_the_shared_tray() {
    let e = env();
    instance(&e, "a", "aa");
    instance(&e, "b", "bb");
    enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();
    enable_tray_sync(&e.root, &e.snaps, "b", false).unwrap();

    delete_instance(&e.root, &e.snaps, "a", false, &|_, _| {}).unwrap();

    assert!(!e.root.join("Los Sims 4_a").exists());
    let shared = shared_tray_dir(&e.root);
    assert!(shared.join("0xaa00.trayitem").is_file());
    assert!(shared.join("0xbb00.trayitem").is_file());
    assert!(tray(&e, "b").join("0xaa00.trayitem").is_file());
}

#[test]
fn duplicating_a_synced_instance_links_the_copy() {
    let e = env();
    instance(&e, "a", "aa");
    enable_tray_sync(&e.root, &e.snaps, "a", false).unwrap();

    duplicate_instance(&e.root, "a", "Copia", "", "b").unwrap();

    let b = summary(&e, "b");
    assert!(b.tray_synced);
    assert_eq!(read(&tray(&e, "b").join("0xaa00.trayitem")), "aa-trayitem");
    assert_eq!(read(&old(&e, "b").join("0xaa00.trayitem")), "aa-trayitem");
}
