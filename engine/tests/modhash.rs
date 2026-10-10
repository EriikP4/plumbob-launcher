use s4im_engine::{codes_match, compare_mods, scan_mods, InstanceKind, Role};
use std::fs;
use std::path::Path;

fn write(p: &Path, c: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, c).unwrap();
}

fn scan(dir: &Path) -> s4im_engine::ModScan {
    scan_mods(dir, None, &|_, _| {}).unwrap()
}

#[test]
fn same_files_in_different_folders_give_the_same_code() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    write(&a.path().join("Mods/x.package"), "one");
    write(&a.path().join("Mods/Script Packs/y.ts4script"), "two");
    write(&a.path().join("Mods/readme.txt"), "ignored");
    write(&a.path().join("Mods/log.log"), "ignored");
    write(&b.path().join("Mods/deep/er/X.package"), "one");
    write(&b.path().join("Mods/y.ts4script"), "two");
    write(&b.path().join("Mods/other.log"), "different log");

    let sa = scan(a.path());
    let sb = scan(b.path());
    assert_eq!((sa.packages, sa.scripts), (1, 1));
    assert!(sa.code.starts_with("PB1-2-"));
    assert!(codes_match(&sa.code, &sb.code));
    assert!(codes_match(&sa.code, &sa.code.to_lowercase().replace("pb1", "PB1")));

    write(&b.path().join("Mods/y.ts4script"), "changed");
    assert!(!codes_match(&sa.code, &scan(b.path()).code));
}

#[test]
fn diff_reports_missing_extra_and_changed() {
    let host = tempfile::tempdir().unwrap();
    let me = tempfile::tempdir().unwrap();
    write(&host.path().join("Mods/same.package"), "s");
    write(&host.path().join("Mods/only_host.package"), "h");
    write(&host.path().join("Mods/changed.ts4script"), "v2");
    write(&me.path().join("Mods/same.package"), "s");
    write(&me.path().join("Mods/only_me.package"), "m");
    write(&me.path().join("Mods/changed.ts4script"), "v1");

    let d = compare_mods(&scan(host.path()).files, &scan(me.path()).files);
    assert_eq!(d.missing.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["only_host.package"]);
    assert_eq!(d.extra.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["only_me.package"]);
    assert_eq!(d.different.len(), 1);
    assert_eq!(d.different[0].host.name, "changed.ts4script");
}

#[test]
fn cache_is_reused_and_invalidated_by_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache").join("x.json");
    write(&tmp.path().join("inst/Mods/a.package"), "aaaa");
    let first = scan_mods(&tmp.path().join("inst"), Some(&cache), &|_, _| {}).unwrap();
    assert!(cache.is_file());
    let again = scan_mods(&tmp.path().join("inst"), Some(&cache), &|_, _| {}).unwrap();
    assert_eq!(first.code, again.code);
    write(&tmp.path().join("inst/Mods/a.package"), "bbbbbbbb");
    let third = scan_mods(&tmp.path().join("inst"), Some(&cache), &|_, _| {}).unwrap();
    assert_ne!(first.code, third.code);
}

#[test]
fn role_only_applies_to_multiplayer() {
    use s4im_engine::{create_instance, set_instance_role};
    let tmp = tempfile::tempdir().unwrap();
    create_instance(tmp.path(), "Solo", "", "so", InstanceKind::Solo).unwrap();
    create_instance(tmp.path(), "Multi", "", "mp", InstanceKind::Mp).unwrap();
    assert_eq!(set_instance_role(tmp.path(), "so", Some(Role::Host)).unwrap().role, None);
    assert_eq!(set_instance_role(tmp.path(), "mp", Some(Role::Client)).unwrap().role, Some(Role::Client));
    assert_eq!(set_instance_role(tmp.path(), "mp", None).unwrap().role, None);
}
