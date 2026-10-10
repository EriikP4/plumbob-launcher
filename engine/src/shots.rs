use crate::gamefolder::legacy_prefix;
use crate::error::Result;
use crate::fsutil::{find_child, is_link};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

const FOLDER_NAMES: [&str; 3] = ["Capturas de pantalla", "Screenshots", "Capturas"];
const EXTENSIONS: [&str; 3] = ["png", "jpg", "jpeg"];
const THUMB_BOX: (u32, u32) = (480, 300);
const INDEX_VERSION: u32 = 1;
pub const IMPORTED: &str = "~imported";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Shot {
    pub id: String,
    pub instance: String,
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub modified: i64,
    pub thumb: Option<PathBuf>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct IndexFile {
    version: u32,
    shots: Vec<Shot>,
}

struct Found {
    instance: String,
    path: PathBuf,
    size: u64,
    modified: i64,
}

pub(crate) fn fnv(text: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn collect(dir: &Path, instance: &str, out: &mut Vec<Found>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if is_link(&path) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            collect(&path, instance, out);
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if !meta.is_file() || !EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        out.push(Found { instance: instance.to_string(), path, size: meta.len(), modified });
    }
}

fn scan(game_root: &Path, imported_dir: &Path) -> Vec<Found> {
    let mut out = Vec::new();
    collect(imported_dir, IMPORTED, &mut out);
    let Ok(rd) = fs::read_dir(game_root) else { return out };
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
        let code = &name[legacy_prefix().len()..];
        for folder in FOLDER_NAMES {
            if let Some(shots) = find_child(&dir, folder) {
                collect(&shots, code, &mut out);
            }
        }
    }
    out
}

fn load(index_file: &Path) -> IndexFile {
    fs::read_to_string(index_file)
        .ok()
        .and_then(|s| serde_json::from_str::<IndexFile>(s.trim_start_matches('\u{feff}')).ok())
        .filter(|i| i.version == INDEX_VERSION)
        .unwrap_or_default()
}

fn save(index_file: &Path, index: &IndexFile) -> Result<()> {
    if let Some(parent) = index_file.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = index_file.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec(index)?)?;
    fs::rename(&tmp, index_file)?;
    Ok(())
}

fn make_thumb(source: &Path, thumb: &Path) -> Option<(u32, u32)> {
    let img = image::open(source).ok()?;
    let (w, h) = (img.width(), img.height());
    let small = img.thumbnail(THUMB_BOX.0, THUMB_BOX.1).to_rgb8();
    let file = fs::File::create(thumb).ok()?;
    let mut writer = std::io::BufWriter::new(file);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, 80);
    encoder.encode_image(&small).ok()?;
    Some((w, h))
}

pub fn index_exists(index_file: &Path) -> bool {
    index_file.is_file() && load(index_file).version == INDEX_VERSION
}

pub fn update_index(
    game_root: &Path,
    imported_dir: &Path,
    index_file: &Path,
    thumbs_dir: &Path,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Result<usize> {
    fs::create_dir_all(thumbs_dir)?;
    let found = scan(game_root, imported_dir);
    let old: HashMap<PathBuf, Shot> = load(index_file).shots.into_iter().map(|s| (s.path.clone(), s)).collect();

    let keep: Vec<Option<Shot>> = found
        .iter()
        .map(|f| {
            old.get(&f.path)
                .filter(|s| {
                    s.size == f.size
                        && s.modified == f.modified
                        && s.instance == f.instance
                        && s.thumb.as_ref().is_some_and(|t| t.is_file())
                })
                .cloned()
        })
        .collect();

    let total = found.len();
    let done = AtomicUsize::new(keep.iter().filter(|k| k.is_some()).count());
    progress(done.load(Ordering::SeqCst), total);
    let results: Mutex<Vec<Option<Shot>>> = Mutex::new(keep);
    let todo: Vec<usize> = results
        .lock()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, k)| k.is_none())
        .map(|(i, _)| i)
        .collect();
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).clamp(1, 8);

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let slot = next.fetch_add(1, Ordering::SeqCst);
                let Some(&i) = todo.get(slot) else { break };
                let f = &found[i];
                let id = fnv(&format!("{}|{}|{}", f.path.display(), f.size, f.modified));
                let thumb = thumbs_dir.join(format!("{id}.jpg"));
                let dims = make_thumb(&f.path, &thumb);
                let shot = Shot {
                    id,
                    instance: f.instance.clone(),
                    path: f.path.clone(),
                    name: f.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                    size: f.size,
                    modified: f.modified,
                    thumb: dims.map(|_| thumb),
                    width: dims.map(|d| d.0).unwrap_or(0),
                    height: dims.map(|d| d.1).unwrap_or(0),
                };
                results.lock().unwrap()[i] = Some(shot);
                progress(done.fetch_add(1, Ordering::SeqCst) + 1, total);
            });
        }
    });

    let mut shots: Vec<Shot> = results.into_inner().unwrap().into_iter().flatten().collect();
    shots.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.name.cmp(&b.name)));
    let live: std::collections::HashSet<&str> = shots.iter().map(|s| s.id.as_str()).collect();
    for stale in old.values().filter(|s| !live.contains(s.id.as_str())) {
        if let Some(t) = &stale.thumb {
            let _ = fs::remove_file(t);
        }
    }
    let count = shots.len();
    save(index_file, &IndexFile { version: INDEX_VERSION, shots })?;
    Ok(count)
}

pub fn list_shots(index_file: &Path, instance: Option<&str>) -> Vec<Shot> {
    let mut shots = load(index_file).shots;
    if let Some(code) = instance {
        shots.retain(|s| s.instance.eq_ignore_ascii_case(code));
    }
    shots
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub done: u32,
    pub skipped: u32,
}

fn image_extension(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    EXTENSIONS.contains(&ext.as_str())
}

fn valid_target_code(code: &str) -> bool {
    !code.is_empty() && code.len() <= 32 && code.chars().all(|c| c.is_ascii_alphanumeric())
}

fn target_dir(game_root: &Path, imported_dir: &Path, target: Option<&str>) -> Result<PathBuf> {
    match target {
        None => {
            fs::create_dir_all(imported_dir)?;
            Ok(imported_dir.to_path_buf())
        }
        Some(code) if code == IMPORTED => {
            fs::create_dir_all(imported_dir)?;
            Ok(imported_dir.to_path_buf())
        }
        Some(code) => {
            if !valid_target_code(code) {
                return Err(crate::error::EngineError::Invalid(format!("code: {code}")));
            }
            let instance = game_root.join(format!("{}{code}", legacy_prefix()));
            if !instance.is_dir() || is_link(&instance) {
                return Err(crate::error::EngineError::NotFound(code.to_string()));
            }
            for folder in FOLDER_NAMES {
                if let Some(existing) = find_child(&instance, folder) {
                    if existing.is_dir() && !is_link(&existing) {
                        return Ok(existing);
                    }
                }
            }
            let created = instance.join(FOLDER_NAMES[0]);
            fs::create_dir_all(&created)?;
            Ok(created)
        }
    }
}

fn unique_path(dir: &Path, file_name: &std::ffi::OsStr) -> PathBuf {
    let first = dir.join(file_name);
    if !first.exists() {
        return first;
    }
    let name = Path::new(file_name);
    let stem = name.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = name.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let mut n = 2;
    loop {
        let candidate = dir.join(format!("{stem} ({n}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

pub fn import_shots(
    game_root: &Path,
    imported_dir: &Path,
    sources: &[PathBuf],
    target: Option<&str>,
) -> Result<Transfer> {
    let dir = target_dir(game_root, imported_dir, target)?;
    let mut report = Transfer::default();
    for source in sources {
        let usable = source.is_file()
            && !is_link(source)
            && image_extension(source)
            && image::image_dimensions(source).is_ok();
        let Some(name) = source.file_name().filter(|_| usable) else {
            report.skipped += 1;
            continue;
        };
        let destination = unique_path(&dir, name);
        if fs::copy(source, &destination).is_ok() {
            report.done += 1;
        } else {
            let _ = fs::remove_file(&destination);
            report.skipped += 1;
        }
    }
    Ok(report)
}

fn inside_known_shots(source: &Path, game_root: &Path, imported_dir: &Path) -> bool {
    let Ok(canon) = fs::canonicalize(source) else { return false };
    if let Ok(imported) = fs::canonicalize(imported_dir) {
        if canon.parent() == Some(imported.as_path()) {
            return true;
        }
    }
    let Ok(root) = fs::canonicalize(game_root) else { return false };
    let Ok(relative) = canon.strip_prefix(&root) else { return false };
    let mut parts = relative.components().map(|c| c.as_os_str().to_string_lossy().into_owned());
    let Some(instance) = parts.next() else { return false };
    let is_instance = instance
        .get(..legacy_prefix().len())
        .is_some_and(|p| p.eq_ignore_ascii_case(&legacy_prefix()))
        && instance.len() > legacy_prefix().len();
    is_instance && parts.next().is_some_and(|folder| FOLDER_NAMES.iter().any(|f| f.eq_ignore_ascii_case(&folder)))
}

pub fn move_shots(
    game_root: &Path,
    imported_dir: &Path,
    sources: &[PathBuf],
    target: Option<&str>,
) -> Result<Transfer> {
    let dir = target_dir(game_root, imported_dir, target)?;
    let dir_canon = fs::canonicalize(&dir)?;
    let mut report = Transfer::default();
    for source in sources {
        let allowed = source.is_file()
            && !is_link(source)
            && image_extension(source)
            && inside_known_shots(source, game_root, imported_dir);
        let Some(name) = source.file_name().filter(|_| allowed) else {
            report.skipped += 1;
            continue;
        };
        let same_folder = fs::canonicalize(source)
            .ok()
            .and_then(|c| c.parent().map(|p| p == dir_canon.as_path()))
            .unwrap_or(false);
        if same_folder {
            report.skipped += 1;
            continue;
        }
        let destination = unique_path(&dir, name);
        let moved = fs::rename(source, &destination).is_ok() || {
            let copied = fs::copy(source, &destination).ok();
            let same_size = copied.is_some_and(|n| fs::metadata(source).map(|m| m.len() == n).unwrap_or(false));
            if same_size {
                fs::remove_file(source).is_ok()
            } else {
                let _ = fs::remove_file(&destination);
                false
            }
        };
        if moved {
            report.done += 1;
        } else {
            report.skipped += 1;
        }
    }
    Ok(report)
}

pub fn delete_shots_with(
    game_root: &Path,
    imported_dir: &Path,
    sources: &[PathBuf],
    remove: &dyn Fn(&Path) -> bool,
) -> Transfer {
    let mut report = Transfer::default();
    for source in sources {
        let allowed = source.is_file()
            && !is_link(source)
            && image_extension(source)
            && inside_known_shots(source, game_root, imported_dir);
        if allowed && remove(source) {
            report.done += 1;
        } else {
            report.skipped += 1;
        }
    }
    report
}

pub fn delete_shots(game_root: &Path, imported_dir: &Path, sources: &[PathBuf]) -> Transfer {
    delete_shots_with(game_root, imported_dir, sources, &|path| trash::delete(path).is_ok())
}
