use s4im_engine::*;
use std::fs;
use std::path::{Path, PathBuf};

fn png(path: &Path, w: u32, h: u32) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut img = image::RgbImage::new(w, h);
    for p in img.pixels_mut() {
        *p = image::Rgb([30, 120, 200]);
    }
    img.save(path).unwrap();
}

struct Env {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    imported: PathBuf,
    index: PathBuf,
    thumbs: PathBuf,
    outside: PathBuf,
}

fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    png(&root.join("Los Sims 4_a/Capturas de pantalla/a1.png"), 64, 36);
    png(&root.join("Los Sims 4_a/Capturas de pantalla/a2.png"), 64, 36);
    png(&root.join("Los Sims 4_b/Screenshots/b1.png"), 48, 27);
    fs::create_dir_all(root.join("Los Sims 4_c")).unwrap();
    let outside = tmp.path().join("Descargas");
    png(&outside.join("photo.png"), 80, 45);
    png(&outside.join("other.jpg.png"), 80, 45);
    fs::write(outside.join("notas.txt"), "no es imagen").unwrap();
    fs::write(outside.join("broken.png"), "not a png").unwrap();
    Env {
        imported: tmp.path().join("datos/imported"),
        index: tmp.path().join("datos/index.json"),
        thumbs: tmp.path().join("datos/thumbs"),
        root,
        outside,
        _tmp: tmp,
    }
}

fn reindex(e: &Env) -> usize {
    update_index(&e.root, &e.imported, &e.index, &e.thumbs, &|_, _| {}).unwrap()
}

#[test]
fn imports_into_an_instance_copying_only_real_images() {
    let e = env();
    let sources = vec![
        e.outside.join("photo.png"),
        e.outside.join("notas.txt"),
        e.outside.join("broken.png"),
        e.outside.join("no-existe.png"),
    ];

    let report = import_shots(&e.root, &e.imported, &sources, Some("a")).unwrap();

    assert_eq!((report.done, report.skipped), (1, 3));
    assert!(e.root.join("Los Sims 4_a/Capturas de pantalla/photo.png").is_file());
    assert!(e.outside.join("photo.png").is_file());
    assert_eq!(reindex(&e), 4);
}

#[test]
fn imports_without_instance_go_to_the_imported_category() {
    let e = env();
    let report = import_shots(&e.root, &e.imported, &[e.outside.join("photo.png")], None).unwrap();
    assert_eq!(report.done, 1);
    assert!(e.imported.join("photo.png").is_file());

    reindex(&e);
    let imported = list_shots(&e.index, Some(IMPORTED));
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].instance, IMPORTED);
    assert_eq!(list_shots(&e.index, None).len(), 4);
}

#[test]
fn imports_create_the_screenshot_folder_and_never_overwrite() {
    let e = env();
    import_shots(&e.root, &e.imported, &[e.outside.join("photo.png")], Some("c")).unwrap();
    assert!(e.root.join("Los Sims 4_c/Capturas de pantalla/photo.png").is_file());

    import_shots(&e.root, &e.imported, &[e.outside.join("photo.png")], Some("c")).unwrap();
    import_shots(&e.root, &e.imported, &[e.outside.join("photo.png")], Some("c")).unwrap();
    let dir = e.root.join("Los Sims 4_c/Capturas de pantalla");
    assert!(dir.join("photo.png").is_file());
    assert!(dir.join("photo (2).png").is_file());
    assert!(dir.join("photo (3).png").is_file());

    import_shots(&e.root, &e.imported, &[e.outside.join("photo.png")], Some("b")).unwrap();
    assert!(e.root.join("Los Sims 4_b/Screenshots/photo.png").is_file());
}

#[test]
fn import_rejects_unknown_or_unsafe_targets() {
    let e = env();
    let src = [e.outside.join("photo.png")];
    assert!(import_shots(&e.root, &e.imported, &src, Some("zzz")).is_err());
    assert!(import_shots(&e.root, &e.imported, &src, Some("..")).is_err());
    assert!(import_shots(&e.root, &e.imported, &src, Some("a/../b")).is_err());
    assert!(import_shots(&e.root, &e.imported, &src, Some("")).is_err());
}

#[test]
fn moves_between_instances_and_to_imported_and_back() {
    let e = env();
    reindex(&e);
    let a1 = e.root.join("Los Sims 4_a/Capturas de pantalla/a1.png");
    let a2 = e.root.join("Los Sims 4_a/Capturas de pantalla/a2.png");

    let to_b = move_shots(&e.root, &e.imported, &[a1.clone()], Some("b")).unwrap();
    assert_eq!((to_b.done, to_b.skipped), (1, 0));
    assert!(!a1.exists());
    assert!(e.root.join("Los Sims 4_b/Screenshots/a1.png").is_file());

    let to_imported = move_shots(&e.root, &e.imported, &[a2.clone()], None).unwrap();
    assert_eq!(to_imported.done, 1);
    assert!(e.imported.join("a2.png").is_file());

    reindex(&e);
    assert_eq!(list_shots(&e.index, Some("a")).len(), 0);
    assert_eq!(list_shots(&e.index, Some("b")).len(), 2);
    assert_eq!(list_shots(&e.index, Some(IMPORTED)).len(), 1);
    assert_eq!(list_shots(&e.index, None).len(), 3);

    let back = move_shots(&e.root, &e.imported, &[e.imported.join("a2.png")], Some("a")).unwrap();
    assert_eq!(back.done, 1);
    assert!(e.root.join("Los Sims 4_a/Capturas de pantalla/a2.png").is_file());
}

#[test]
fn moving_into_the_same_folder_or_with_name_clash_is_safe() {
    let e = env();
    let a1 = e.root.join("Los Sims 4_a/Capturas de pantalla/a1.png");
    let same = move_shots(&e.root, &e.imported, &[a1.clone()], Some("a")).unwrap();
    assert_eq!((same.done, same.skipped), (0, 1));
    assert!(a1.is_file());

    png(&e.root.join("Los Sims 4_b/Screenshots/a1.png"), 10, 10);
    let clash = move_shots(&e.root, &e.imported, &[a1.clone()], Some("b")).unwrap();
    assert_eq!(clash.done, 1);
    assert!(e.root.join("Los Sims 4_b/Screenshots/a1.png").is_file());
    assert!(e.root.join("Los Sims 4_b/Screenshots/a1 (2).png").is_file());
    assert_eq!(image::image_dimensions(e.root.join("Los Sims 4_b/Screenshots/a1.png")).unwrap(), (10, 10));
}

#[test]
fn move_refuses_files_outside_the_known_screenshot_folders() {
    let e = env();
    let stranger = e.outside.join("photo.png");
    let game_file = e.root.join("Los Sims 4_a/Mods/dentro.png");
    png(&game_file, 8, 8);
    let doc = e.root.join("Los Sims 4_a/Options.ini");
    fs::write(&doc, "x").unwrap();

    let report = move_shots(
        &e.root,
        &e.imported,
        &[stranger.clone(), game_file.clone(), doc.clone(), e.root.join("../EA/Los Sims 4_a/saves/x.png")],
        Some("b"),
    )
    .unwrap();

    assert_eq!((report.done, report.skipped), (0, 4));
    assert!(stranger.is_file() && game_file.is_file() && doc.is_file());
    assert!(!e.root.join("Los Sims 4_b/Screenshots/photo.png").exists());
}

#[test]
fn move_to_an_invalid_target_changes_nothing() {
    let e = env();
    let a1 = e.root.join("Los Sims 4_a/Capturas de pantalla/a1.png");
    assert!(move_shots(&e.root, &e.imported, &[a1.clone()], Some("nope")).is_err());
    assert!(a1.is_file());
}

#[test]
fn delete_only_removes_screenshots_inside_known_folders() {
    let e = env();
    let a1 = e.root.join("Los Sims 4_a/Capturas de pantalla/a1.png");
    let b1 = e.root.join("Los Sims 4_b/Screenshots/b1.png");
    import_shots(&e.root, &e.imported, &[e.outside.join("photo.png")], None).unwrap();
    let imported = e.imported.join("photo.png");
    let stranger = e.outside.join("other.jpg.png");
    let mod_png = e.root.join("Los Sims 4_a/Mods/x.png");
    png(&mod_png, 8, 8);
    let removed = std::cell::RefCell::new(Vec::new());
    let remove = |p: &Path| {
        removed.borrow_mut().push(p.to_path_buf());
        fs::remove_file(p).is_ok()
    };

    let report = delete_shots_with(
        &e.root,
        &e.imported,
        &[a1.clone(), b1.clone(), imported.clone(), stranger.clone(), mod_png.clone(), e.outside.join("notas.txt")],
        &remove,
    );

    assert_eq!((report.done, report.skipped), (3, 3));
    assert!(!a1.exists() && !b1.exists() && !imported.exists());
    assert!(stranger.is_file() && mod_png.is_file() && e.outside.join("notas.txt").is_file());
    assert_eq!(removed.borrow().len(), 3);
    assert!(e.root.join("Los Sims 4_a/Capturas de pantalla/a2.png").is_file());
}

#[test]
fn delete_counts_failed_removals_as_skipped_and_the_index_follows() {
    let e = env();
    reindex(&e);
    let a1 = e.root.join("Los Sims 4_a/Capturas de pantalla/a1.png");
    let a2 = e.root.join("Los Sims 4_a/Capturas de pantalla/a2.png");

    let report = delete_shots_with(&e.root, &e.imported, &[a1.clone(), a2.clone()], &|p| p.ends_with("a1.png") && fs::remove_file(p).is_ok());

    assert_eq!((report.done, report.skipped), (1, 1));
    assert!(a2.is_file());
    reindex(&e);
    let left = list_shots(&e.index, Some("a"));
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].name, "a2.png");
    assert_eq!(fs::read_dir(&e.thumbs).unwrap().count(), 2);
}
