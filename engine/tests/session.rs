#![cfg(windows)]

use s4im_engine::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Duration;

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn fake_game(dir: &Path, name: &str) -> PathBuf {
    let exe = dir.join(name);
    fs::copy(r"C:\Windows\System32\PING.EXE", &exe).unwrap();
    exe
}

fn run_fake(exe: &Path, seconds: u32) -> Child {
    Command::new(exe)
        .args(["-n", &seconds.to_string(), "127.0.0.1"])
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap()
}

#[test]
fn session_waits_for_process_to_appear_and_exit() {
    let tmp = tempfile::tempdir().unwrap();
    let name = "s4im_fake_game_a.exe";
    let exe = fake_game(tmp.path(), name);
    assert!(!process_running(name));

    let mut child = run_fake(&exe, 4);
    let mut started = false;
    let end = wait_for_session(name, Duration::from_secs(10), Duration::from_millis(300), || started = true);
    child.wait().unwrap();

    assert_eq!(end, SessionEnd::Exited);
    assert!(started);
    assert!(!process_running(name));
}

#[test]
fn session_reports_never_started_after_timeout() {
    let mut started = false;
    let end = wait_for_session(
        "s4im_fake_game_never.exe",
        Duration::from_millis(600),
        Duration::from_millis(200),
        || started = true,
    );
    assert_eq!(end, SessionEnd::NeverStarted);
    assert!(!started);
}

#[test]
fn closing_a_session_marks_last_played_and_snapshots() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    let snaps = tmp.path().join("snaps");
    write(&root.join("Los Sims 4_solo/saves/s.save"), "solo");
    adopt_instance(&root, "solo", "Solo", "", InstanceKind::Solo).unwrap();
    assert_eq!(list_instances(&root).unwrap()[0].last_played, None);

    close_session(&root, &snaps, "solo", 90).unwrap();

    let inst = &list_instances(&root).unwrap()[0];
    assert!(inst.last_played.unwrap() > 1_700_000_000);
    assert_eq!(inst.name, "Solo");
    assert_eq!(inst.play_seconds, 90);
    close_session(&root, &snaps, "solo", 30).unwrap();
    assert_eq!(list_instances(&root).unwrap()[0].play_seconds, 120);
    let list = list_snapshots(&snaps, "solo").unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].label, "snapshot.sessionClose");
    assert_eq!(list[0].kind, SnapshotKind::Auto);
}

#[test]
fn closing_a_session_of_an_unregistered_instance_fails_cleanly() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    write(&root.join("Los Sims 4_old/saves/s.save"), "x");
    assert!(close_session(&root, &tmp.path().join("snaps"), "old", 5).is_err());
    assert!(!root.join("Los Sims 4_old/.plumbob.json").exists());
}


#[test]
fn session_gives_up_when_the_launcher_closes_before_the_game_appears() {
    use std::sync::atomic::AtomicBool;
    let tmp = tempfile::tempdir().unwrap();
    let launcher = "s4im_fake_launcher_b.exe";
    let exe = fake_game(tmp.path(), launcher);
    let mut child = run_fake(&exe, 3);
    let begun = std::time::Instant::now();
    let mut started = false;
    let end = wait_for_session_ex(
        "s4im_fake_game_never_b.exe",
        Some(launcher),
        Duration::from_secs(60),
        Duration::from_secs(1),
        Duration::from_millis(250),
        &AtomicBool::new(false),
        || started = true,
    );
    child.wait().unwrap();
    assert_eq!(end, SessionEnd::NeverStarted);
    assert!(!started);
    assert!(begun.elapsed() < Duration::from_secs(20));
}

#[test]
fn session_wait_can_be_cancelled() {
    use std::sync::atomic::AtomicBool;
    let cancel = AtomicBool::new(true);
    let end = wait_for_session_ex(
        "s4im_fake_game_never_c.exe",
        None,
        Duration::from_secs(60),
        Duration::ZERO,
        Duration::from_millis(100),
        &cancel,
        || (),
    );
    assert_eq!(end, SessionEnd::NeverStarted);
}
