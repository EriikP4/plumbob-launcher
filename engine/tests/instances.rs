use s4im_engine::*;
use std::fs;
use std::path::Path;

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn make_instance(root: &Path, folder: &str) {
    let dir = root.join(folder);
    write(&dir.join("Mods").join("a.package"), "aaaa");
    write(&dir.join("Mods").join("sub").join("b.ts4script"), "bb");
    write(&dir.join("Mods").join("readme.txt"), "x");
    write(&dir.join("saves").join("Slot_00000001.save"), "save1");
    write(&dir.join("saves").join("Slot_00000001.save.ver0"), "old");
    write(&dir.join("Tray").join("house.trayitem"), "tray");
    write(&dir.join("Options.ini"), "opt");
}

#[test]
fn lists_instances_and_ignores_other_folders() {
    let tmp = tempfile::tempdir().unwrap();
    make_instance(tmp.path(), "Los Sims 4_solo");
    make_instance(tmp.path(), "Los Sims 4_mp1");
    fs::create_dir_all(tmp.path().join("Sync")).unwrap();
    fs::create_dir_all(tmp.path().join("Los Sims 4")).unwrap();
    write(&tmp.path().join("Los Sims 4_file"), "not a dir");

    let list = list_instances(tmp.path()).unwrap();
    let ids: Vec<&str> = list.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(ids, ["mp1", "solo"]);
    assert_eq!(list[0].kind, InstanceKind::Mp);
    assert_eq!(list[1].kind, InstanceKind::Solo);
    assert!(list.iter().all(|i| i.legacy));
    assert_eq!(list[1].mod_count, 2);
    assert_eq!(list[1].save_count, 1);
    assert_eq!(list[1].tray_count, 1);
    assert_eq!(list[1].size_bytes, 4 + 2 + 1 + 5 + 3 + 4);
    assert_eq!(list[1].top_mods, ["a.package", "readme.txt", "sub"]);
}

#[test]
fn missing_game_root_gives_empty_list() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(list_instances(&tmp.path().join("nope")).unwrap().is_empty());
}

#[test]
fn metadata_gives_name_and_clears_legacy_flag() {
    let game = tempfile::tempdir().unwrap();
    make_instance(game.path(), "Los Sims 4_solo");
    make_instance(game.path(), "Los Sims 4_rl");
    write(
        &game.path().join("Los Sims 4_solo").join(".plumbob.json"),
        r#"{"name":"Solo","kind":"solo","description":"Primary"}"#,
    );

    let list = list_instances(game.path()).unwrap();
    assert_eq!(list.len(), 2);
    let solo = list.iter().find(|i| i.id == "solo").unwrap();
    assert_eq!(solo.name, "Solo");
    assert_eq!(solo.description, "Primary");
    assert!(!solo.legacy);
    assert!(list.iter().find(|i| i.id == "rl").unwrap().legacy);
}

#[test]
fn listing_never_modifies_the_source() {
    let tmp = tempfile::tempdir().unwrap();
    make_instance(tmp.path(), "Los Sims 4_solo");
    let before = fs::read(tmp.path().join("Los Sims 4_solo/saves/Slot_00000001.save")).unwrap();
    list_instances(tmp.path()).unwrap();
    let after = fs::read(tmp.path().join("Los Sims 4_solo/saves/Slot_00000001.save")).unwrap();
    assert_eq!(before, after);
}

#[test]
fn active_state_reports_missing_and_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let game = tmp.path().join("Los Sims 4");
    assert_eq!(active_state(&game), ActiveState::Missing);
    fs::create_dir_all(&game).unwrap();
    assert_eq!(active_state(&game), ActiveState::Folder);
}
