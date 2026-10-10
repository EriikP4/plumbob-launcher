use s4im_engine::{check_game_root, RootState};
use std::fs;

#[test]
fn accepts_only_the_game_root() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("The Sims 4");
    let bin = root.join("Game").join("Bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("Default.ini"), "[Version]\ncdsversion = 47.0\ngameversion = 1.128.90.1030\n").unwrap();

    let ok = check_game_root(&root);
    assert_eq!(ok.state, RootState::Ok);
    assert_eq!(ok.version.as_deref(), Some("1.128.90.1030"));

    for inner in [bin.clone(), root.join("Game")] {
        let c = check_game_root(&inner);
        assert_eq!(c.state, RootState::Inside);
        assert_eq!(c.root.as_deref(), Some(root.as_path()));
    }

    assert_eq!(check_game_root(tmp.path()).state, RootState::Invalid);
    assert_eq!(check_game_root(&tmp.path().join("nada")).state, RootState::Invalid);
}
