use crate::discover::{read_instance_meta, write_instance_meta};
use crate::error::{EngineError, Result};
use crate::instances::{instance_dir, process_running};
use crate::settings::{Platform, Settings};
use crate::snapshot::{create_snapshot, prune_auto_snapshots, SnapshotKind};
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const GAME_PROCESS: &str = "TS4_x64.exe";
pub const S4MM_PROCESS: &str = "Sims 4 Mod Manager.exe";
const DEFAULT_STEAM_ID: &str = "1222670";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const KEEP_AUTO_SNAPSHOTS: usize = 10;
const GAME_EXE: &str = "Game\\Bin\\TS4_x64.exe";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEnd {
    NeverStarted,
    Exited,
}

fn is_url(s: &str) -> bool {
    let Some((scheme, rest)) = s.split_once("://") else { return false };
    !rest.is_empty()
        && scheme.len() > 1
        && scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
}

pub fn split_command(cmd: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in cmd.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => cur.push(c),
        }
    }
    if any || !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn open_url(url: &str) -> Result<()> {
    if !is_url(url) {
        return Err(EngineError::Invalid(format!("url: {url}")));
    }
    Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;
    Ok(())
}

fn run_exe(path: &Path, args: &[String]) -> Result<()> {
    if !path.is_file() {
        return Err(EngineError::NotFound(path.display().to_string()));
    }
    Command::new(path)
        .args(args)
        .current_dir(path.parent().unwrap_or(Path::new(".")))
        .spawn()?;
    Ok(())
}

pub fn candidate_install_dirs(platform: Platform) -> Vec<PathBuf> {
    let paths: &[&str] = match platform {
        Platform::Steam => &[r"C:\Program Files (x86)\Steam\steamapps\common\The Sims 4"],
        Platform::Ea => &[
            r"C:\Program Files\EA Games\The Sims 4",
            r"C:\Program Files (x86)\Origin Games\The Sims 4",
        ],
        Platform::Epic => &[r"C:\Program Files\Epic Games\The Sims 4"],
        Platform::Custom => &[],
    };
    paths.iter().map(PathBuf::from).collect()
}

pub fn detect_install_dir(platform: Platform) -> Option<PathBuf> {
    candidate_install_dirs(platform)
        .into_iter()
        .find(|d| crate::install::game_exe(d).is_some() || d.join(GAME_EXE).is_file())
}

pub fn ea_offer_ids(install_dir: &Path) -> Vec<String> {
    let Ok(xml) = fs::read_to_string(install_dir.join("__Installer").join("installerdata.xml")) else {
        return Vec::new();
    };
    let mut ids = Vec::new();
    let mut rest = xml.as_str();
    while let Some(start) = rest.find("<contentID>") {
        let after = &rest[start + "<contentID>".len()..];
        let Some(end) = after.find("</contentID>") else { break };
        let id = after[..end].trim();
        if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) && !ids.iter().any(|i| i == id) {
            ids.push(id.to_string());
        }
        rest = &after[end..];
    }
    ids
}

pub fn detect_s4mm() -> Option<PathBuf> {
    let base = dirs::data_local_dir()?;
    let exe = base.join("Programs").join("Sims 4 Mod Manager").join(S4MM_PROCESS);
    exe.is_file().then_some(exe)
}

pub const GAME_ARGS: [&str; 4] = ["-w", "-f", "-nointro", "-nohovereffects"];

pub fn effective_game_args(settings: &Settings) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for arg in &settings.game_args {
        let arg = arg.trim();
        if !GAME_ARGS.contains(&arg) || out.iter().any(|a| a == arg) {
            continue;
        }
        let clash = (arg == "-w" && out.iter().any(|a| a == "-f")) || (arg == "-f" && out.iter().any(|a| a == "-w"));
        if !clash {
            out.push(arg.to_string());
        }
    }
    out
}

fn game_exe_path(dir: &Path) -> PathBuf {
    let root = crate::install::resolve_game_root(dir).unwrap_or_else(|| dir.to_path_buf());
    crate::install::game_exe(&root).unwrap_or_else(|| root.join(GAME_EXE))
}

pub fn launch_game(settings: &Settings) -> Result<()> {
    match settings.platform {
        Platform::Steam => {
            let id = settings.steam_id.as_deref().filter(|s| !s.is_empty()).unwrap_or(DEFAULT_STEAM_ID);
            if !id.chars().all(|c| c.is_ascii_digit()) {
                return Err(EngineError::Invalid(format!("launch id: {id}")));
            }
            open_url(&format!("steam://rungameid/{id}"))
        }
        Platform::Ea | Platform::Epic => {
            let dir = settings
                .install_dir
                .clone()
                .or_else(|| detect_install_dir(settings.platform))
                .ok_or_else(|| EngineError::Invalid("install dir".into()))?;
            if settings.platform == Platform::Ea {
                let ids = ea_offer_ids(&dir);
                if !ids.is_empty() {
                    return open_url(&format!("origin2://game/launch?offerIds={}", ids.join(",")));
                }
            }
            run_exe(&game_exe_path(&dir), &[])
        }
        Platform::Custom => {
            let cmd = settings.custom_command.as_deref().map(str::trim).filter(|c| !c.is_empty());
            let Some(cmd) = cmd else {
                let dir = settings
                    .install_dir
                    .as_ref()
                    .ok_or_else(|| EngineError::Invalid("custom command".into()))?;
                return run_exe(&game_exe_path(dir), &effective_game_args(settings));
            };
            if is_url(cmd) {
                return open_url(cmd);
            }
            let parts = split_command(cmd);
            let (exe, args) = parts
                .split_first()
                .ok_or_else(|| EngineError::Invalid("custom command".into()))?;
            run_exe(Path::new(exe), args)
        }
    }
}

pub fn launch_s4mp(settings: &Settings) -> Result<()> {
    let path = settings
        .multiplayer
        .launcher
        .as_ref()
        .ok_or_else(|| EngineError::Invalid("s4mp launcher not configured".into()))?;
    run_exe(path, &[])
}

pub fn launch_s4mm(settings: &Settings) -> Result<()> {
    let path = settings
        .s4mm
        .path
        .clone()
        .or_else(detect_s4mm)
        .ok_or_else(|| EngineError::Invalid("s4mm not configured".into()))?;
    if !path.is_file() {
        return Err(EngineError::NotFound(path.display().to_string()));
    }
    if process_running(S4MM_PROCESS) {
        return Ok(());
    }
    run_exe(&path, &[])
}

pub fn wait_for_session(
    process: &str,
    appear_timeout: Duration,
    poll: Duration,
    on_started: impl FnOnce(),
) -> SessionEnd {
    wait_for_session_ex(process, None, appear_timeout, Duration::ZERO, poll, &AtomicBool::new(false), on_started)
}

pub fn wait_for_session_ex(
    process: &str,
    launcher: Option<&str>,
    appear_timeout: Duration,
    launcher_grace: Duration,
    poll: Duration,
    cancel: &AtomicBool,
    on_started: impl FnOnce(),
) -> SessionEnd {
    let deadline = Instant::now() + appear_timeout;
    let mut launcher_seen = false;
    let mut launcher_gone_at: Option<Instant> = None;
    while !process_running(process) {
        let now = Instant::now();
        if cancel.load(Ordering::SeqCst) || now >= deadline {
            return SessionEnd::NeverStarted;
        }
        if let Some(name) = launcher {
            if process_running(name) {
                launcher_seen = true;
                launcher_gone_at = None;
            } else if launcher_seen && now.duration_since(*launcher_gone_at.get_or_insert(now)) >= launcher_grace {
                return SessionEnd::NeverStarted;
            }
        }
        std::thread::sleep(poll);
    }
    on_started();
    while process_running(process) {
        std::thread::sleep(poll);
    }
    SessionEnd::Exited
}

pub fn mark_played(game_root: &Path, code: &str, elapsed_secs: u64) -> Result<()> {
    let dir = instance_dir(game_root, code);
    let mut meta = read_instance_meta(&dir)?
        .ok_or_else(|| EngineError::NotFound(code.to_string()))?;
    meta.play_seconds = meta.play_seconds.saturating_add(elapsed_secs);
    meta.last_played = Some(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
    );
    write_instance_meta(&dir, &meta)
}

pub fn close_session(game_root: &Path, snapshots_root: &Path, code: &str, elapsed_secs: u64) -> Result<()> {
    mark_played(game_root, code, elapsed_secs)?;
    let dir = instance_dir(game_root, code);
    match create_snapshot(&dir, snapshots_root, code, "snapshot.sessionClose", SnapshotKind::Auto, false) {
        Ok(_) | Err(EngineError::Invalid(_)) => {}
        Err(e) => return Err(e),
    }
    prune_auto_snapshots(snapshots_root, code, KEEP_AUTO_SNAPSHOTS)?;
    Ok(())
}
