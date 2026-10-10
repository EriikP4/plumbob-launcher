use crate::discover::{
    active_state, list_instances, read_instance_meta, write_instance_meta, META_FILE,
};
use crate::error::{EngineError, Result};
use crate::gamefolder::{game_folder_name, legacy_prefix};
use crate::fsutil::{copy_dir, dir_stats, find_child, is_link};
use crate::link::{create_junction, remove_junction};
use crate::model::{ActiveState, ImportMode, InstanceKind, InstanceMeta, InstanceSummary};
use crate::snapshot::{create_snapshot, prune_auto_snapshots, SnapshotKind};
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const GAME_PROCESS: &str = "TS4_x64.exe";
const KEEP_AUTO_SNAPSHOTS: usize = 10;
const RESOURCE_CFG: &str = concat!(
    "Priority 500\n",
    "PackedFile *.package\n",
    "PackedFile */*.package\n",
    "PackedFile */*/*.package\n",
    "PackedFile */*/*/*.package\n",
    "PackedFile */*/*/*/*.package\n",
    "PackedFile */*/*/*/*/*.package\n",
);

pub fn game_dir(game_root: &Path) -> PathBuf {
    game_root.join(game_folder_name())
}

pub fn instance_dir(game_root: &Path, code: &str) -> PathBuf {
    game_root.join(format!("{}{code}", legacy_prefix()))
}

pub fn valid_code(code: &str) -> bool {
    !code.is_empty() && code.len() <= 32 && code.chars().all(|c| c.is_ascii_alphanumeric())
}

pub(crate) fn check_new_code(game_root: &Path, code: &str) -> Result<()> {
    if !valid_code(code) {
        return Err(EngineError::Invalid(format!("code: {code}")));
    }
    let taken = fs::read_dir(game_root)
        .map(|rd| {
            rd.flatten().any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&format!("{}{code}", legacy_prefix()))
            })
        })
        .unwrap_or(false);
    if taken {
        return Err(EngineError::Invalid(format!("code already in use: {code}")));
    }
    Ok(())
}

pub(crate) fn check_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.len() > 80 {
        return Err(EngineError::Invalid("name".into()));
    }
    Ok(name.to_string())
}

pub(crate) fn summary_of(game_root: &Path, code: &str) -> Result<InstanceSummary> {
    list_instances(game_root)?
        .into_iter()
        .find(|i| i.id.eq_ignore_ascii_case(code))
        .ok_or_else(|| EngineError::NotFound(code.to_string()))
}

pub fn process_running(name: &str) -> bool {
    Command::new("tasklist")
        .raw_arg(format!("/FI \"IMAGENAME eq {name}\" /NH"))
        .creation_flags(0x0800_0000)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_ascii_lowercase().contains(&name.to_ascii_lowercase()))
        .unwrap_or(false)
}

pub fn game_running() -> bool {
    process_running(GAME_PROCESS)
}

pub fn active_instance_id(game_root: &Path) -> Option<String> {
    match active_state(&game_dir(game_root)) {
        ActiveState::Junction(target) => {
            let name = target.file_name()?.to_string_lossy().into_owned();
            name.get(legacy_prefix().len()..)
                .filter(|_| name.len() > legacy_prefix().len())
                .map(str::to_string)
        }
        _ => None,
    }
}

pub fn needs_setup(game_root: &Path) -> bool {
    active_state(&game_dir(game_root)) == ActiveState::Folder
}

pub(crate) fn rename_with_retry(from: &Path, to: &Path) -> Result<()> {
    for attempt in 0..5 {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                if attempt < 4 {
                    std::thread::sleep(std::time::Duration::from_millis(400));
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
    let root = from.parent().unwrap_or(from);
    let report = crate::diag::summary_line(&crate::diag::diagnose(root, from));
    Err(EngineError::Invalid(format!("access denied: {}\n[{report}]", from.display())))
}

pub fn register_main(
    game_root: &Path,
    snapshots_root: &Path,
    name: &str,
    description: &str,
    code: &str,
) -> Result<InstanceSummary> {
    let name = check_name(name)?;
    check_new_code(game_root, code)?;
    if game_running() {
        return Err(EngineError::GameRunning);
    }
    let current = game_dir(game_root);
    if active_state(&current) != ActiveState::Folder {
        return Err(EngineError::Invalid("no real game folder to register".into()));
    }
    let target = instance_dir(game_root, code);
    create_snapshot(&current, snapshots_root, code, "snapshot.beforeRegister", SnapshotKind::Auto, false)
        .map(|_| ())
        .or_else(|e| match e {
            EngineError::Invalid(_) => Ok(()),
            other => Err(other),
        })?;
    let meta = InstanceMeta {
        name,
        kind: InstanceKind::Solo,
        description: description.trim().to_string(),
        last_played: None,
        play_seconds: 0,
        role: None,
    };
    write_instance_meta(&current, &meta)?;
    if let Err(e) = rename_with_retry(&current, &target) {
        let _ = fs::remove_file(current.join(META_FILE));
        return Err(e);
    }
    if let Err(e) = create_junction(&current, &target) {
        let _ = fs::rename(&target, &current);
        let _ = fs::remove_file(current.join(META_FILE));
        return Err(e);
    }
    summary_of(game_root, code)
}

pub fn create_instance(
    game_root: &Path,
    name: &str,
    description: &str,
    code: &str,
    kind: InstanceKind,
) -> Result<InstanceSummary> {
    let name = check_name(name)?;
    check_new_code(game_root, code)?;
    let dir = instance_dir(game_root, code);
    fs::create_dir_all(dir.join("Mods"))?;
    fs::write(dir.join("Mods").join("Resource.cfg"), RESOURCE_CFG)?;
    let meta = InstanceMeta { name, kind, description: description.trim().to_string(), last_played: None, play_seconds: 0, role: None };
    if let Err(e) = write_instance_meta(&dir, &meta) {
        let _ = fs::remove_dir_all(&dir);
        return Err(e);
    }
    summary_of(game_root, code)
}

pub fn adopt_instance(
    game_root: &Path,
    code: &str,
    name: &str,
    description: &str,
    kind: InstanceKind,
) -> Result<InstanceSummary> {
    let name = check_name(name)?;
    let summary = summary_of(game_root, code)?;
    if !summary.legacy {
        return Err(EngineError::Invalid(format!("already registered: {code}")));
    }
    let meta = InstanceMeta { name, kind, description: description.trim().to_string(), last_played: None, play_seconds: 0, role: None };
    write_instance_meta(&summary.path, &meta)?;
    summary_of(game_root, code)
}

pub fn activate_instance(
    game_root: &Path,
    snapshots_root: &Path,
    code: &str,
    check_running: bool,
) -> Result<()> {
    let summary = summary_of(game_root, code)?;
    if read_instance_meta(&summary.path)?.is_none() {
        return Err(EngineError::Invalid(format!("not registered: {code}")));
    }
    let link = game_dir(game_root);
    let previous = match active_state(&link) {
        ActiveState::Folder => {
            return Err(EngineError::Invalid("game folder is not registered".into()))
        }
        ActiveState::Junction(t) => Some(t),
        ActiveState::Missing => None,
    };
    if previous.as_deref().map(|t| t.file_name()) == Some(summary.path.file_name()) {
        return Ok(());
    }
    if check_running && game_running() {
        return Err(EngineError::GameRunning);
    }
    create_snapshot(
        &summary.path,
        snapshots_root,
        &summary.id,
        "snapshot.beforeActivate",
        SnapshotKind::Auto,
        false,
    )
    .map(|_| ())
    .or_else(|e| match e {
        EngineError::Invalid(_) => Ok(()),
        other => Err(other),
    })?;
    let _ = prune_auto_snapshots(snapshots_root, &summary.id, KEEP_AUTO_SNAPSHOTS);
    if is_link(&link) {
        remove_junction(&link)?;
    }
    if let Err(e) = create_junction(&link, &summary.path) {
        if let Some(old) = previous {
            let _ = create_junction(&link, &old);
        }
        return Err(e);
    }
    Ok(())
}

fn looks_like_game_folder(dir: &Path) -> bool {
    ["saves", "Mods", "Tray", "Options.ini"]
        .iter()
        .any(|n| find_child(dir, n).is_some())
}

pub fn import_instance(
    game_root: &Path,
    source: &Path,
    mode: ImportMode,
    name: &str,
    description: &str,
    code: &str,
    kind: InstanceKind,
) -> Result<InstanceSummary> {
    let name = check_name(name)?;
    if !source.is_dir() {
        return Err(EngineError::NotFound(source.display().to_string()));
    }
    fs::create_dir_all(game_root)?;
    let src = fs::canonicalize(source)?;
    let root = fs::canonicalize(game_root)?;

    if src.parent() == Some(root.as_path()) {
        let folder = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if let Some(prefix) = folder.get(..legacy_prefix().len()) {
            if prefix.eq_ignore_ascii_case(&legacy_prefix()) && folder.len() > legacy_prefix().len() {
                return adopt_instance(game_root, &folder[legacy_prefix().len()..], &name, description, kind);
            }
        }
    }

    if is_link(source) || src == root || root.starts_with(&src) || src.starts_with(&root) {
        return Err(EngineError::Invalid("source not allowed".into()));
    }
    if !looks_like_game_folder(&src) {
        return Err(EngineError::Invalid("not a game folder".into()));
    }
    check_new_code(game_root, code)?;

    let target = instance_dir(game_root, code);
    let meta = InstanceMeta { name, kind, description: description.trim().to_string(), last_played: None, play_seconds: 0, role: None };

    if mode == ImportMode::Move && fs::rename(&src, &target).is_ok() {
        if let Err(e) = write_instance_meta(&target, &meta) {
            let _ = fs::rename(&target, &src);
            return Err(e);
        }
        return summary_of(game_root, code);
    }

    let tmp = game_root.join(format!(".import-tmp-{code}"));
    let _ = fs::remove_dir_all(&tmp);
    let copied = (|| -> Result<()> {
        copy_dir(&src, &tmp)?;
        if dir_stats(&src) != dir_stats(&tmp) {
            return Err(EngineError::Invalid("copy mismatch".into()));
        }
        write_instance_meta(&tmp, &meta)?;
        fs::rename(&tmp, &target)?;
        Ok(())
    })();
    if let Err(e) = copied {
        let _ = fs::remove_dir_all(&tmp);
        return Err(e);
    }
    if mode == ImportMode::Move && fs::remove_dir_all(&src).is_err() {
        return Err(EngineError::Invalid("move incomplete".into()));
    }
    summary_of(game_root, code)
}
