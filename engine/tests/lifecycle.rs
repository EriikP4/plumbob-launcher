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

fn real_game_folder(root: &Path) {
    write(&root.join("Los Sims 4/saves/Slot_00000001.save"), "main-save");
    write(&root.join("Los Sims 4/Mods/a.package"), "m");
    write(&root.join("Los Sims 4/Options.ini"), "opt");
}

#[test]
fn first_run_registers_main_and_leaves_a_working_junction() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Electronic Arts");
    let snaps = tmp.path().join("snaps");
    real_game_folder(&root);
    assert!(needs_setup(&root));

    let main = register_main(&root, &snaps, "Primary", "", "Main").unwrap();

    assert_eq!(main.id, "Main");
    assert_eq!(main.name, "Primary");
    assert!(!main.legacy);
    assert!(!needs_setup(&root));
    assert_eq!(active_instance_id(&root).as_deref(), Some("Main"));
    assert_eq!(read(&root.join("Los Sims 4_Main/saves/Slot_00000001.save")), "main-save");
    assert_eq!(read(&root.join("Los Sims 4/saves/Slot_00000001.save")), "main-save");
    assert_eq!(read(&root.join("Los Sims 4_Main/Options.ini")), "opt");
    assert_eq!(list_snapshots(&snaps, "Main").unwrap().len(), 1);
}

#[test]
fn register_refuses_without_real_folder_or_with_bad_code() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let snaps = tmp.path().join("snaps");
    fs::create_dir_all(&root).unwrap();
    assert!(register_main(&root, &snaps, "P", "", "Main").is_err());
    real_game_folder(&root);
    assert!(register_main(&root, &snaps, "P", "", "not valid!").is_err());
    assert!(register_main(&root, &snaps, "P", "", "").is_err());
    assert!(register_main(&root, &snaps, "  ", "", "Main").is_err());
    assert!(root.join("Los Sims 4/saves").is_dir());
    assert!(!root.join("Los Sims 4/.plumbob.json").exists());
}

#[test]
fn create_then_switch_moves_only_the_link() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let snaps = tmp.path().join("snaps");
    real_game_folder(&root);
    register_main(&root, &snaps, "Primary", "", "Main").unwrap();

    let test = create_instance(&root, "Test Instance", "", "test", InstanceKind::Solo).unwrap();
    assert_eq!(test.id, "test");
    assert!(root.join("Los Sims 4_test").is_dir());
    assert_eq!(active_instance_id(&root).as_deref(), Some("Main"));

    activate_instance(&root, &snaps, "test", false).unwrap();
    assert_eq!(active_instance_id(&root).as_deref(), Some("test"));
    write(&root.join("Los Sims 4/saves/new.save"), "test-save");
    assert_eq!(read(&root.join("Los Sims 4_test/saves/new.save")), "test-save");
    assert_eq!(read(&root.join("Los Sims 4_Main/saves/Slot_00000001.save")), "main-save");
    assert!(!root.join("Los Sims 4_Main/saves/new.save").exists());

    activate_instance(&root, &snaps, "Main", false).unwrap();
    assert_eq!(active_instance_id(&root).as_deref(), Some("Main"));
    assert_eq!(read(&root.join("Los Sims 4_test/saves/new.save")), "test-save");
    assert_eq!(list_instances(&root).unwrap().len(), 2);
}

#[test]
fn duplicate_codes_are_rejected_ignoring_case() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let snaps = tmp.path().join("snaps");
    real_game_folder(&root);
    register_main(&root, &snaps, "Primary", "", "Main").unwrap();
    assert!(create_instance(&root, "Other", "", "main", InstanceKind::Solo).is_err());
    create_instance(&root, "Una", "", "x1", InstanceKind::Mp).unwrap();
    assert!(create_instance(&root, "Dos", "", "X1", InstanceKind::Mp).is_err());
}

#[test]
fn adopting_an_existing_folder_keeps_its_content() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let snaps = tmp.path().join("snaps");
    write(&root.join("Los Sims 4_mp1/saves/s.save"), "mp");
    assert!(!needs_setup(&root));
    assert!(list_instances(&root).unwrap()[0].legacy);

    let adopted = adopt_instance(&root, "mp1", "Cheesies", "S4MP", InstanceKind::Mp).unwrap();
    assert_eq!(adopted.name, "Cheesies");
    assert!(!adopted.legacy);
    assert!(adopt_instance(&root, "mp1", "x", "", InstanceKind::Mp).is_err());
    assert_eq!(read(&root.join("Los Sims 4_mp1/saves/s.save")), "mp");

    activate_instance(&root, &snaps, "mp1", false).unwrap();
    assert_eq!(active_instance_id(&root).as_deref(), Some("mp1"));
    assert_eq!(read(&root.join("Los Sims 4/saves/s.save")), "mp");
}

#[test]
fn activation_refuses_unregistered_instances_and_real_game_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let snaps = tmp.path().join("snaps");
    write(&root.join("Los Sims 4_old/saves/s.save"), "old");
    assert!(activate_instance(&root, &snaps, "old", false).is_err());
    adopt_instance(&root, "old", "Vieja", "", InstanceKind::Solo).unwrap();
    real_game_folder(&root);
    assert!(activate_instance(&root, &snaps, "old", false).is_err());
    assert_eq!(read(&root.join("Los Sims 4/saves/Slot_00000001.save")), "main-save");
    assert!(activate_instance(&root, &snaps, "nope", false).is_err());
}

#[test]
fn activation_with_no_current_link_creates_it() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let snaps = tmp.path().join("snaps");
    write(&root.join("Los Sims 4_solo/saves/s.save"), "solo");
    adopt_instance(&root, "solo", "Solo", "", InstanceKind::Solo).unwrap();
    assert_eq!(active_instance_id(&root), None);
    activate_instance(&root, &snaps, "solo", false).unwrap();
    assert_eq!(active_instance_id(&root).as_deref(), Some("solo"));
    activate_instance(&root, &snaps, "solo", false).unwrap();
    assert_eq!(list_snapshots(&snaps, "solo").unwrap().len(), 1);
}

#[test]
fn new_instances_get_the_standard_mods_resource_file() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    create_instance(&root, "Fresh", "", "fresh", InstanceKind::Solo).unwrap();
    let cfg = read(&instance_dir(&root, "fresh").join("Mods/Resource.cfg"));
    assert!(cfg.starts_with("Priority 500\n"));
    assert_eq!(cfg.matches("PackedFile").count(), 6);
}
