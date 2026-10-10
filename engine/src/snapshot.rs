use crate::error::{EngineError, Result};
use crate::fsutil::{copy_dir, find_child, valid_id};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use time::macros::format_description;
use time::OffsetDateTime;

use std::sync::atomic::{AtomicBool, Ordering};

static AUTO_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn set_auto_snapshots(on: bool) {
    AUTO_ENABLED.store(on, Ordering::Relaxed);
}

const CORE_ITEMS: [&str; 2] = ["saves", "Tray"];
const MODS_ITEM: &str = "Mods";
const META_FILE: &str = "snapshot.json";
const DATA_DIR: &str = "data";
const RESTORE_STAGE: &str = ".restore-tmp";
const STALE_TMP_AGE: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotKind {
    Manual,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotInfo {
    pub id: String,
    pub instance_id: String,
    pub label: String,
    pub kind: SnapshotKind,
    pub created: i64,
    pub size_bytes: u64,
    pub includes_mods: bool,
    pub items: Vec<String>,
}

fn check_id(id: &str, what: &str) -> Result<()> {
    if valid_id(id) {
        Ok(())
    } else {
        Err(EngineError::Invalid(format!("{what}: {id}")))
    }
}

fn instance_snapshots_dir(root: &Path, instance_id: &str) -> PathBuf {
    root.join(instance_id)
}

fn unique_id(dir: &Path, now: OffsetDateTime) -> String {
    let base = now
        .format(format_description!("[year][month][day]-[hour][minute][second]"))
        .unwrap_or_else(|_| now.unix_timestamp().to_string());
    let mut id = base.clone();
    let mut n = 2;
    while dir.join(&id).exists() {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

pub fn clean_stale_tmp(base: &Path, max_age: Duration) -> usize {
    let Ok(rd) = fs::read_dir(base) else { return 0 };
    let mut removed = 0;
    for entry in rd.flatten() {
        if !entry.file_name().to_string_lossy().starts_with(".tmp-") || !entry.path().is_dir() {
            continue;
        }
        let age = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .unwrap_or(Duration::MAX);
        if age >= max_age && fs::remove_dir_all(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

pub fn create_snapshot(
    instance_dir: &Path,
    root: &Path,
    instance_id: &str,
    label: &str,
    kind: SnapshotKind,
    include_mods: bool,
) -> Result<SnapshotInfo> {
    check_id(instance_id, "instance id")?;
    if kind == SnapshotKind::Auto
        && !AUTO_ENABLED.load(Ordering::Relaxed)
        && matches!(label, "snapshot.beforeActivate" | "snapshot.sessionClose")
    {
        return Err(EngineError::Invalid("auto snapshots off".into()));
    }
    if !instance_dir.is_dir() {
        return Err(EngineError::NotFound(instance_dir.display().to_string()));
    }
    let base = instance_snapshots_dir(root, instance_id);
    fs::create_dir_all(&base)?;
    clean_stale_tmp(&base, STALE_TMP_AGE);
    let now = OffsetDateTime::now_utc();
    let id = unique_id(&base, now);
    let tmp = base.join(format!(".tmp-{id}"));
    let data = tmp.join(DATA_DIR);

    let build = || -> Result<SnapshotInfo> {
        fs::create_dir_all(&data)?;
        let mut wanted: Vec<&str> = CORE_ITEMS.to_vec();
        if include_mods {
            wanted.push(MODS_ITEM);
        }
        let mut items = Vec::new();
        let mut size = 0;
        for name in wanted {
            if let Some(src) = find_child(instance_dir, name) {
                if src.is_dir() && !crate::fsutil::is_link(&src) {
                    let actual = src.file_name().unwrap().to_string_lossy().into_owned();
                    size += copy_dir(&src, &data.join(&actual))?;
                    items.push(actual);
                }
            }
        }
        if items.is_empty() {
            return Err(EngineError::Invalid("nothing to back up".into()));
        }
        let info = SnapshotInfo {
            id: id.clone(),
            instance_id: instance_id.to_string(),
            label: label.to_string(),
            kind,
            created: now.unix_timestamp(),
            size_bytes: size,
            includes_mods: include_mods,
            items,
        };
        fs::write(tmp.join(META_FILE), serde_json::to_vec_pretty(&info)?)?;
        fs::rename(&tmp, base.join(&id))?;
        Ok(info)
    };

    build().inspect_err(|_| {
        let _ = fs::remove_dir_all(&tmp);
    })
}

fn read_info(dir: &Path) -> Result<SnapshotInfo> {
    Ok(serde_json::from_slice(&fs::read(dir.join(META_FILE))?)?)
}

pub fn list_snapshots(root: &Path, instance_id: &str) -> Result<Vec<SnapshotInfo>> {
    check_id(instance_id, "instance id")?;
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(instance_snapshots_dir(root, instance_id)) else { return Ok(out) };
    for entry in rd.flatten() {
        if entry.file_name().to_string_lossy().starts_with('.') || !entry.path().is_dir() {
            continue;
        }
        if let Ok(info) = read_info(&entry.path()) {
            out.push(info);
        }
    }
    out.sort_by(|a, b| b.created.cmp(&a.created).then_with(|| b.id.cmp(&a.id)));
    Ok(out)
}

pub fn restore_snapshot(
    instance_dir: &Path,
    root: &Path,
    instance_id: &str,
    snapshot_id: &str,
) -> Result<SnapshotInfo> {
    check_id(instance_id, "instance id")?;
    check_id(snapshot_id, "snapshot id")?;
    let snap_dir = instance_snapshots_dir(root, instance_id).join(snapshot_id);
    if !snap_dir.is_dir() {
        return Err(EngineError::NotFound(snapshot_id.to_string()));
    }
    let info = read_info(&snap_dir)?;
    let safety = create_snapshot(
        instance_dir,
        root,
        instance_id,
        &format!("snapshot.beforeRestore:{snapshot_id}"),
        SnapshotKind::Auto,
        info.includes_mods,
    )?;

    let stage = instance_dir.join(RESTORE_STAGE);
    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    let staged = (|| -> Result<()> {
        for item in &info.items {
            copy_dir(&snap_dir.join(DATA_DIR).join(item), &stage.join(item))?;
        }
        Ok(())
    })();
    if let Err(e) = staged {
        let _ = fs::remove_dir_all(&stage);
        return Err(e);
    }
    for item in &info.items {
        let target = find_child(instance_dir, item).unwrap_or_else(|| instance_dir.join(item));
        if crate::fsutil::is_link(&target) {
            continue;
        }
        if target.exists() {
            fs::remove_dir_all(&target)?;
        }
        fs::rename(stage.join(item), &target)?;
    }
    fs::remove_dir_all(&stage)?;
    Ok(safety)
}

pub fn delete_snapshot(root: &Path, instance_id: &str, snapshot_id: &str) -> Result<()> {
    check_id(instance_id, "instance id")?;
    check_id(snapshot_id, "snapshot id")?;
    let dir = instance_snapshots_dir(root, instance_id).join(snapshot_id);
    if !dir.join(META_FILE).is_file() {
        return Err(EngineError::NotFound(snapshot_id.to_string()));
    }
    fs::remove_dir_all(dir)?;
    Ok(())
}

pub fn prune_auto_snapshots(root: &Path, instance_id: &str, keep: usize) -> Result<Vec<String>> {
    let mut removed = Vec::new();
    let autos: Vec<SnapshotInfo> = list_snapshots(root, instance_id)?
        .into_iter()
        .filter(|s| s.kind == SnapshotKind::Auto)
        .collect();
    for old in autos.into_iter().skip(keep) {
        delete_snapshot(root, instance_id, &old.id)?;
        removed.push(old.id);
    }
    Ok(removed)
}
