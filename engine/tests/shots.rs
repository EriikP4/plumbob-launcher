use s4im_engine::*;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

fn png(path: &Path, w: u32, h: u32, shade: u8) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut img = image::RgbImage::new(w, h);
    for p in img.pixels_mut() {
        *p = image::Rgb([shade, 120, 200]);
    }
    img.save(path).unwrap();
}

fn setup() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    png(&root.join("Los Sims 4_solo/Capturas de pantalla/a.png"), 1600, 900, 10);
    png(&root.join("Los Sims 4_solo/Capturas de pantalla/sub/b.png"), 800, 800, 50);
    png(&root.join("Los Sims 4_mp1/Screenshots/c.png"), 640, 360, 90);
    fs::write(root.join("Los Sims 4_mp1/Screenshots/notas.txt"), "no es imagen").unwrap();
    fs::create_dir_all(root.join("Los Sims 4_vacia")).unwrap();
    let index = tmp.path().join("datos/index.json");
    let thumbs = tmp.path().join("datos/thumbs");
    let imported = tmp.path().join("datos/imported");
    (tmp, root, imported, index, thumbs)
}

fn no_progress(_: usize, _: usize) {}

#[test]
fn indexes_all_instances_and_builds_small_thumbnails() {
    let (_t, root, imported, index, thumbs) = setup();
    assert!(!index_exists(&index));

    let count = update_index(&root, &imported, &index, &thumbs, &no_progress).unwrap();

    assert_eq!(count, 3);
    assert!(index_exists(&index));
    let all = list_shots(&index, None);
    assert_eq!(all.len(), 3);
    let a = all.iter().find(|s| s.name == "a.png").unwrap();
    assert_eq!((a.width, a.height, a.instance.as_str()), (1600, 900, "solo"));
    let thumb = a.thumb.as_ref().unwrap();
    assert!(thumb.is_file());
    let small = image::open(thumb).unwrap();
    assert!(small.width() <= 480 && small.height() <= 300);
    assert!(a.size > 0);
    assert!(fs::metadata(thumb).unwrap().len() < a.size);
}

#[test]
fn filters_by_instance_and_missing_index_is_empty() {
    let (_t, root, imported, index, thumbs) = setup();
    assert!(list_shots(&index, None).is_empty());
    update_index(&root, &imported, &index, &thumbs, &no_progress).unwrap();
    assert_eq!(list_shots(&index, Some("solo")).len(), 2);
    assert_eq!(list_shots(&index, Some("MP1")).len(), 1);
    assert!(list_shots(&index, Some("vacia")).is_empty());
}

#[test]
fn rerun_only_processes_new_files_and_drops_deleted_ones() {
    let (_t, root, imported, index, thumbs) = setup();
    update_index(&root, &imported, &index, &thumbs, &no_progress).unwrap();
    let before: Vec<_> = list_shots(&index, None).into_iter().map(|s| s.thumb.unwrap()).collect();
    let stamps: Vec<_> = before.iter().map(|t| fs::metadata(t).unwrap().modified().unwrap()).collect();

    png(&root.join("Los Sims 4_solo/Capturas de pantalla/new.png"), 400, 300, 200);
    fs::remove_file(root.join("Los Sims 4_mp1/Screenshots/c.png")).unwrap();
    let counter = AtomicUsize::new(0);
    let tracker = |done: usize, _total: usize| {
        counter.store(done, Ordering::SeqCst);
    };
    let count = update_index(&root, &imported, &index, &thumbs, &tracker).unwrap();

    assert_eq!(count, 3);
    assert_eq!(counter.load(Ordering::SeqCst), 3);
    let names: Vec<_> = list_shots(&index, None).into_iter().map(|s| s.name).collect();
    assert!(names.contains(&"new.png".to_string()));
    assert!(!names.contains(&"c.png".to_string()));
    for (t, stamp) in before.iter().zip(stamps) {
        if t.is_file() {
            assert_eq!(fs::metadata(t).unwrap().modified().unwrap(), stamp);
        }
    }
    assert_eq!(fs::read_dir(&thumbs).unwrap().count(), 3);
}

#[test]
fn broken_images_are_listed_without_thumbnail_and_never_fail_the_index() {
    let (_t, root, imported, index, thumbs) = setup();
    fs::write(root.join("Los Sims 4_solo/Capturas de pantalla/broken.png"), "not a png").unwrap();

    let count = update_index(&root, &imported, &index, &thumbs, &no_progress).unwrap();

    assert_eq!(count, 4);
    let broken = list_shots(&index, None).into_iter().find(|s| s.name == "broken.png").unwrap();
    assert!(broken.thumb.is_none());
}

#[test]
fn indexing_never_modifies_the_originals() {
    let (_t, root, imported, index, thumbs) = setup();
    let original = root.join("Los Sims 4_solo/Capturas de pantalla/a.png");
    let before = fs::read(&original).unwrap();
    update_index(&root, &imported, &index, &thumbs, &no_progress).unwrap();
    assert_eq!(fs::read(&original).unwrap(), before);
}

#[test]
fn corrupt_index_file_is_rebuilt() {
    let (_t, root, imported, index, thumbs) = setup();
    fs::create_dir_all(index.parent().unwrap()).unwrap();
    fs::write(&index, "{roto").unwrap();
    assert!(!index_exists(&index));
    assert_eq!(update_index(&root, &imported, &index, &thumbs, &no_progress).unwrap(), 3);
    assert!(index_exists(&index));
}
