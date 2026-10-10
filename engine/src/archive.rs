use crate::cache::{cache_items, clear_cache};
use crate::error::{EngineError, Result};
use crate::fsutil::{dir_stats, is_link};
use crate::instances::{active_instance_id, instance_dir, rename_with_retry, summary_of, valid_code};
use crate::manage::registered;
use crate::model::InstanceSummary;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

pub const ARCHIVE_DIR: &str = "Archived";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedInstance {
    #[serde(flatten)]
    pub summary: InstanceSummary,
    pub compressed: bool,
    pub archived_at: i64,
    pub archive_bytes: u64,
}

pub fn archive_root(game_root: &Path) -> PathBuf {
    game_root.join(ARCHIVE_DIR)
}

fn sidecar(game_root: &Path, code: &str) -> PathBuf {
    archive_root(game_root).join(format!("{code}.json"))
}

fn zip_path(game_root: &Path, code: &str) -> PathBuf {
    archive_root(game_root).join(format!("{code}.zip"))
}

fn folder_path(game_root: &Path, code: &str) -> PathBuf {
    archive_root(game_root).join(code)
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

struct Entry {
    rel: String,
    abs: PathBuf,
    is_dir: bool,
}

fn gather(base: &Path, dir: &Path, out: &mut Vec<Entry>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let abs = entry.path();
        if is_link(&abs) {
            continue;
        }
        let Ok(rel) = abs.strip_prefix(base) else { continue };
        let rel = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            out.push(Entry { rel, abs: abs.clone(), is_dir: true });
            gather(base, &abs, out);
        } else if meta.is_file() {
            out.push(Entry { rel, abs, is_dir: false });
        }
    }
}

fn write_zip(entries: &[Entry], tmp: &Path, progress: &dyn Fn(u64, u64)) -> Result<()> {
    let total = entries.len() as u64;
    let mut zip = ZipWriter::new(BufWriter::new(File::create(tmp)?));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(9));
    progress(0, total);
    for (i, e) in entries.iter().enumerate() {
        if e.is_dir {
            zip.add_directory(format!("{}/", e.rel), options).map_err(io::Error::other)?;
        } else {
            let big = fs::metadata(&e.abs)?.len() >= u32::MAX as u64;
            zip.start_file(e.rel.clone(), options.large_file(big)).map_err(io::Error::other)?;
            io::copy(&mut BufReader::new(File::open(&e.abs)?), &mut zip)?;
        }
        progress(i as u64 + 1, total);
    }
    let mut writer = zip.finish().map_err(io::Error::other)?;
    io::Write::flush(&mut writer)?;
    Ok(())
}

fn compress_dir(dir: &Path, target: &Path, progress: &dyn Fn(u64, u64)) -> Result<()> {
    let mut entries = Vec::new();
    gather(dir, dir, &mut entries);
    let tmp = target.with_extension("zip.part");
    let _ = fs::remove_file(&tmp);
    match write_zip(&entries, &tmp, progress) {
        Ok(()) => {
            fs::rename(&tmp, target)?;
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

fn extract_zip(zip_file: &Path, target: &Path, progress: &dyn Fn(u64, u64)) -> Result<()> {
    let mut archive = ZipArchive::new(BufReader::new(File::open(zip_file)?)).map_err(io::Error::other)?;
    let total = archive.len() as u64;
    fs::create_dir_all(target)?;
    progress(0, total);
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(io::Error::other)?;
        let Some(rel) = file.enclosed_name() else {
            return Err(EngineError::Invalid("archive corrupt".into()));
        };
        let out = target.join(rel);
        if file.is_dir() {
            fs::create_dir_all(&out)?;
        } else {
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }
            io::copy(&mut file, &mut BufWriter::new(File::create(&out)?))?;
        }
        progress(i as u64 + 1, total);
    }
    Ok(())
}

fn remove_links(dir: &Path) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let p = entry.path();
        if is_link(&p) {
            let _ = fs::remove_dir(&p);
        }
    }
}

fn read_sidecar(path: &Path) -> Option<ArchivedInstance> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

pub fn archive_instance(
    game_root: &Path,
    code: &str,
    clean_cache: bool,
    compress: bool,
    progress: &dyn Fn(u64, u64),
) -> Result<ArchivedInstance> {
    if !valid_code(code) {
        return Err(EngineError::Invalid(format!("code: {code}")));
    }
    let (dir, _) = registered(game_root, code)?;
    if is_link(&dir) {
        return Err(EngineError::Invalid("delete refused".into()));
    }
    if active_instance_id(game_root).is_some_and(|a| a.eq_ignore_ascii_case(code)) {
        return Err(EngineError::Invalid("archive active".into()));
    }
    let folder = folder_path(game_root, code);
    let zip = zip_path(game_root, code);
    if folder.exists() || zip.exists() || sidecar(game_root, code).exists() {
        return Err(EngineError::Invalid("archive exists".into()));
    }
    fs::create_dir_all(archive_root(game_root))?;

    if clean_cache {
        let ids: Vec<String> = cache_items(&dir).into_iter().map(|i| i.id).collect();
        clear_cache(&dir, &ids, &|_, _| {})?;
    }

    let mut summary = summary_of(game_root, code)?;
    summary.size_bytes = dir_stats(&dir).1;
    let mut record = ArchivedInstance { summary, compressed: compress, archived_at: now(), archive_bytes: 0 };

    if compress {
        compress_dir(&dir, &zip, progress)?;
        record.archive_bytes = fs::metadata(&zip)?.len();
        record.summary.path = zip;
        fs::write(sidecar(game_root, code), serde_json::to_vec_pretty(&record)?)?;
        remove_links(&dir);
        if fs::remove_dir_all(&dir).is_err() {
            return Err(EngineError::Invalid("archive incomplete".into()));
        }
    } else {
        progress(0, 1);
        rename_with_retry(&dir, &folder)?;
        record.archive_bytes = dir_stats(&folder).1;
        record.summary.path = folder;
        fs::write(sidecar(game_root, code), serde_json::to_vec_pretty(&record)?)?;
        progress(1, 1);
    }
    Ok(record)
}

pub fn list_archived(game_root: &Path) -> Vec<ArchivedInstance> {
    let Ok(rd) = fs::read_dir(archive_root(game_root)) else { return Vec::new() };
    let mut out: Vec<ArchivedInstance> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("json")))
        .filter_map(|p| read_sidecar(&p))
        .filter(|a| a.summary.path.exists())
        .collect();
    out.sort_by_key(|a| std::cmp::Reverse(a.archived_at));
    out
}

pub fn restore_archived(game_root: &Path, code: &str, progress: &dyn Fn(u64, u64)) -> Result<InstanceSummary> {
    if !valid_code(code) {
        return Err(EngineError::Invalid(format!("code: {code}")));
    }
    let record = read_sidecar(&sidecar(game_root, code)).ok_or_else(|| EngineError::NotFound(code.to_string()))?;
    let target = instance_dir(game_root, code);
    if target.exists() {
        return Err(EngineError::Invalid(format!("code already in use: {code}")));
    }
    if record.compressed {
        let zip = zip_path(game_root, code);
        if !zip.is_file() {
            return Err(EngineError::NotFound(zip.display().to_string()));
        }
        let tmp = game_root.join(format!("{}.restoring", target.file_name().unwrap_or_default().to_string_lossy()));
        let _ = fs::remove_dir_all(&tmp);
        if let Err(e) = extract_zip(&zip, &tmp, progress) {
            let _ = fs::remove_dir_all(&tmp);
            return Err(e);
        }
        if let Err(e) = rename_with_retry(&tmp, &target) {
            let _ = fs::remove_dir_all(&tmp);
            return Err(e);
        }
        let _ = fs::remove_file(&zip);
    } else {
        let folder = folder_path(game_root, code);
        if !folder.is_dir() {
            return Err(EngineError::NotFound(folder.display().to_string()));
        }
        progress(0, 1);
        rename_with_retry(&folder, &target)?;
        progress(1, 1);
    }
    let _ = fs::remove_file(sidecar(game_root, code));
    if record.summary.tray_synced {
        let _ = crate::tray::link_new_instance_tray(game_root, code);
    }
    summary_of(game_root, code)
}
