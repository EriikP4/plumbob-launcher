#![cfg(windows)]

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

fn instance(root: &Path, code: &str, kind: InstanceKind) {
    let dir = root.join(format!("Los Sims 4_{code}"));
    write(&dir.join("saves/Slot_00000001.save"), "save");
    write(&dir.join("Mods/mimod.package"), "mod");
    write(&dir.join("GameVersion.txt"), "1.128.90.1030\n");
    adopt_instance(root, code, "Nombre", "Desc", kind).unwrap();
}

fn s4mp_files(root: &Path, code: &str) {
    let mods = root.join(format!("Los Sims 4_{code}")).join("Mods");
    write(&mods.join("S4MP_logs/log.txt"), "l");
    write(&mods.join("S4MP_native/n.dll"), "n");
    write(&mods.join("S4MP_release.ts4script"), "r");
    write(&mods.join("s4mpconfig"), "c");
}

#[test]
fn update_changes_name_and_description_only() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    instance(&root, "a", InstanceKind::Solo);

    let updated = update_instance(&root, "a", "  New name ", " other description ").unwrap();

    assert_eq!(updated.name, "New name");
    assert_eq!(updated.description, "other description");
    assert_eq!(updated.kind, InstanceKind::Solo);
    assert_eq!(read(&root.join("Los Sims 4_a/saves/Slot_00000001.save")), "save");
    assert!(update_instance(&root, "a", "   ", "").is_err());
    assert!(update_instance(&root, "zzz", "x", "").is_err());
}

#[test]
fn update_refuses_unregistered_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    write(&root.join("Los Sims 4_old/saves/s.save"), "x");
    assert!(update_instance(&root, "old", "Nombre", "").is_err());
    assert!(!root.join("Los Sims 4_old/.plumbob.json").exists());
}

#[test]
fn converting_to_multiplayer_keeps_everything() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    instance(&root, "a", InstanceKind::Solo);

    let converted = convert_instance(&root, "a", InstanceKind::Mp, false).unwrap();

    assert_eq!(converted.kind, InstanceKind::Mp);
    assert_eq!(read(&root.join("Los Sims 4_a/Mods/mimod.package")), "mod");
    assert_eq!(read(&root.join("Los Sims 4_a/saves/Slot_00000001.save")), "save");
}

#[test]
fn converting_to_single_player_can_remove_only_the_s4mp_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    instance(&root, "a", InstanceKind::Mp);
    s4mp_files(&root, "a");
    write(&root.join("Los Sims 4_a/Mods/OtroMod/x.package"), "otro");

    let mut found = s4mp_leftovers(&root, "a");
    found.sort();
    assert_eq!(found, ["S4MP_logs", "S4MP_native", "S4MP_release.ts4script", "s4mpconfig"]);

    let converted = convert_instance(&root, "a", InstanceKind::Solo, true).unwrap();

    assert_eq!(converted.kind, InstanceKind::Solo);
    assert!(s4mp_leftovers(&root, "a").is_empty());
    let mods = root.join("Los Sims 4_a/Mods");
    assert!(!mods.join("S4MP_logs").exists());
    assert!(!mods.join("S4MP_native").exists());
    assert!(!mods.join("S4MP_release.ts4script").exists());
    assert!(!mods.join("s4mpconfig").exists());
    assert_eq!(read(&mods.join("mimod.package")), "mod");
    assert_eq!(read(&mods.join("OtroMod/x.package")), "otro");
    assert_eq!(read(&root.join("Los Sims 4_a/saves/Slot_00000001.save")), "save");
}

#[test]
fn converting_without_deleting_leaves_the_s4mp_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    instance(&root, "a", InstanceKind::Mp);
    s4mp_files(&root, "a");

    let converted = convert_instance(&root, "a", InstanceKind::Solo, false).unwrap();

    assert_eq!(converted.kind, InstanceKind::Solo);
    assert_eq!(s4mp_leftovers(&root, "a").len(), 4);
}

#[test]
fn only_existing_s4mp_files_are_listed() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    instance(&root, "a", InstanceKind::Mp);
    write(&root.join("Los Sims 4_a/Mods/s4mpconfig"), "c");
    write(&root.join("Los Sims 4_a/Mods/sub/S4MP_logs/x"), "anidado");
    assert_eq!(s4mp_leftovers(&root, "a"), ["s4mpconfig"]);
    assert!(s4mp_leftovers(&root, "no-existe").is_empty());
}

#[test]
fn duplicate_copies_content_with_a_new_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    instance(&root, "a", InstanceKind::Mp);
    mark_played(&root, "a", 600).unwrap();

    let copy = duplicate_instance(&root, "a", "Test copy", "to try mods", "b").unwrap();

    assert_eq!(copy.id, "b");
    assert_eq!(copy.name, "Test copy");
    assert_eq!(copy.kind, InstanceKind::Mp);
    assert_eq!(copy.play_seconds, 0);
    assert_eq!(copy.last_played, None);
    assert_eq!(copy.save_count, 1);
    assert_eq!(read(&root.join("Los Sims 4_b/Mods/mimod.package")), "mod");
    let original = list_instances(&root).unwrap().into_iter().find(|i| i.id == "a").unwrap();
    assert_eq!(original.play_seconds, 600);
    assert_eq!(original.name, "Nombre");
    assert!(!root.join(".import-tmp-b").exists());
}

#[test]
fn duplicate_refuses_bad_codes_and_unregistered_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    instance(&root, "a", InstanceKind::Solo);
    write(&root.join("Los Sims 4_old/saves/s.save"), "x");
    assert!(duplicate_instance(&root, "a", "X", "", "A").is_err());
    assert!(duplicate_instance(&root, "a", "X", "", "mal codigo").is_err());
    assert!(duplicate_instance(&root, "old", "X", "", "n1").is_err());
    assert!(!root.join("Los Sims 4_n1").exists());
}

#[test]
fn game_version_is_read_from_the_instance() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    instance(&root, "a", InstanceKind::Solo);
    assert_eq!(list_instances(&root).unwrap()[0].game_version.as_deref(), Some("1.128.90.1030"));
    assert_eq!(read_game_version(&root.join("Los Sims 4_a")).as_deref(), Some("1.128.90.1030"));

    write(&root.join("Los Sims 4_a/GameVersion.txt"), "raro texto");
    assert_eq!(read_game_version(&root.join("Los Sims 4_a")), None);
    fs::remove_file(root.join("Los Sims 4_a/GameVersion.txt")).unwrap();
    assert_eq!(read_game_version(&root.join("Los Sims 4_a")), None);
}
