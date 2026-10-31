use s4im_engine::*;
use std::fs;
use std::io::Write;

fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts = zip::write::SimpleFileOptions::default();
        for (name, data) in files {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    buf
}

fn package() -> Vec<u8> {
    let mut data = b"DBPF".to_vec();
    data.extend_from_slice(&[0u8; 96]);
    data
}

#[test]
fn only_the_package_is_extracted_to_the_mods_root() {
    let tmp = tempfile::tempdir().unwrap();
    let mods = tmp.path().join("Mods");
    let zip = zip_of(&[("readme.txt", b"hi"), ("sub/dir/Simser_NoIntro.package", &package())]);
    let name = extract_no_intro(&zip, &mods).unwrap();
    assert_eq!(name, "Simser_NoIntro.package");
    assert!(mods.join("Simser_NoIntro.package").is_file());
    assert_eq!(fs::read_dir(&mods).unwrap().count(), 1);
}

#[test]
fn html_pages_broken_zips_and_foreign_content_install_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let mods = tmp.path().join("Mods");
    assert!(extract_no_intro(b"<html>login required</html>", &mods).is_err());
    assert!(extract_no_intro(b"PK\x03\x04garbage", &mods).is_err());
    assert!(extract_no_intro(&zip_of(&[("a.txt", b"x")]), &mods).is_err());
    assert!(extract_no_intro(&zip_of(&[("mod.package", b"NOTDBPF-data")]), &mods).is_err());
    assert!(extract_no_intro(&zip_of(&[("script.ts4script", b"DBPF1234")]), &mods).is_err());
    assert!(!mods.exists() || fs::read_dir(&mods).unwrap().count() == 0);
}

#[test]
fn hostile_names_are_flattened_and_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let mods = tmp.path().join("Mods");
    let name = extract_no_intro(&zip_of(&[("../../escape/NoIntro.package", &package())]), &mods).unwrap();
    assert_eq!(name, "NoIntro.package");
    assert!(mods.join("NoIntro.package").is_file());
    assert!(!tmp.path().join("escape").exists());
    assert!(extract_no_intro(&zip_of(&[("bad:name?.package", &package())]), &mods).is_err());
}

#[test]
fn installation_is_detected_by_marker_or_name_and_can_be_removed() {
    let tmp = tempfile::tempdir().unwrap();
    let instance = tmp.path().join("inst");
    let mods = instance.join("Mods");
    fs::create_dir_all(&mods).unwrap();
    assert!(no_intro_installed(&instance, &mods).is_none());

    fs::write(mods.join("Other.package"), package()).unwrap();
    assert!(no_intro_installed(&instance, &mods).is_none());

    let name = extract_no_intro(&zip_of(&[("Weird Name.package", &package())]), &mods).unwrap();
    record_no_intro(&instance, &name).unwrap();
    assert_eq!(no_intro_installed(&instance, &mods).unwrap(), mods.join("Weird Name.package"));
    assert!(remove_no_intro(&instance, &mods).unwrap());
    assert!(no_intro_installed(&instance, &mods).is_none());
    assert!(mods.join("Other.package").is_file());
    assert!(!remove_no_intro(&instance, &mods).unwrap());

    fs::write(mods.join("TS4_No_Intro_v2.package"), package()).unwrap();
    assert!(no_intro_installed(&instance, &mods).is_some());
}
