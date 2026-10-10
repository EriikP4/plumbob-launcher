use crate::discover::{read_instance_meta, write_instance_meta};
use crate::error::{EngineError, Result};
use crate::fsutil::{copy_dir, dir_stats, find_child, is_link};
use crate::link::remove_junction;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::instances::{
    active_instance_id, check_name, check_new_code, game_dir, game_running, instance_dir, summary_of, valid_code,
};
use crate::model::{InstanceKind, InstanceMeta, InstanceSummary};
use std::fs;
use std::path::Path;

pub const S4MP_LEFTOVERS: [&str; 4] = ["S4MP_logs", "S4MP_native", "S4MP_release.ts4script", "s4mpconfig"];

pub(crate) fn registered(game_root: &Path, code: &str) -> Result<(std::path::PathBuf, InstanceMeta)> {
    let dir = instance_dir(game_root, code);
    if !dir.is_dir() {
        return Err(EngineError::NotFound(code.to_string()));
    }
    let meta = read_instance_meta(&dir)?
        .ok_or_else(|| EngineError::Invalid(format!("not registered: {code}")))?;
    Ok((dir, meta))
}

pub fn s4mp_leftovers(game_root: &Path, code: &str) -> Vec<String> {
    let dir = instance_dir(game_root, code);
    let Some(mods) = find_child(&dir, "Mods") else { return Vec::new() };
    S4MP_LEFTOVERS
        .iter()
        .filter_map(|name| find_child(&mods, name))
        .filter(|p| !is_link(p))
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect()
}

pub fn update_instance(
    game_root: &Path,
    code: &str,
    name: &str,
    description: &str,
) -> Result<InstanceSummary> {
    let name = check_name(name)?;
    let (dir, mut meta) = registered(game_root, code)?;
    meta.name = name;
    meta.description = description.trim().to_string();
    write_instance_meta(&dir, &meta)?;
    summary_of(game_root, code)
}

pub fn set_instance_role(game_root: &Path, code: &str, role: Option<crate::model::Role>) -> Result<InstanceSummary> {
    let (dir, mut meta) = registered(game_root, code)?;
    meta.role = if meta.kind == InstanceKind::Mp { role } else { None };
    write_instance_meta(&dir, &meta)?;
    summary_of(game_root, code)
}

pub fn convert_instance(
    game_root: &Path,
    code: &str,
    kind: InstanceKind,
    remove_s4mp: bool,
) -> Result<InstanceSummary> {
    let (dir, mut meta) = registered(game_root, code)?;
    if meta.kind == kind {
        return summary_of(game_root, code);
    }
    if remove_s4mp {
        let is_active = active_instance_id(game_root).is_some_and(|a| a.eq_ignore_ascii_case(code));
        if is_active && game_running() {
            return Err(EngineError::GameRunning);
        }
        if let Some(mods) = find_child(&dir, "Mods") {
            for name in s4mp_leftovers(game_root, code) {
                let Some(path) = find_child(&mods, &name) else { continue };
                if is_link(&path) {
                    continue;
                }
                if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }
            }
        }
    }
    meta.kind = kind;
    if kind == InstanceKind::Solo {
        meta.role = None;
    }
    write_instance_meta(&dir, &meta)?;
    summary_of(game_root, code)
}

pub fn duplicate_instance(
    game_root: &Path,
    source_code: &str,
    name: &str,
    description: &str,
    code: &str,
) -> Result<InstanceSummary> {
    let name = check_name(name)?;
    let (src, meta) = registered(game_root, source_code)?;
    check_new_code(game_root, code)?;
    let target = instance_dir(game_root, code);
    let tmp = game_root.join(format!(".import-tmp-{code}"));
    let _ = fs::remove_dir_all(&tmp);
    let was_synced = crate::tray::is_synced(game_root, &src);
    let built = (|| -> Result<()> {
        copy_dir(&src, &tmp)?;
        if !was_synced && dir_stats(&src) != dir_stats(&tmp) {
            return Err(EngineError::Invalid("copy mismatch".into()));
        }
        let copy_meta = InstanceMeta {
            name,
            kind: meta.kind,
            description: description.trim().to_string(),
            last_played: None,
            play_seconds: 0,
            role: meta.role,
        };
        write_instance_meta(&tmp, &copy_meta)?;
        fs::rename(&tmp, &target)?;
        Ok(())
    })();
    if let Err(e) = built {
        let _ = fs::remove_dir_all(&tmp);
        return Err(e);
    }
    if was_synced {
        crate::tray::link_new_instance_tray(game_root, code)?;
    }
    summary_of(game_root, code)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteStats {
    pub name: String,
    pub bytes: u64,
    pub files: u64,
    pub mods: u32,
    pub saves: u32,
    pub play_seconds: u64,
    pub kept_snapshots: u32,
}

fn gather(dir: &Path, out: &mut Vec<(PathBuf, bool)>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if is_link(&path) {
            out.push((path, false));
        } else if path.is_dir() {
            gather(&path, out);
            out.push((path, true));
        } else {
            out.push((path, false));
        }
    }
}

fn remove_one(path: &Path, is_dir: bool) -> Result<()> {
    let attempt = || -> std::io::Result<()> {
        if is_dir || (is_link(path) && path.is_dir()) {
            fs::remove_dir(path)
        } else {
            fs::remove_file(path)
        }
    };
    match attempt() {
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            if let Ok(meta) = fs::symlink_metadata(path) {
                let mut perms = meta.permissions();
                perms.set_readonly(false);
                let _ = fs::set_permissions(path, perms);
            }
            Ok(attempt()?)
        }
        other => Ok(other?),
    }
}

pub fn delete_instance(
    game_root: &Path,
    snapshots_root: &Path,
    code: &str,
    check_running: bool,
    progress: &dyn Fn(u64, u64),
) -> Result<DeleteStats> {
    if !valid_code(code) {
        return Err(EngineError::Invalid(format!("code: {code}")));
    }
    let (dir, meta) = registered(game_root, code)?;
    let summary = summary_of(game_root, code)?;
    let canon = fs::canonicalize(&dir)?;
    let root = fs::canonicalize(game_root)?;
    if is_link(&dir) || canon.parent() != Some(root.as_path()) {
        return Err(EngineError::Invalid("delete refused".into()));
    }

    let active = active_instance_id(game_root).is_some_and(|a| a.eq_ignore_ascii_case(code));
    if active && check_running && game_running() {
        return Err(EngineError::GameRunning);
    }

    let (files, bytes) = dir_stats(&dir);
    let mut entries = Vec::new();
    gather(&dir, &mut entries);
    let total = entries.len() as u64 + 1;

    if active {
        remove_junction(&game_dir(game_root))?;
    }

    progress(0, total);
    for (i, (path, is_dir)) in entries.iter().enumerate() {
        remove_one(path, *is_dir)?;
        progress(i as u64 + 1, total);
    }
    remove_one(&dir, true)?;
    progress(total, total);

    let kept = crate::snapshot::list_snapshots(snapshots_root, code).map(|l| l.len() as u32).unwrap_or(0);
    Ok(DeleteStats {
        name: meta.name,
        bytes,
        files,
        mods: summary.mod_count,
        saves: summary.save_count,
        play_seconds: summary.play_seconds,
        kept_snapshots: kept,
    })
}
