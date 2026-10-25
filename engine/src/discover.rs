use crate::error::Result;
use crate::fsutil::{find_child, is_link, walk_files};
use crate::model::{ActiveState, InstanceKind, InstanceMeta, InstanceSummary};
use std::fs;
use crate::gamefolder::legacy_prefix;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const META_FILE: &str = ".plumbob.json";
const MOD_EXTENSIONS: [&str; 2] = ["package", "ts4script"];

fn unix_secs(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn guess_kind(id: &str) -> InstanceKind {
    if id.to_ascii_lowercase().starts_with("mp") {
        InstanceKind::Mp
    } else {
        InstanceKind::Solo
    }
}

fn summarize(dir: &Path, id: &str, meta: Option<InstanceMeta>, legacy: bool) -> InstanceSummary {
    let mut size = 0u64;
    let mut mods = 0u32;
    let mut saves = 0u32;
    let mut tray_items = 0u32;
    let mut tray_synced = false;
    let mut newest = fs::metadata(dir).and_then(|m| m.modified()).map(unix_secs).unwrap_or(0);
    let mut top_mods: Vec<String> = Vec::new();

    if let Some(mods_dir) = find_child(dir, "Mods") {
        walk_files(&mods_dir, &mut |p, m| {
            size += m.len();
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
            if MOD_EXTENSIONS.contains(&ext.as_str()) {
                mods += 1;
            }
        });
        if let Ok(rd) = fs::read_dir(&mods_dir) {
            let mut names: Vec<String> = rd
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort_by_key(|n| n.to_ascii_lowercase());
            top_mods = names.into_iter().take(5).collect();
        }
    }
    if let Some(saves_dir) = find_child(dir, "saves") {
        walk_files(&saves_dir, &mut |p, m| {
            size += m.len();
            if p.extension().and_then(|e| e.to_str()) == Some("save") {
                saves += 1;
            }
            if let Ok(t) = m.modified() {
                newest = newest.max(unix_secs(t));
            }
        });
    }
    if let Some(tray) = find_child(dir, "Tray") {
        let linked = is_link(&tray);
        tray_synced = linked
            && dir.parent().is_some_and(|root| {
                matches!(
                    (fs::canonicalize(&tray), fs::canonicalize(crate::fsutil::shared_tray_path(root))),
                    (Ok(a), Ok(b)) if a == b
                )
            });
        walk_files(&tray, &mut |p, m| {
            if !linked {
                size += m.len();
            }
            if p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("trayitem")) {
                tray_items += 1;
            }
        });
    }

    let last_played = meta.as_ref().and_then(|m| m.last_played);
    let play_seconds = meta.as_ref().map(|m| m.play_seconds).unwrap_or(0);
    let (name, kind, description, role) = match meta {
        Some(m) => (m.name, m.kind, m.description, m.role),
        None => (id.to_string(), guess_kind(id), String::new(), None),
    };
    InstanceSummary {
        id: id.to_string(),
        name,
        kind,
        role,
        description,
        path: dir.to_path_buf(),
        legacy,
        mod_count: mods,
        save_count: saves,
        tray_count: tray_items,
        tray_synced,
        tray_old: find_child(dir, "TrayOld").is_some_and(|p| p.is_dir() && !is_link(&p) && crate::fsutil::dir_stats(&p).0 > 0),
        size_bytes: size,
        last_modified: newest,
        last_played,
        play_seconds,
        game_version: read_game_version(dir),
        top_mods,
    }
}

pub fn read_game_version(dir: &Path) -> Option<String> {
    let file = find_child(dir, "GameVersion.txt")?;
    let text = fs::read(file).ok()?;
    let text: String = String::from_utf8_lossy(&text).chars().filter(|c| *c != char::from(0u8)).collect();
    let token = text.split_whitespace().next()?;
    token
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.')
        .then(|| token.to_string())
}

pub fn list_instances(game_root: &Path) -> Result<Vec<InstanceSummary>> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(game_root) else { return Ok(out) };
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(prefix) = name.get(..legacy_prefix().len()) else { continue };
        let dir = entry.path();
        if name.len() <= legacy_prefix().len()
            || !prefix.eq_ignore_ascii_case(&legacy_prefix())
            || is_link(&dir)
            || !dir.is_dir()
        {
            continue;
        }
        let id = name[legacy_prefix().len()..].to_string();
        let meta = read_instance_meta(&dir)?;
        let legacy = meta.is_none();
        out.push(summarize(&dir, &id, meta, legacy));
    }
    out.sort_by_key(|i| i.name.to_ascii_lowercase());
    Ok(out)
}

pub fn read_instance_meta(dir: &Path) -> Result<Option<InstanceMeta>> {
    let file = dir.join(META_FILE);
    if !file.is_file() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&fs::read(file)?)?))
}

pub fn write_instance_meta(dir: &Path, meta: &InstanceMeta) -> Result<()> {
    fs::write(dir.join(META_FILE), serde_json::to_vec_pretty(meta)?)?;
    Ok(())
}

pub fn active_state(game_dir: &Path) -> ActiveState {
    if fs::symlink_metadata(game_dir).is_err() {
        return ActiveState::Missing;
    }
    if is_link(game_dir) {
        return match fs::read_link(game_dir) {
            Ok(t) => ActiveState::Junction(t),
            Err(_) => ActiveState::Folder,
        };
    }
    ActiveState::Folder
}
