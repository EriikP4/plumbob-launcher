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

fn game_data(dir: &Path) {
    write(&dir.join("saves/Slot_00000001.save"), "save1");
    write(&dir.join("saves/nested/x.bin"), "n");
    write(&dir.join("Mods/a.package"), "mod");
    write(&dir.join("Options.ini"), "opt");
}

#[test]
fn copy_keeps_the_original_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let other = tmp.path().join("Mis Sims");
    game_data(&other);

    let imported = import_instance(&root, &other, ImportMode::Copy, "Copia", "d", "cp", InstanceKind::Solo).unwrap();

    assert_eq!(imported.id, "cp");
    assert_eq!(imported.name, "Copia");
    assert!(!imported.legacy);
    assert_eq!(read(&root.join("Los Sims 4_cp/saves/Slot_00000001.save")), "save1");
    assert_eq!(read(&root.join("Los Sims 4_cp/saves/nested/x.bin")), "n");
    assert!(root.join("Los Sims 4_cp/.plumbob.json").is_file());
    assert_eq!(read(&other.join("saves/Slot_00000001.save")), "save1");
    assert!(!other.join(".plumbob.json").exists());
    assert!(!root.join(".import-tmp-cp").exists());
}

#[test]
fn move_relocates_the_folder_and_removes_the_source() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let other = tmp.path().join("Otro sitio").join("Sims");
    game_data(&other);

    let imported = import_instance(&root, &other, ImportMode::Move, "Movida", "", "mv", InstanceKind::Mp).unwrap();

    assert_eq!(imported.kind, InstanceKind::Mp);
    assert_eq!(imported.save_count, 1);
    assert_eq!(imported.mod_count, 1);
    assert_eq!(read(&root.join("Los Sims 4_mv/Options.ini")), "opt");
    assert!(!other.exists());
}

#[test]
fn a_folder_already_named_like_an_instance_is_adopted_in_place() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    game_data(&root.join("Los Sims 4_old"));

    let adopted = import_instance(&root, &root.join("Los Sims 4_old"), ImportMode::Move, "Vieja", "", "ignorado", InstanceKind::Solo).unwrap();

    assert_eq!(adopted.id, "old");
    assert!(!adopted.legacy);
    assert_eq!(read(&root.join("Los Sims 4_old/saves/Slot_00000001.save")), "save1");
}

#[test]
fn unsafe_or_unrelated_sources_are_refused_and_left_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    fs::create_dir_all(&root).unwrap();
    let docs = tmp.path().join("Documentos");
    write(&docs.join("tesis.docx"), "importante");
    let nested = root.join("folder");
    game_data(&nested);
    let ok = tmp.path().join("ok");
    game_data(&ok);

    let attempt = |src: &Path, code: &str| import_instance(&root, src, ImportMode::Move, "X", "", code, InstanceKind::Solo);

    assert!(attempt(&docs, "d1").is_err());
    assert!(docs.join("tesis.docx").exists());
    assert!(attempt(&root, "d2").is_err());
    assert!(attempt(tmp.path(), "d3").is_err());
    assert!(attempt(&nested, "d4").is_err());
    assert!(nested.join("saves").exists());
    assert!(attempt(&tmp.path().join("no existe"), "d5").is_err());
    assert!(attempt(&ok, "mal codigo").is_err());
    assert!(attempt(&ok, "").is_err());
    assert!(ok.join("saves").exists());
    assert!(!root.join("Los Sims 4_d1").exists());
}

#[test]
fn duplicate_codes_are_refused_and_the_source_survives() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    game_data(&tmp.path().join("a"));
    game_data(&tmp.path().join("b"));
    import_instance(&root, &tmp.path().join("a"), ImportMode::Copy, "A", "", "dup", InstanceKind::Solo).unwrap();

    assert!(import_instance(&root, &tmp.path().join("b"), ImportMode::Move, "B", "", "DUP", InstanceKind::Solo).is_err());
    assert!(tmp.path().join("b/saves").exists());
}

#[test]
fn a_junction_source_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let real = tmp.path().join("real");
    game_data(&real);
    let link = tmp.path().join("enlace");
    create_junction(&link, &real).unwrap();

    assert!(import_instance(&root, &link, ImportMode::Move, "X", "", "lk", InstanceKind::Solo).is_err());
    assert!(real.join("saves").exists());
    remove_junction(&link).unwrap();
}

#[test]
fn imported_instances_can_be_activated() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let snaps = tmp.path().join("snaps");
    game_data(&tmp.path().join("x"));
    import_instance(&root, &tmp.path().join("x"), ImportMode::Copy, "X", "", "xx", InstanceKind::Solo).unwrap();

    activate_instance(&root, &snaps, "xx", false).unwrap();

    assert_eq!(active_instance_id(&root).as_deref(), Some("xx"));
    assert_eq!(read(&root.join("Los Sims 4/saves/Slot_00000001.save")), "save1");
}
