use s4im_engine::*;
use std::fs;
use std::path::Path;

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn groups_slot_files_and_ignores_the_rest() {
    let tmp = tempfile::tempdir().unwrap();
    let saves = tmp.path().join("saves");
    write(&saves.join("Slot_00000001.save"), "aaaa");
    write(&saves.join("Slot_00000001.save.ver0"), "bb");
    write(&saves.join("Slot_00000001.save.day.ver1"), "c");
    write(&saves.join("Slot_00000002.save"), "dddddd");
    write(&saves.join("Slot_00000003.save.ver0"), "huerfano");
    write(&saves.join("notes.txt"), "x");
    write(&saves.join("Slot_abc.save"), "x");
    write(&saves.join("BasementalDrugs").join("Slot_00000009.save"), "x");

    let mut list = list_saves(tmp.path(), None);
    list.sort_by_key(|s| s.slot);

    assert_eq!(list.len(), 2);
    assert_eq!((list[0].slot, list[0].size_bytes, list[0].versions), (1, 7, 2));
    assert_eq!((list[1].slot, list[1].size_bytes, list[1].versions), (2, 6, 0));
    assert!(list.iter().all(|s| s.modified > 1_700_000_000));
}

#[test]
fn newest_first_and_missing_folder_is_empty() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(list_saves(tmp.path(), None).is_empty());
    assert!(list_saves(&tmp.path().join("nada"), None).is_empty());

    let saves = tmp.path().join("Saves");
    write(&saves.join("Slot_00000001.save"), "old");
    write(&saves.join("Slot_00000002.save"), "new");
    let old = fs::File::options().write(true).open(saves.join("Slot_00000001.save")).unwrap();
    old.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(86_400)).unwrap();

    let list = list_saves(tmp.path(), None);
    assert_eq!(list.iter().map(|s| s.slot).collect::<Vec<_>>(), [2, 1]);
}
