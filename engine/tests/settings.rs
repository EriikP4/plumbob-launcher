#![cfg(windows)]

use s4im_engine::*;
use std::fs;
use std::path::PathBuf;

#[test]
fn defaults_when_file_is_missing_or_broken() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("settings.json");
    let s = load_settings(&file);
    assert_eq!(s.language, "es");
    assert!(!s.setup_done);
    assert_eq!(s.platform, Platform::Steam);
    assert!(s.theme.animations);
    assert!(s.boot.animation && s.boot.sound);
    assert!(!s.shots.quality_badges);
    fs::write(&file, "{roto").unwrap();
    assert!(!load_settings(&file).setup_done);
}

#[test]
fn roundtrip_and_partial_files_and_bom() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("datos").join("settings.json");
    let mut s = Settings::default();
    s.language = "en".into();
    s.setup_done = true;
    s.platform = Platform::Custom;
    s.custom_command = Some("steam://rungameid/1".into());
    s.s4mm.enabled = true;
    s.multiplayer.launcher = Some(PathBuf::from(r"C:\mp\mp_launcher.exe"));
    s.theme.accent = Some("#ff8800".into());
    s.theme.custom.insert("--bg".into(), "#000000".into());
    save_settings(&file, &s).unwrap();

    let back = load_settings(&file);
    assert_eq!(back.language, "en");
    assert_eq!(back.platform, Platform::Custom);
    assert_eq!(back.custom_command.as_deref(), Some("steam://rungameid/1"));
    assert!(back.s4mm.enabled);
    assert!(!back.s4mm.launch_with_game);
    assert_eq!(back.multiplayer.launcher, s.multiplayer.launcher);
    assert_eq!(back.theme.custom.get("--bg").map(String::as_str), Some("#000000"));

    fs::write(&file, "\u{feff}{\"language\":\"en\",\"platform\":\"ea\"}").unwrap();
    let partial = load_settings(&file);
    assert_eq!(partial.platform, Platform::Ea);
    assert!(partial.theme.animations);
    assert!(!partial.s4mm.enabled);
}

#[test]
fn config_applies_path_overrides() {
    let cfg = Config::detect().unwrap();
    let mut s = Settings::default();
    assert_eq!(cfg.clone().apply(&s).game_root, cfg.game_root);
    s.game_root = Some(PathBuf::from(r"D:\Docs\Electronic Arts"));
    s.snapshots_root = Some(PathBuf::from(r"D:\Copias"));
    let applied = cfg.apply(&s);
    assert_eq!(applied.game_root, PathBuf::from(r"D:\Docs\Electronic Arts"));
    assert_eq!(applied.game_dir, PathBuf::from(r"D:\Docs\Electronic Arts\Los Sims 4"));
    assert_eq!(applied.snapshots_root, PathBuf::from(r"D:\Copias"));
}

#[test]
fn split_command_handles_quotes() {
    assert_eq!(split_command(r#""C:\Program Files\x.exe" -a "b c" d"#), ["C:\\Program Files\\x.exe", "-a", "b c", "d"]);
    assert_eq!(split_command("  a   b "), ["a", "b"]);
    assert!(split_command("   ").is_empty());
}

#[test]
fn launchers_reject_bad_input_without_running_anything() {
    let mut s = Settings::default();
    s.steam_id = Some("12;calc".into());
    assert!(launch_game(&s).is_err());

    let mut custom = Settings::default();
    custom.platform = Platform::Custom;
    assert!(launch_game(&custom).is_err());
    custom.custom_command = Some(r"C:\missing\game.exe -x".into());
    assert!(matches!(launch_game(&custom), Err(EngineError::NotFound(_))));

    assert!(launch_s4mp(&Settings::default()).is_err());
    let mut missing = Settings::default();
    missing.multiplayer.launcher = Some(PathBuf::from(r"C:\missing\mp.exe"));
    assert!(matches!(launch_s4mp(&missing), Err(EngineError::NotFound(_))));

    let mut s4mm = Settings::default();
    s4mm.s4mm.path = Some(PathBuf::from(r"C:\missing\s4mm.exe"));
    assert!(launch_s4mm(&s4mm).is_err());

    let mut epic = Settings::default();
    epic.platform = Platform::Epic;
    epic.install_dir = Some(PathBuf::from(r"C:\missing"));
    assert!(launch_game(&epic).is_err());
}

#[test]
fn ea_offer_ids_are_read_from_installer_data() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("The Sims 4");
    fs::create_dir_all(dir.join("__Installer")).unwrap();
    fs::write(
        dir.join("__Installer").join("installerdata.xml"),
        "<a><contentID>1026023</contentID><b><contentID> Origin.OFR.50.0001 </contentID><contentID>1026023</contentID><contentID>mal&amp;id</contentID></b></a>",
    )
    .unwrap();
    assert_eq!(ea_offer_ids(&dir), ["1026023", "Origin.OFR.50.0001"]);
    assert!(ea_offer_ids(&tmp.path().join("nada")).is_empty());
}

#[test]
fn install_detection_needs_the_game_executable() {
    for p in [Platform::Steam, Platform::Ea, Platform::Epic] {
        assert!(!candidate_install_dirs(p).is_empty());
    }
    assert!(candidate_install_dirs(Platform::Custom).is_empty());
    assert!(detect_install_dir(Platform::Custom).is_none());
}
