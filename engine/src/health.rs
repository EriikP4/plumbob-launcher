use crate::error::{EngineError, Result};
use crate::fsutil::{find_child, is_link, walk_files};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModStatus {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthWarning {
    pub kind: String,
    pub count: u32,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub mods: Vec<ModStatus>,
    pub warnings: Vec<HealthWarning>,
    pub last_exception: Option<u64>,
    pub last_exception_file: Option<String>,
    pub last_exception_source: Option<String>,
    pub mod_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceEntry {
    pub name: String,
    pub rel: String,
    pub bytes: u64,
    pub files: u64,
    pub is_dir: bool,
}

fn ext_of(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

fn name_of(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn mtime(meta: &fs::Metadata) -> u64 {
    meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0)
}

fn version_in_name(name: &str) -> Option<String> {
    let b = name.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() && (i == 0 || !b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'v' || b[i - 1] == b'V') {
            let mut j = i;
            let mut dots = 0;
            while j < b.len() && (b[j].is_ascii_digit() || (b[j] == b'.' && j + 1 < b.len() && b[j + 1].is_ascii_digit())) {
                if b[j] == b'.' {
                    dots += 1;
                }
                j += 1;
            }
            if dots >= 1 {
                return Some(name[i..j].to_string());
            }
            i = j;
        } else {
            i += 1;
        }
    }
    None
}

fn read_text(path: &Path) -> String {
    fs::read(path).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
}

fn between(text: &str, start: &str, end: &str) -> Option<String> {
    let a = text.find(start)? + start.len();
    let b = text[a..].find(end)? + a;
    let v = text[a..b].trim();
    (!v.is_empty()).then(|| v.to_string())
}

fn mccc_version(scripts_dir: &Path) -> Option<String> {
    let log = read_text(&scripts_dir.join("mc_cmd_center.log"));
    let line = log.lines().rev().find(|l| l.contains("Initializing MC Command Center version:"))?;
    let v = line.split("version:").nth(1)?.trim().trim_end_matches('.').trim();
    (!v.is_empty()).then(|| v.to_string())
}

fn betterexceptions_version(mods: &Path) -> Option<String> {
    let dir = find_child(mods, "TMEX-Settings")?;
    let dat = read_text(&find_child(&dir, "BetterExceptions-XMLREADER.dat")?);
    between(&dat, "<BEversion>", "</BEversion>").map(|v| v.trim_start_matches('v').to_string())
}

fn wickedwhims_version(script: &Path) -> Option<String> {
    let file = fs::File::open(script).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    for name in ["wickedwhims/version_registry.pyc", "wickedwhims/version_control.pyc"] {
        let Ok(mut entry) = zip.by_name(name) else { continue };
        let mut data = Vec::new();
        if entry.read_to_end(&mut data).is_err() {
            continue;
        }
        let key = b"WickedWhims_v";
        if let Some(pos) = data.windows(key.len()).position(|w| w == key) {
            let v: String = data[pos + key.len()..]
                .iter()
                .take_while(|c| c.is_ascii_alphanumeric() || **c == b'.')
                .map(|c| *c as char)
                .collect();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

pub fn analyze_health(instance_dir: &Path) -> Health {
    let mods = find_child(instance_dir, "Mods");
    let mut found: HashMap<&'static str, PathBuf> = HashMap::new();
    let mut deep_scripts = Vec::new();
    let mut deep_packages = Vec::new();
    let mut archives = Vec::new();
    let mut seen: HashMap<(String, u64), Vec<String>> = HashMap::new();
    let mut mccc_report: Option<(u64, String)> = None;
    let mut mod_files = 0u32;

    if let Some(mods) = &mods {
        walk_files(mods, &mut |p, m| {
            let rel = p.strip_prefix(mods).unwrap_or(p);
            let depth = rel.components().count();
            let name = name_of(p);
            let lower = name.to_ascii_lowercase();
            let ext = ext_of(p);
            let rel_s = rel.to_string_lossy().replace('\\', "/");
            match ext.as_str() {
                "ts4script" => {
                    mod_files += 1;
                    if depth > 2 {
                        deep_scripts.push(rel_s.clone());
                    }
                    seen.entry((lower.clone(), m.len())).or_default().push(rel_s.clone());
                }
                "package" => {
                    mod_files += 1;
                    if depth > 6 {
                        deep_packages.push(rel_s.clone());
                    }
                    seen.entry((lower.clone(), m.len())).or_default().push(rel_s.clone());
                }
                "zip" | "rar" | "7z" => archives.push(rel_s.clone()),
                _ => {}
            }
            if lower == "mc_lastexception.html" {
                let t = mtime(m);
                if mccc_report.as_ref().is_none_or(|(c, _)| t > *c) {
                    mccc_report = Some((t, format!("Mods/{rel_s}")));
                }
            }
            if ext == "ts4script" {
                if lower == "mc_cmd_center.ts4script" {
                    found.entry("mccc").or_insert_with(|| p.to_path_buf());
                } else if lower.contains("wickedwhims_scripts") {
                    found.entry("ww").or_insert_with(|| p.to_path_buf());
                } else if lower.contains("betterexceptions") {
                    found.entry("be").or_insert_with(|| p.to_path_buf());
                } else if lower.contains("xmlinjector") {
                    found.entry("xml").or_insert_with(|| p.to_path_buf());
                }
            }
        });
    }
    let mut be_report: Option<(u64, String)> = None;
    let mut game_report: Option<(u64, String)> = None;
    for entry in fs::read_dir(instance_dir).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let n = name.to_ascii_lowercase();
        let Ok(m) = entry.metadata() else { continue };
        if !m.is_file() {
            continue;
        }
        let t = mtime(&m);
        if n == "be-exceptionreport.html" {
            be_report = Some((t, name));
        } else if n.starts_with("lastexception") && n.ends_with(".txt") && game_report.as_ref().is_none_or(|(c, _)| t > *c) {
            game_report = Some((t, name));
        }
    }
    let (last_exception, last_exception_file, last_exception_source) = if let (true, Some((t, f))) = (found.contains_key("be"), be_report) {
        (Some(t), Some(f), Some("betterexceptions".to_string()))
    } else if let (true, Some((t, f))) = (found.contains_key("mccc"), mccc_report) {
        (Some(t), Some(f), Some("mccc".to_string()))
    } else if let Some((t, f)) = game_report {
        (Some(t), Some(f), Some("game".to_string()))
    } else {
        (None, None, None)
    };

    let mods_dir = mods.clone().unwrap_or_default();
    let status = |id: &str, name: &str, key: &str, version: &dyn Fn(&Path) -> Option<String>| ModStatus {
        id: id.into(),
        name: name.into(),
        installed: found.contains_key(key),
        version: found.get(key).and_then(|p| version(p)),
    };
    let mod_list = vec![
        status("betterexceptions", "BetterExceptions", "be", &|p| {
            betterexceptions_version(&mods_dir).or_else(|| version_in_name(&name_of(p)))
        }),
        status("mccc", "MC Command Center", "mccc", &|p| {
            mccc_version(p.parent().unwrap_or(Path::new("."))).or_else(|| version_in_name(&name_of(p)))
        }),
        status("xmlinjector", "XML Injector", "xml", &|p| version_in_name(&name_of(p))),
        status("wickedwhims", "WickedWhims", "ww", &|p| wickedwhims_version(p)),
    ];

    let mut dups: Vec<String> = seen
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(_, v)| v.join(" = "))
        .collect();
    dups.sort();

    let mut warnings = Vec::new();
    for (kind, mut items) in [
        ("deepScripts", deep_scripts),
        ("deepPackages", deep_packages),
        ("archives", archives),
        ("duplicates", dups),
    ] {
        if items.is_empty() {
            continue;
        }
        items.sort();
        warnings.push(HealthWarning { kind: kind.into(), count: items.len() as u32, items: items.into_iter().take(8).collect() });
    }

    Health { mods: mod_list, warnings, last_exception, last_exception_file, last_exception_source, mod_files }
}

fn safe_rel(rel: &str) -> Result<PathBuf> {
    let p = PathBuf::from(rel);
    if p.components().all(|c| matches!(c, Component::Normal(_))) {
        Ok(p)
    } else {
        Err(EngineError::Invalid(format!("path: {rel}")))
    }
}

pub fn space_children(instance_dir: &Path, rel: &str) -> Result<Vec<SpaceEntry>> {
    let base = instance_dir.join(safe_rel(rel)?);
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(&base) else { return Ok(out) };
    for entry in rd.flatten() {
        let p = entry.path();
        if is_link(&p) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        let child_rel = if rel.is_empty() { name.clone() } else { format!("{}/{}", rel.trim_end_matches('/'), name) };
        if meta.is_dir() {
            let (files, bytes) = crate::fsutil::dir_stats(&p);
            out.push(SpaceEntry { name, rel: child_rel, bytes, files, is_dir: true });
        } else if meta.is_file() {
            out.push(SpaceEntry { name, rel: child_rel, bytes: meta.len(), files: 1, is_dir: false });
        }
    }
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    Ok(out)
}

pub fn space_top_files(instance_dir: &Path, limit: usize) -> Vec<SpaceEntry> {
    let mut all = Vec::new();
    walk_files(instance_dir, &mut |p, m| {
        let rel = p.strip_prefix(instance_dir).unwrap_or(p).to_string_lossy().replace('\\', "/");
        all.push(SpaceEntry { name: name_of(p), rel, bytes: m.len(), files: 1, is_dir: false });
    });
    all.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    all.truncate(limit);
    all
}

pub fn resolve_instance_file(instance_dir: &Path, rel: &str) -> Result<PathBuf> {
    let path = instance_dir.join(safe_rel(rel)?);
    if path.is_file() && !is_link(&path) {
        Ok(path)
    } else {
        Err(EngineError::NotFound(rel.to_string()))
    }
}
