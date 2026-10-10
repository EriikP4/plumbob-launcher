use s4im_engine::{analyze_health, space_children, space_top_files};
use std::fs;
use std::path::Path;

fn write(p: &Path, c: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, c).unwrap();
}

#[test]
fn detects_mods_versions_and_warnings() {
    let tmp = tempfile::tempdir().unwrap();
    let mods = tmp.path().join("Mods");
    write(&mods.join("Script Packs/mc_cmd_center.ts4script"), "x");
    write(&mods.join("Script Packs/mc_cmd_center.log"), "[a]Initializing MC Command Center version: 2026.5.0...\n");
    write(&mods.join("Script Packs/Tmex-BetterExceptions.ts4script"), "x");
    write(&mods.join("TMEX-Settings/BetterExceptions-XMLREADER.dat"), "<root><BEversion>v3.17</BEversion></root>");
    write(&mods.join("Script Packs/XmlInjector_Script_v4.2.ts4script"), "x");
    write(&mods.join("a/b/c/deep.ts4script"), "x");
    write(&mods.join("pack.zip"), "z");
    write(&mods.join("one/same.package"), "abc");
    write(&mods.join("two/same.package"), "abc");

    let h = analyze_health(tmp.path());
    let by = |id: &str| h.mods.iter().find(|m| m.id == id).unwrap().clone();
    assert!(by("mccc").installed);
    assert_eq!(by("mccc").version.as_deref(), Some("2026.5.0"));
    assert_eq!(by("betterexceptions").version.as_deref(), Some("3.17"));
    assert_eq!(by("xmlinjector").version.as_deref(), Some("4.2"));
    assert!(!by("wickedwhims").installed);
    let kinds: Vec<_> = h.warnings.iter().map(|w| w.kind.as_str()).collect();
    assert!(kinds.contains(&"deepScripts") && kinds.contains(&"archives") && kinds.contains(&"duplicates"));
}

#[test]
fn space_is_sorted_and_rejects_traversal() {
    let tmp = tempfile::tempdir().unwrap();
    write(&tmp.path().join("Mods/big.package"), "0123456789");
    write(&tmp.path().join("Saves/s.save"), "01");
    let top = space_children(tmp.path(), "").unwrap();
    assert_eq!(top[0].name, "Mods");
    assert_eq!(top[0].bytes, 10);
    assert!(space_children(tmp.path(), "../x").is_err());
    assert_eq!(space_top_files(tmp.path(), 1)[0].name, "big.package");
}

#[test]
fn exception_report_priority() {
    let tmp = tempfile::tempdir().unwrap();
    write(&tmp.path().join("lastException_1.txt"), "g");
    let h = analyze_health(tmp.path());
    assert_eq!(h.last_exception_source.as_deref(), Some("game"));

    write(&tmp.path().join("Mods/Script Packs/mc_cmd_center.ts4script"), "x");
    write(&tmp.path().join("Mods/Script Packs/mc_lastexception.html"), "m");
    let h = analyze_health(tmp.path());
    assert_eq!(h.last_exception_source.as_deref(), Some("mccc"));
    assert_eq!(h.last_exception_file.as_deref(), Some("Mods/Script Packs/mc_lastexception.html"));

    write(&tmp.path().join("Mods/Script Packs/Tmex-BetterExceptions.ts4script"), "x");
    write(&tmp.path().join("BE-ExceptionReport.html"), "b");
    write(&tmp.path().join("BE-ExceptionReport-OLD.html"), "o");
    let h = analyze_health(tmp.path());
    assert_eq!(h.last_exception_source.as_deref(), Some("betterexceptions"));
    assert_eq!(h.last_exception_file.as_deref(), Some("BE-ExceptionReport.html"));
}
