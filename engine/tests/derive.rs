#![cfg(windows)]

use s4im_engine::*;
use std::fs;
use std::path::Path;

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn base(root: &Path) {
    let dir = root.join("Los Sims 4_base");
    write(&dir.join("saves/Slot_00000001.save"), "save");
    write(&dir.join("Options.ini"), "[options]\r\nmusiclevel = 12\r\n");
    write(&dir.join("GameVersion.txt"), "1.0\n");
    write(&dir.join("Mods/Resource.cfg"), "custom");
    write(&dir.join("Mods/TURBODRIVER_WickedWhims_Main.ts4script"), "ww");
    write(&dir.join("Mods/WickedWhims/data.package"), "wwd");
    write(&dir.join("Mods/BasementalDrugs/core.package"), "bd");
    write(&dir.join("Mods/mc_cmd_center/mccc.ts4script"), "mc");
    write(&dir.join("Mods/z_ES_strings.package"), "tr");
    write(&dir.join("Mods/1_Spanish_strings.package"), "tr2");
    write(&dir.join("Mods/CC/Hair/hair.package"), "hair");
    write(&dir.join("Mods/CC/Clothes/dress.package"), "dress");
    adopt_instance(root, "base", "Base", "", InstanceKind::Solo).unwrap();
}

fn tree(dir: &Path) -> Vec<(String, u64)> {
    fn walk(dir: &Path, out: &mut Vec<(String, u64)>) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push((p.to_string_lossy().into_owned(), fs::metadata(&p).unwrap().len()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

#[test]
fn scan_detects_groups_and_sizes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    base(&root);
    let scan = scan_base(&root, "base").unwrap();
    assert!(scan.has_config && scan.has_mods);
    assert_eq!(scan.config_files, vec!["Options.ini".to_string()]);
    let ids: Vec<&str> = scan.groups.iter().map(|g| g.id.as_str()).collect();
    assert_eq!(ids, vec!["wickedwhims", "basementaldrugs", "mccc", "translations"]);
    let ww = scan.groups.iter().find(|g| g.id == "wickedwhims").unwrap();
    assert_eq!(ww.files, 2);
    let tr = scan.groups.iter().find(|g| g.id == "translations").unwrap();
    assert_eq!(tr.files, 2);
}

#[test]
fn derive_copies_only_the_selection_and_never_touches_the_base() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    base(&root);
    let before = tree(&root.join("Los Sims 4_base"));
    let sel = DeriveSelection { config: true, all_mods: false, groups: vec!["wickedwhims".into(), "translations".into()], paths: vec!["CC/Hair".into()] };
    let mut seen = Vec::new();
    let summary = derive_instance(&root, "base", "Nueva", "d", "nueva", InstanceKind::Solo, &sel, &mut |d, t| seen.push((d, t))).unwrap();
    assert_eq!(summary.id, "nueva");
    let dir = root.join("Los Sims 4_nueva");
    assert!(dir.join("Options.ini").is_file());
    assert!(!dir.join("saves").exists());
    assert!(!dir.join("GameVersion.txt").exists());
    assert!(dir.join("Mods/TURBODRIVER_WickedWhims_Main.ts4script").is_file());
    assert!(dir.join("Mods/WickedWhims/data.package").is_file());
    assert!(dir.join("Mods/z_ES_strings.package").is_file());
    assert!(dir.join("Mods/1_Spanish_strings.package").is_file());
    assert!(dir.join("Mods/CC/Hair/hair.package").is_file());
    assert!(!dir.join("Mods/CC/Clothes").exists());
    assert!(!dir.join("Mods/BasementalDrugs").exists());
    assert_ne!(read(&dir.join("Mods/Resource.cfg")), "custom");
    assert_eq!(seen.last().copied(), Some((6, 6)));
    assert_eq!(tree(&root.join("Los Sims 4_base")), before);
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap()
}

#[test]
fn derive_rejects_escaping_paths_and_leaves_nothing_behind() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    base(&root);
    let sel = DeriveSelection { paths: vec![r"..\saves".into()], ..Default::default() };
    assert!(derive_instance(&root, "base", "N", "", "nueva", InstanceKind::Solo, &sel, &mut |_, _| {}).is_err());
    assert!(!root.join("Los Sims 4_nueva").exists());
    let sel = DeriveSelection { paths: vec!["../saves".into()], ..Default::default() };
    assert!(derive_instance(&root, "base", "N", "", "nueva", InstanceKind::Solo, &sel, &mut |_, _| {}).is_err());
    assert!(!root.join("Los Sims 4_nueva").exists());
}

#[test]
fn mod_dir_listing_is_lazy_and_sorted() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    base(&root);
    let top = list_mod_dir(&root, "base", "").unwrap();
    assert!(top.first().unwrap().is_dir);
    let cc = list_mod_dir(&root, "base", "CC").unwrap();
    assert_eq!(cc.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["Clothes", "Hair"]);
    assert!(list_mod_dir(&root, "base", "..").is_err());
}
