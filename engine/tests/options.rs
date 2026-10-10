use s4im_engine::{apply_game_options, GameOptions, MusicChoice};
use std::fs;

fn all() -> GameOptions {
    GameOptions { mods: true, skip_dlc_intro: true, no_telemetry: true, music: MusicChoice::Half, windowed: true }
}

#[test]
fn writes_a_fresh_options_file_and_the_dlc_marker() {
    let tmp = tempfile::tempdir().unwrap();
    apply_game_options(tmp.path(), &all()).unwrap();
    let ini = fs::read_to_string(tmp.path().join("Options.ini")).unwrap();
    for line in [
        "[options]",
        "modsdisabled = 0",
        "scriptmodsenabled = 1",
        "enabletelemetry = 0",
        "musiclevel = 128",
        "menumusiclevel = 128",
        "fullscreen = 1",
        "windowedfullscreen = 1",
        "packguidanceenabled = 0",
        "tutorialenabled = 0",
    ] {
        assert!(ini.lines().any(|l| l == line), "missing {line}");
    }
    assert!(tmp.path().join("ReticulatedSplinesView").metadata().unwrap().len() > 1000);
}

#[test]
fn music_off_and_merge_keep_other_keys() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Options.ini"), "[options]\r\nmusiclevel = 255\r\nresolutionwidth = 1280\r\n").unwrap();
    let opts = GameOptions { music: MusicChoice::Off, ..GameOptions::default() };
    apply_game_options(tmp.path(), &opts).unwrap();
    let ini = fs::read_to_string(tmp.path().join("Options.ini")).unwrap();
    assert!(ini.lines().any(|l| l == "musiclevel = 0"));
    assert!(ini.lines().any(|l| l == "menumusiclevel = 0"));
    assert!(ini.lines().any(|l| l == "resolutionwidth = 1280"));
    assert!(!tmp.path().join("ReticulatedSplinesView").exists());
}

#[test]
fn nothing_selected_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    apply_game_options(tmp.path(), &GameOptions::default()).unwrap();
    assert!(!tmp.path().join("Options.ini").exists());
}
