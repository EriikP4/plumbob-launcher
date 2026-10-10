#![cfg(windows)]

use s4im_engine::*;
use std::fs;
use std::path::Path;

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn setup(root: &Path, snaps: &Path) {
    write(&root.join("Los Sims 4/saves/Slot_00000001.save"), "main-save");
    write(&root.join("Los Sims 4/Mods/a.package"), "m");
    write(&root.join("Los Sims 4/Options.ini"), "opt");
    register_main(root, snaps, "Primary", "", "Main").unwrap();
    create_instance(root, "Other", "desc", "Alt", InstanceKind::Solo).unwrap();
    let alt = instance_dir(root, "Alt");
    write(&alt.join("Mods/b.package"), "bbbb");
    write(&alt.join("saves/Slot_00000001.save"), "alt-save");
    write(&alt.join("localthumbcache.package"), "thumbs");
    write(&alt.join("Mods/empty/.keep"), "");
    fs::create_dir_all(alt.join("Mods/emptydir")).unwrap();
}

#[test]
fn compressed_archive_round_trips_and_clears_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Electronic Arts");
    let snaps = tmp.path().join("snaps");
    setup(&root, &snaps);

    let record = archive_instance(&root, "Alt", true, true, &|_, _| {}).unwrap();
    assert!(record.compressed);
    assert!(!instance_dir(&root, "Alt").exists());
    assert!(root.join("Archived/Alt.zip").is_file());
    let listed = list_archived(&root);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].summary.name, "Other");
    assert!(list_instances(&root).unwrap().iter().all(|i| i.id != "Alt"));

    restore_archived(&root, "Alt", &|_, _| {}).unwrap();
    let alt = instance_dir(&root, "Alt");
    assert_eq!(fs::read_to_string(alt.join("Mods/b.package")).unwrap(), "bbbb");
    assert_eq!(fs::read_to_string(alt.join("saves/Slot_00000001.save")).unwrap(), "alt-save");
    assert!(alt.join("Mods/emptydir").is_dir());
    assert!(!alt.join("localthumbcache.package").exists());
    assert!(list_archived(&root).is_empty());
    assert!(!root.join("Archived/Alt.zip").exists());
}

#[test]
fn plain_archive_moves_the_folder_and_restores_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Electronic Arts");
    let snaps = tmp.path().join("snaps");
    setup(&root, &snaps);

    let record = archive_instance(&root, "Alt", false, false, &|_, _| {}).unwrap();
    assert!(!record.compressed);
    assert!(root.join("Archived/Alt/localthumbcache.package").is_file());
    assert_eq!(list_archived(&root).len(), 1);

    restore_archived(&root, "Alt", &|_, _| {}).unwrap();
    assert!(instance_dir(&root, "Alt").join("localthumbcache.package").is_file());
    assert!(!root.join("Archived/Alt").exists());
}

#[test]
fn active_instance_cannot_be_archived_and_codes_must_be_free_to_restore() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Electronic Arts");
    let snaps = tmp.path().join("snaps");
    setup(&root, &snaps);

    assert!(archive_instance(&root, "Main", false, false, &|_, _| {}).is_err());
    assert!(instance_dir(&root, "Main").is_dir());

    archive_instance(&root, "Alt", false, true, &|_, _| {}).unwrap();
    create_instance(&root, "New", "", "Alt", InstanceKind::Solo).unwrap();
    assert!(restore_archived(&root, "Alt", &|_, _| {}).is_err());
    assert_eq!(list_archived(&root).len(), 1);
}

#[test]
fn diagnostics_report_a_clean_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Electronic Arts");
    write(&root.join("Los Sims 4/Mods/a.package"), "m");
    let d = diagnose(&root, &root.join("Los Sims 4"));
    assert!(d.game_dir_exists);
    assert_eq!(d.cloud_files, 0);
    assert_eq!(d.locked_files, 0);
    assert!(d.write_error.is_none());
}

#[test]
fn diagnostics_find_files_held_without_delete_sharing() {
    use std::os::windows::fs::OpenOptionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Electronic Arts");
    let file = root.join("Los Sims 4/Mods/a.package");
    write(&file, "m");
    let _held = fs::OpenOptions::new().read(true).share_mode(3).open(&file).unwrap();
    let d = diagnose(&root, &root.join("Los Sims 4"));
    assert_eq!(d.locked_files, 1);
    assert_eq!(d.locked_sample.as_deref(), Some(file.as_path()));
}

#[test]
fn restoring_a_compressed_instance_relinks_the_shared_tray() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Electronic Arts");
    let snaps = tmp.path().join("snaps");
    setup(&root, &snaps);
    write(&instance_dir(&root, "Alt").join("Tray/a.trayitem"), "t");
    enable_tray_sync(&root, &snaps, "Alt", false).unwrap();
    assert!(tray_is_synced(&root, &instance_dir(&root, "Alt")));

    archive_instance(&root, "Alt", false, true, &|_, _| {}).unwrap();
    assert!(root.join("SharedTray/a.trayitem").is_file());
    restore_archived(&root, "Alt", &|_, _| {}).unwrap();

    let alt = instance_dir(&root, "Alt");
    assert!(tray_is_synced(&root, &alt));
    assert!(alt.join("TrayOld/a.trayitem").is_file());
    assert!(root.join("SharedTray/a.trayitem").is_file());
}

#[test]
fn store_cache_folder_is_listed_and_cleared() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("inst");
    write(&dir.join("cachestr/spotlight_en-us.package"), "spot");
    let items = cache_items(&dir);
    let item = items.iter().find(|i| i.id == "cachestr").unwrap();
    assert_eq!(item.files, 1);
    let report = clear_cache(&dir, &["cachestr".to_string()], &|_, _| {}).unwrap();
    assert_eq!(report.failed, 0);
    assert!(!dir.join("cachestr/spotlight_en-us.package").exists());
}

#[test]
fn zip_uses_plain_headers_and_original_size_covers_the_whole_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Electronic Arts");
    let snaps = tmp.path().join("snaps");
    setup(&root, &snaps);
    let alt = instance_dir(&root, "Alt");
    write(&alt.join("Screenshots/shot.png"), &"x".repeat(5000));
    write(&alt.join("Options.ini"), &"y".repeat(3000));

    let record = archive_instance(&root, "Alt", true, true, &|_, _| {}).unwrap();
    assert!(record.summary.size_bytes >= 8000);

    let bytes = fs::read(root.join("Archived/Alt.zip")).unwrap();
    let mut at = 0;
    let mut seen = 0;
    while at + 30 <= bytes.len() && &bytes[at..at + 4] == b"PK\x03\x04" {
        let name_len = u16::from_le_bytes([bytes[at + 26], bytes[at + 27]]) as usize;
        let extra_len = u16::from_le_bytes([bytes[at + 28], bytes[at + 29]]) as usize;
        assert_eq!(extra_len, 0);
        let packed = u32::from_le_bytes([bytes[at + 18], bytes[at + 19], bytes[at + 20], bytes[at + 21]]) as usize;
        at += 30 + name_len + extra_len + packed;
        seen += 1;
    }
    assert!(seen >= 3);
}
