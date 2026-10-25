use crate::error::{EngineError, Result};
use crate::fsutil::{find_child, is_link};
use crate::instances::{active_instance_id, game_running, instance_dir};
use crate::link::{create_junction, remove_junction};
use crate::manage::registered;
use crate::snapshot::{create_snapshot, SnapshotKind};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const SHARED_TRAY: &str = crate::fsutil::SHARED_TRAY_NAME;
const TRAY: &str = "Tray";
const TRAY_OLD: &str = "TrayOld";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrayDisable {
    Restore,
    Merge,
    Empty,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeReport {
    pub copied: u32,
    pub updated: u32,
    pub skipped: u32,
}

pub fn shared_tray_dir(game_root: &Path) -> PathBuf {
    crate::fsutil::shared_tray_path(game_root)
}

fn modified(path: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

pub fn merge_dir(src: &Path, dst: &Path, report: &mut MergeReport) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        if is_link(&from) {
            continue;
        }
        let to = dst.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            merge_dir(&from, &to, report)?;
        } else if kind.is_file() {
            if !to.exists() {
                fs::copy(&from, &to)?;
                report.copied += 1;
                continue;
            }
            let same_size = fs::metadata(&from)?.len() == fs::metadata(&to)?.len();
            let (src_time, dst_time) = (modified(&from), modified(&to));
            if same_size && src_time == dst_time {
                report.skipped += 1;
            } else if src_time > dst_time {
                fs::copy(&from, &to)?;
                report.updated += 1;
            } else {
                report.skipped += 1;
            }
        }
    }
    Ok(())
}

fn tray_path(dir: &Path) -> PathBuf {
    find_child(dir, TRAY).unwrap_or_else(|| dir.join(TRAY))
}

fn old_path(dir: &Path) -> Option<PathBuf> {
    find_child(dir, TRAY_OLD).filter(|p| p.is_dir() && !is_link(p))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayOldInfo {
    pub path: PathBuf,
    pub files: u64,
    pub bytes: u64,
}

pub fn tray_old_info(game_root: &Path, code: &str) -> Result<Option<TrayOldInfo>> {
    let (dir, _) = registered(game_root, code)?;
    Ok(old_path(&dir).map(|path| {
        let (files, bytes) = crate::fsutil::dir_stats(&path);
        TrayOldInfo { path, files, bytes }
    }))
}

pub fn delete_tray_old(game_root: &Path, code: &str) -> Result<()> {
    let (dir, _) = registered(game_root, code)?;
    if let Some(old) = old_path(&dir) {
        fs::remove_dir_all(old)?;
    }
    Ok(())
}

pub fn is_synced(game_root: &Path, dir: &Path) -> bool {
    let tray = tray_path(dir);
    if !is_link(&tray) {
        return false;
    }
    match (fs::canonicalize(&tray), fs::canonicalize(shared_tray_dir(game_root))) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn guard_game(game_root: &Path, code: &str, check_running: bool) -> Result<()> {
    let active = active_instance_id(game_root).is_some_and(|a| a.eq_ignore_ascii_case(code));
    if check_running && (game_running() || (active && game_running())) {
        return Err(EngineError::GameRunning);
    }
    Ok(())
}

fn free_name(dir: &Path, base: &str) -> PathBuf {
    let mut n = 2;
    loop {
        let candidate = dir.join(format!("{base}_{n}"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

pub fn enable_tray_sync(
    game_root: &Path,
    snapshots_root: &Path,
    code: &str,
    check_running: bool,
) -> Result<MergeReport> {
    let (dir, _) = registered(game_root, code)?;
    guard_game(game_root, code, check_running)?;
    let shared = shared_tray_dir(game_root);
    let tray = tray_path(&dir);
    let mut report = MergeReport::default();

    if is_link(&tray) {
        if is_synced(game_root, &dir) {
            return Ok(report);
        }
        return Err(EngineError::Invalid("tray is a link".into()));
    }

    match create_snapshot(&dir, snapshots_root, code, "snapshot.beforeTraySync", SnapshotKind::Auto, false) {
        Ok(_) | Err(EngineError::Invalid(_)) => {}
        Err(e) => return Err(e),
    }

    fs::create_dir_all(&shared)?;
    let mut moved_aside = None;
    if tray.is_dir() {
        merge_dir(&tray, &shared, &mut report)?;
        if let Some(existing) = old_path(&dir) {
            fs::rename(&existing, free_name(&dir, TRAY_OLD))?;
        }
        let old = dir.join(TRAY_OLD);
        fs::rename(&tray, &old)?;
        moved_aside = Some(old);
    }
    if let Err(e) = create_junction(&dir.join(TRAY), &shared) {
        if let Some(old) = moved_aside {
            let _ = fs::rename(&old, dir.join(TRAY));
        }
        return Err(e);
    }
    Ok(report)
}

pub fn disable_tray_sync(
    game_root: &Path,
    code: &str,
    mode: TrayDisable,
    check_running: bool,
) -> Result<MergeReport> {
    let (dir, _) = registered(game_root, code)?;
    guard_game(game_root, code, check_running)?;
    let tray = tray_path(&dir);
    if !is_synced(game_root, &dir) {
        return Err(EngineError::Invalid("tray not synced".into()));
    }
    let shared = shared_tray_dir(game_root);
    let old = old_path(&dir);
    let mut report = MergeReport::default();

    match mode {
        TrayDisable::Restore => {
            let old = old.ok_or_else(|| EngineError::Invalid("no tray backup".into()))?;
            remove_junction(&tray)?;
            if let Err(e) = fs::rename(&old, &tray) {
                let _ = create_junction(&tray, &shared);
                return Err(e.into());
            }
        }
        TrayDisable::Merge => {
            remove_junction(&tray)?;
            let built = (|| -> Result<()> {
                fs::create_dir_all(&tray)?;
                if let Some(old) = &old {
                    merge_dir(old, &tray, &mut report)?;
                }
                merge_dir(&shared, &tray, &mut report)
            })();
            if let Err(e) = built {
                let _ = fs::remove_dir_all(&tray);
                let _ = create_junction(&tray, &shared);
                return Err(e);
            }
        }
        TrayDisable::Empty => {
            remove_junction(&tray)?;
            fs::create_dir_all(&tray)?;
        }
    }
    Ok(report)
}

pub fn link_new_instance_tray(game_root: &Path, code: &str) -> Result<()> {
    let dir = instance_dir(game_root, code);
    let tray = tray_path(&dir);
    if tray.exists() || is_link(&tray) {
        return Ok(());
    }
    let shared = shared_tray_dir(game_root);
    fs::create_dir_all(&shared)?;
    create_junction(&tray, &shared)
}
