use crate::error::{EngineError, Result};
use crate::fsutil::{find_child, is_link};
use crate::instances::{check_new_code, create_instance, instance_dir, summary_of};
use crate::manage::registered;
use crate::model::{InstanceKind, InstanceSummary};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const GROUPS: [&str; 4] = ["wickedwhims", "basementaldrugs", "mccc", "translations"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModGroupInfo {
    pub id: String,
    pub files: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseScan {
    pub has_config: bool,
    pub config_files: Vec<String>,
    pub has_mods: bool,
    pub mods_files: u64,
    pub mods_bytes: u64,
    pub groups: Vec<ModGroupInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModEntry {
    pub name: String,
    pub rel: String,
    pub is_dir: bool,
    pub files: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DeriveSelection {
    pub config: bool,
    pub all_mods: bool,
    pub groups: Vec<String>,
    pub paths: Vec<String>,
}

fn normalize(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

fn is_translation(name: &str) -> bool {
    let lower = name.to_lowercase();
    if lower.starts_with("z_") {
        return true;
    }
    match lower.strip_prefix("1_") {
        Some(rest) => {
            let token = rest.chars().take_while(|c| c.is_ascii_alphabetic()).count();
            (2..=12).contains(&token)
        }
        None => false,
    }
}

pub(crate) fn group_of(name: &str) -> Option<&'static str> {
    if is_translation(name) {
        return Some("translations");
    }
    let n = normalize(name);
    if n.contains("wickedwhims") || n.contains("turbodriver") {
        Some("wickedwhims")
    } else if n.contains("basementaldrugs") {
        Some("basementaldrugs")
    } else if n.contains("mccc") || n.contains("mccommandcenter") || n.contains("mccmdcenter") {
        Some("mccc")
    } else {
        None
    }
}

pub(crate) fn mods_dir(base: &Path) -> Option<PathBuf> {
    find_child(base, "Mods").filter(|p| p.is_dir())
}

fn walk_plain(dir: &Path, rel: &Path, out: &mut Vec<(PathBuf, u64)>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if is_link(&path) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let next = rel.join(entry.file_name());
        if meta.is_dir() {
            walk_plain(&path, &next, out);
        } else if meta.is_file() {
            out.push((next, meta.len()));
        }
    }
}

fn walk_grouped(dir: &Path, rel: &Path, inherited: Option<&'static str>, out: &mut Vec<(PathBuf, u64, Option<&'static str>)>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if is_link(&path) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        let next = rel.join(&name);
        let group = inherited.or_else(|| group_of(&name));
        if meta.is_dir() {
            walk_grouped(&path, &next, group, out);
        } else if meta.is_file() {
            out.push((next, meta.len(), group));
        }
    }
}

fn config_files(base: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir(base).into_iter().flatten().flatten() {
        let path = entry.path();
        if !path.is_file() || is_link(&path) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name.ends_with(".ini") || name == "reticulatedsplinesview" {
            out.push(path);
        }
    }
    out.sort();
    out
}

pub fn scan_base(game_root: &Path, code: &str) -> Result<BaseScan> {
    let (base, _) = registered(game_root, code)?;
    let config: Vec<String> = config_files(&base)
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    let mut scan = BaseScan {
        has_config: !config.is_empty(),
        config_files: config,
        has_mods: false,
        mods_files: 0,
        mods_bytes: 0,
        groups: Vec::new(),
    };
    let Some(mods) = mods_dir(&base) else { return Ok(scan) };
    scan.has_mods = true;
    let mut files = Vec::new();
    walk_grouped(&mods, Path::new(""), None, &mut files);
    let mut totals: BTreeMap<&'static str, (u64, u64)> = BTreeMap::new();
    for (_, size, group) in &files {
        scan.mods_files += 1;
        scan.mods_bytes += size;
        if let Some(g) = group {
            let entry = totals.entry(g).or_default();
            entry.0 += 1;
            entry.1 += size;
        }
    }
    for id in GROUPS {
        if let Some((count, bytes)) = totals.get(id) {
            scan.groups.push(ModGroupInfo { id: id.to_string(), files: *count, bytes: *bytes });
        }
    }
    Ok(scan)
}

pub(crate) fn safe_rel(rel: &str) -> Result<PathBuf> {
    let path = PathBuf::from(rel.replace('/', "\\"));
    if path.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(EngineError::Invalid("derive path".into()));
    }
    Ok(path)
}

pub fn list_mod_dir(game_root: &Path, code: &str, rel: &str) -> Result<Vec<ModEntry>> {
    let (base, _) = registered(game_root, code)?;
    let mods = mods_dir(&base).ok_or_else(|| EngineError::Invalid("derive no mods".into()))?;
    let sub = if rel.is_empty() { PathBuf::new() } else { safe_rel(rel)? };
    let dir = mods.join(&sub);
    if !dir.is_dir() || is_link(&dir) {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)?.flatten() {
        let path = entry.path();
        if is_link(&path) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        let child = sub.join(&name).to_string_lossy().replace('\\', "/");
        if meta.is_dir() {
            let mut files = Vec::new();
            walk_plain(&path, Path::new(""), &mut files);
            out.push(ModEntry { name, rel: child, is_dir: true, files: files.len() as u64, bytes: files.iter().map(|f| f.1).sum() });
        } else if meta.is_file() {
            out.push(ModEntry { name, rel: child, is_dir: false, files: 1, bytes: meta.len() });
        }
    }
    out.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(out)
}

fn plan_mods(mods: &Path, sel: &DeriveSelection) -> Result<BTreeMap<PathBuf, u64>> {
    let mut plan: BTreeMap<PathBuf, u64> = BTreeMap::new();
    if sel.all_mods {
        let mut files = Vec::new();
        walk_plain(mods, Path::new(""), &mut files);
        plan.extend(files);
    } else if !sel.groups.is_empty() {
        let wanted: Vec<&str> = sel.groups.iter().map(String::as_str).collect();
        let mut files = Vec::new();
        walk_grouped(mods, Path::new(""), None, &mut files);
        for (rel, size, group) in files {
            if group.is_some_and(|g| wanted.contains(&g)) {
                plan.insert(rel, size);
            }
        }
    }
    for rel in &sel.paths {
        let sub = safe_rel(rel)?;
        let full = mods.join(&sub);
        if is_link(&full) {
            continue;
        }
        if full.is_file() {
            let size = fs::metadata(&full).map(|m| m.len()).unwrap_or(0);
            plan.insert(sub, size);
        } else if full.is_dir() {
            let mut files = Vec::new();
            walk_plain(&full, &sub, &mut files);
            plan.extend(files);
        }
    }
    plan.retain(|rel, _| !(rel.components().count() == 1 && rel.to_string_lossy().eq_ignore_ascii_case("Resource.cfg")));
    Ok(plan)
}

pub fn derive_instance(
    game_root: &Path,
    base_code: &str,
    name: &str,
    description: &str,
    code: &str,
    kind: InstanceKind,
    sel: &DeriveSelection,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<InstanceSummary> {
    let (base, _) = registered(game_root, base_code)?;
    check_new_code(game_root, code)?;
    let wants_mods = sel.all_mods || !sel.groups.is_empty() || !sel.paths.is_empty();
    let plan = if wants_mods {
        let mods = mods_dir(&base).ok_or_else(|| EngineError::Invalid("derive no mods".into()))?;
        plan_mods(&mods, sel)?
    } else {
        BTreeMap::new()
    };
    let configs = if sel.config { config_files(&base) } else { Vec::new() };
    let total = (plan.len() + configs.len()) as u64;

    create_instance(game_root, name, description, code, kind)?;
    let target = instance_dir(game_root, code);
    let built = (|| -> Result<()> {
        let mut done = 0u64;
        progress(0, total);
        for file in &configs {
            if let Some(file_name) = file.file_name() {
                fs::copy(file, target.join(file_name))?;
            }
            done += 1;
            progress(done, total);
        }
        if !plan.is_empty() {
            let target_mods = mods_dir(&target).unwrap_or_else(|| target.join("Mods"));
            let source_mods = mods_dir(&base).ok_or_else(|| EngineError::Invalid("derive no mods".into()))?;
            for rel in plan.keys() {
                let to = target_mods.join(rel);
                if let Some(parent) = to.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(source_mods.join(rel), &to)?;
                done += 1;
                progress(done, total);
            }
        }
        Ok(())
    })();
    if let Err(e) = built {
        if target != base && target.starts_with(game_root) {
            let _ = fs::remove_dir_all(&target);
        }
        return Err(e);
    }
    summary_of(game_root, code)
}

pub(crate) fn group_of_path(rel: &str) -> Option<&'static str> {
    let parts: Vec<&str> = rel.split(|c| c == '/' || c == '\\').filter(|p| !p.is_empty()).collect();
    parts.iter().find_map(|p| group_of(p))
}
