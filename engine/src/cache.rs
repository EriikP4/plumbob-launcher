use crate::error::{EngineError, Result};
use crate::fsutil::{find_child, is_link, walk_files};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheItem {
    pub id: String,
    pub bytes: u64,
    pub files: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearReport {
    pub freed: u64,
    pub failed: u64,
}

const FILE_ITEMS: [(&str, &str); 3] = [
    ("thumbs", "localthumbcache.package"),
    ("textures", "localsimtexturecache.package"),
    ("avatar", "avatarcache.package"),
];
const DIR_ITEMS: [(&str, &str); 3] = [("cache", "cache"), ("cachestr", "cachestr"), ("onlinethumbs", "onlinethumbnailcache")];

fn collect(dir: &Path, out: &mut Vec<(PathBuf, u64)>) {
    walk_files(dir, &mut |p, m| out.push((p.to_path_buf(), m.len())));
}

fn files_of(instance_dir: &Path, id: &str) -> Option<Vec<(PathBuf, u64)>> {
    let mut out = Vec::new();
    if let Some((_, name)) = FILE_ITEMS.iter().find(|(i, _)| *i == id) {
        let p = find_child(instance_dir, name)?;
        if is_link(&p) || !p.is_file() {
            return None;
        }
        let len = fs::metadata(&p).ok()?.len();
        out.push((p, len));
        return Some(out);
    }
    if let Some((_, name)) = DIR_ITEMS.iter().find(|(i, _)| *i == id) {
        let p = find_child(instance_dir, name)?;
        if is_link(&p) || !p.is_dir() {
            return None;
        }
        collect(&p, &mut out);
        return Some(out);
    }
    if id == "errors" {
        walk_files(instance_dir, &mut |p, m| {
            let n = p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            let numbered = |prefix: &str| {
                n.strip_prefix(prefix)
                    .and_then(|r| r.strip_suffix(".txt"))
                    .is_some_and(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()))
            };
            let named = matches!(
                n.as_str(),
                "be-exceptionreport-old.html" | "be-exceptionreport.html" | "mc_lastexception.html" | "be-patchscannerreport.html" | "mc_systeminfo.html"
            );
            if named || numbered("lastexception_") || numbered("lastuiexception_") {
                out.push((p.to_path_buf(), m.len()));
            }
        });
        return Some(out);
    }
    if id == "logs" {
        walk_files(instance_dir, &mut |p, m| {
            if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("log")) {
                out.push((p.to_path_buf(), m.len()));
            }
        });
        return Some(out);
    }
    None
}

pub fn cache_items(instance_dir: &Path) -> Vec<CacheItem> {
    ["thumbs", "textures", "avatar", "cache", "cachestr", "onlinethumbs", "errors", "logs"]
        .iter()
        .filter_map(|id| {
            let list = files_of(instance_dir, id)?;
            if (*id == "logs" || *id == "errors") && list.is_empty() {
                return None;
            }
            Some(CacheItem { id: id.to_string(), bytes: list.iter().map(|(_, b)| b).sum(), files: list.len() as u64 })
        })
        .collect()
}

fn remove_file(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            if let Ok(meta) = fs::symlink_metadata(path) {
                let mut perms = meta.permissions();
                perms.set_readonly(false);
                let _ = fs::set_permissions(path, perms);
            }
            fs::remove_file(path)
        }
        other => other,
    }
}

fn prune_empty_dirs(dir: &Path) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let p = entry.path();
        if !is_link(&p) && p.is_dir() {
            prune_empty_dirs(&p);
            let _ = fs::remove_dir(&p);
        }
    }
}

pub fn clear_cache(instance_dir: &Path, ids: &[String], progress: &dyn Fn(u64, u64)) -> Result<ClearReport> {
    if !instance_dir.is_dir() {
        return Err(EngineError::NotFound(instance_dir.display().to_string()));
    }
    let mut targets = Vec::new();
    let mut seen = Vec::new();
    for id in ids {
        if seen.contains(id) {
            continue;
        }
        seen.push(id.clone());
        match files_of(instance_dir, id) {
            Some(list) => targets.extend(list),
            None if FILE_ITEMS.iter().chain(DIR_ITEMS.iter()).any(|(i, _)| i == id) || id == "logs" || id == "errors" => {}
            None => return Err(EngineError::Invalid(format!("cache item: {id}"))),
        }
    }
    let total = targets.len() as u64;
    let mut report = ClearReport::default();
    progress(0, total);
    for (i, (path, bytes)) in targets.iter().enumerate() {
        match remove_file(path) {
            Ok(()) => report.freed += bytes,
            Err(_) => report.failed += 1,
        }
        progress(i as u64 + 1, total);
    }
    for (_, name) in DIR_ITEMS {
        if let Some(d) = find_child(instance_dir, name) {
            if seen.iter().any(|s| DIR_ITEMS.iter().any(|(i, n)| i == s && *n == name)) {
                prune_empty_dirs(&d);
            }
        }
    }
    Ok(report)
}
