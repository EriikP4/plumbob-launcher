use s4im_engine::{add_music, cache_items, clear_cache, delete_track, list_music};
use std::fs;
use std::path::Path;

fn write(p: &Path, c: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, c).unwrap();
}

#[test]
fn lists_and_clears_only_selected_cache_items() {
    let tmp = tempfile::tempdir().unwrap();
    let d = tmp.path();
    write(&d.join("localthumbcache.package"), "12345");
    write(&d.join("avatarcache.package"), "12");
    write(&d.join("onlinethumbnailcache/a.bin"), "1234567890");
    write(&d.join("Config.log"), "1");
    write(&d.join("Mods/Script Packs/x.log"), "22");
    write(&d.join("Mods/keep.package"), "keep");
    write(&d.join("saves/Slot_00000001.save"), "save");

    let items = cache_items(d);
    let by = |id: &str| items.iter().find(|i| i.id == id).map(|i| (i.bytes, i.files));
    assert_eq!(by("thumbs"), Some((5, 1)));
    assert_eq!(by("onlinethumbs"), Some((10, 1)));
    assert_eq!(by("logs"), Some((3, 2)));
    assert_eq!(by("textures"), None);

    let r = clear_cache(d, &["thumbs".into(), "onlinethumbs".into()], &|_, _| {}).unwrap();
    assert_eq!(r.freed, 15);
    assert!(!d.join("localthumbcache.package").exists());
    assert!(d.join("onlinethumbnailcache").is_dir());
    assert!(d.join("avatarcache.package").exists());
    assert!(d.join("Mods/Script Packs/x.log").exists());

    clear_cache(d, &["logs".into()], &|_, _| {}).unwrap();
    assert!(!d.join("Config.log").exists());
    assert!(!d.join("Mods/Script Packs/x.log").exists());
    assert!(d.join("Mods/keep.package").exists());
    assert!(d.join("saves/Slot_00000001.save").exists());
    assert!(clear_cache(d, &["../x".into()], &|_, _| {}).is_err());
}

#[test]
fn music_add_list_and_guard_delete() {
    let tmp = tempfile::tempdir().unwrap();
    let inst = tmp.path().join("inst");
    let src = tmp.path().join("src");
    write(&src.join("a.mp3"), "aaa");
    write(&src.join("b.txt"), "no");
    fs::create_dir_all(&inst).unwrap();

    assert!(add_music(&inst, "../evil", &[src.join("a.mp3")], &|_, _| {}).is_err());
    assert!(add_music(&inst, "Pop", &[src.join("b.txt")], &|_, _| {}).is_err());
    assert_eq!(add_music(&inst, "Pop", &[src.join("a.mp3")], &|_, _| {}).unwrap(), 1);
    assert_eq!(add_music(&inst, "Pop", &[src.join("a.mp3")], &|_, _| {}).unwrap(), 1);

    let genres = list_music(&inst);
    assert_eq!(genres.len(), 1);
    assert_eq!(genres[0].tracks.len(), 2);

    assert!(delete_track(&inst, &src.join("a.mp3")).is_err());
}

#[test]
fn error_reports_are_found_anywhere_and_numbered_ones_only() {
    let tmp = tempfile::tempdir().unwrap();
    let d = tmp.path();
    write(&d.join("BE-ExceptionReport.html"), "a");
    write(&d.join("BE-ExceptionReport-OLD.html"), "b");
    write(&d.join("BE-PatchScannerReport.html"), "c");
    write(&d.join("lastException_63927027406.txt"), "d");
    write(&d.join("lastUIException_63926656352.txt"), "e");
    write(&d.join("lastExceptionNotes.txt"), "keep");
    write(&d.join("Mods/Script Packs/mc_lastexception.html"), "f");
    write(&d.join("Mods/Script Packs/mc_systeminfo.html"), "g");
    write(&d.join("Mods/Script Packs/mc_settings.cfg"), "keep");

    let items = cache_items(d);
    let errors = items.iter().find(|i| i.id == "errors").unwrap();
    assert_eq!(errors.files, 7);
    clear_cache(d, &["errors".into()], &|_, _| {}).unwrap();
    assert!(!d.join("BE-ExceptionReport.html").exists());
    assert!(!d.join("Mods/Script Packs/mc_systeminfo.html").exists());
    assert!(d.join("lastExceptionNotes.txt").exists());
    assert!(d.join("Mods/Script Packs/mc_settings.cfg").exists());
}
