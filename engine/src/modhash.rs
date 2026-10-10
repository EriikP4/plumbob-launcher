use crate::error::Result;
use crate::fsutil::{find_child, walk_files};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModScan {
    pub files: Vec<ModFile>,
    pub code: String,
    pub packages: u32,
    pub scripts: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModDiff {
    pub missing: Vec<ModFile>,
    pub extra: Vec<ModFile>,
    pub different: Vec<ModChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModChange {
    pub host: ModFile,
    pub local: ModFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    size: u64,
    mtime: u64,
    hash: String,
}

fn mtime_of(meta: &fs::Metadata) -> u64 {
    meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_nanos() as u64).unwrap_or(0)
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn is_mod_file(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("package") || e.eq_ignore_ascii_case("ts4script"))
}

pub fn code_of(files: &[ModFile]) -> String {
    let mut keys: Vec<String> = files.iter().map(|f| format!("{}\0{}\0{}", f.name.to_lowercase(), f.hash, f.size)).collect();
    keys.sort();
    let mut hasher = blake3::Hasher::new();
    for k in &keys {
        hasher.update(k.as_bytes());
        hasher.update(b"\n");
    }
    let hex = hasher.finalize().to_hex().to_ascii_uppercase();
    let groups: Vec<&str> = (0..4).map(|i| &hex[i * 5..i * 5 + 5]).collect();
    format!("PB1-{}-{}", files.len(), groups.join("-"))
}

pub fn parse_code(code: &str) -> Option<(u32, String)> {
    let c: String = code.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_uppercase();
    let rest = c.strip_prefix("PB1-")?;
    let (count, hash) = rest.split_once('-')?;
    let count: u32 = count.parse().ok()?;
    let hash = hash.replace('-', "");
    (hash.len() == 20 && hash.chars().all(|c| c.is_ascii_hexdigit())).then_some((count, hash))
}

pub fn codes_match(a: &str, b: &str) -> bool {
    match (parse_code(a), parse_code(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

pub fn scan_mods(instance_dir: &Path, cache_file: Option<&Path>, progress: &(dyn Fn(u64, u64) + Sync)) -> Result<ModScan> {
    let Some(mods) = find_child(instance_dir, "Mods") else {
        return Ok(ModScan { files: Vec::new(), code: code_of(&[]), packages: 0, scripts: 0 });
    };
    let mut found: Vec<(PathBuf, u64, u64)> = Vec::new();
    walk_files(&mods, &mut |p, m| {
        if is_mod_file(p) {
            found.push((p.to_path_buf(), m.len(), mtime_of(m)));
        }
    });

    let old: HashMap<String, CacheEntry> = cache_file
        .and_then(|f| fs::read_to_string(f).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();

    let total = found.len() as u64;
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<(ModFile, CacheEntry)>>> = Mutex::new(vec![None; found.len()]);
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 8);
    progress(0, total);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some((path, size, mtime)) = found.get(i) else { break };
                let rel = path.strip_prefix(&mods).unwrap_or(path).to_string_lossy().replace('\\', "/");
                let key = rel.to_lowercase();
                let hash = match old.get(&key) {
                    Some(c) if c.size == *size && c.mtime == *mtime => Some(c.hash.clone()),
                    _ => hash_file(path).ok(),
                };
                if let Some(hash) = hash {
                    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    let entry = CacheEntry { size: *size, mtime: *mtime, hash: hash.clone() };
                    let file = ModFile { name, path: rel, size: *size, hash };
                    if let Ok(mut r) = results.lock() {
                        r[i] = Some((file, entry));
                    }
                }
                let d = done.fetch_add(1, Ordering::SeqCst) + 1;
                progress(d as u64, total);
            });
        }
    });

    let results = results.into_inner().unwrap_or_default();
    let mut files = Vec::new();
    let mut fresh: HashMap<String, CacheEntry> = HashMap::new();
    for (file, entry) in results.into_iter().flatten() {
        fresh.insert(file.path.to_lowercase(), entry);
        files.push(file);
    }
    files.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
    if let Some(f) = cache_file {
        if let Some(dir) = f.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string(&fresh) {
            let _ = fs::write(f, text);
        }
    }
    let packages = files.iter().filter(|f| f.name.to_ascii_lowercase().ends_with(".package")).count() as u32;
    let scripts = files.len() as u32 - packages;
    Ok(ModScan { code: code_of(&files), files, packages, scripts })
}

pub fn compare_mods(host: &[ModFile], local: &[ModFile]) -> ModDiff {
    let mut diff = ModDiff::default();
    let local_names: HashSet<String> = local.iter().map(|f| f.name.to_lowercase()).collect();
    let host_names: HashSet<String> = host.iter().map(|f| f.name.to_lowercase()).collect();
    let local_pairs: HashSet<(String, &str)> = local.iter().map(|f| (f.name.to_lowercase(), f.hash.as_str())).collect();
    let host_pairs: HashSet<(String, &str)> = host.iter().map(|f| (f.name.to_lowercase(), f.hash.as_str())).collect();

    for h in host {
        let key = h.name.to_lowercase();
        if local_pairs.contains(&(key.clone(), h.hash.as_str())) {
            continue;
        }
        if local_names.contains(&key) {
            if let Some(l) = local.iter().find(|l| l.name.to_lowercase() == key && !host_pairs.contains(&(key.clone(), l.hash.as_str()))) {
                diff.different.push(ModChange { host: h.clone(), local: l.clone() });
            }
        } else {
            diff.missing.push(h.clone());
        }
    }
    for l in local {
        let key = l.name.to_lowercase();
        if !host_names.contains(&key) {
            diff.extra.push(l.clone());
        }
    }
    diff
}
