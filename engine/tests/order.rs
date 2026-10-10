use s4im_engine::*;
use std::fs;
use std::path::Path;

fn instance(root: &Path, code: &str) {
    fs::create_dir_all(root.join(format!("Los Sims 4_{code}")).join("saves")).unwrap();
}

#[test]
fn order_roundtrip_and_missing_or_corrupt_file() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("datos").join("order.json");
    assert!(load_order(&file).is_empty());
    save_order(&file, &["b".into(), "a".into()]).unwrap();
    assert_eq!(load_order(&file), ["b", "a"]);
    fs::write(&file, "no es json").unwrap();
    assert!(load_order(&file).is_empty());
}

#[test]
fn saved_order_wins_and_unknown_instances_go_last() {
    let tmp = tempfile::tempdir().unwrap();
    for c in ["a", "b", "c", "d"] {
        instance(tmp.path(), c);
    }
    let mut list = list_instances(tmp.path()).unwrap();
    apply_order(&mut list, &["c".into(), "ghost".into(), "A".into()]);
    let ids: Vec<&str> = list.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(ids, ["c", "a", "b", "d"]);
}

#[test]
fn empty_order_keeps_default_sorting() {
    let tmp = tempfile::tempdir().unwrap();
    for c in ["z", "m"] {
        instance(tmp.path(), c);
    }
    let mut list = list_instances(tmp.path()).unwrap();
    apply_order(&mut list, &[]);
    let ids: Vec<&str> = list.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(ids, ["m", "z"]);
}
