use crate::error::{EngineError, Result};
use crate::fsutil::find_child;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

pub const NO_INTRO_URL: &str = "https://chii.modthesims.info/getfile.php?file=1456917";
pub const MAX_ZIP_BYTES: u64 = 20 * 1024 * 1024;
const MAX_PACKAGE_BYTES: u64 = 40 * 1024 * 1024;
pub const MARKER: &str = "plumbob-nointro.txt";

fn safe_name(raw: &str) -> Option<String> {
    let name = raw.rsplit(['/', '\\']).next()?.trim();
    let ok = !name.is_empty()
        && name.len() <= 120
        && !name.starts_with('.')
        && name.to_lowercase().ends_with(".package")
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '(' | ')' | '[' | ']'));
    ok.then(|| name.to_string())
}

pub fn extract_package(zip_bytes: &[u8], mods_dir: &Path) -> Result<String> {
    if zip_bytes.len() < 4 || &zip_bytes[..2] != b"PK" {
        return Err(EngineError::Invalid("nointro not zip".into()));
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes)).map_err(|_| EngineError::Invalid("nointro not zip".into()))?;
    let mut found: Option<(String, Vec<u8>)> = None;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|_| EngineError::Invalid("nointro not zip".into()))?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = safe_name(entry.name()) else { continue };
        if entry.size() > MAX_PACKAGE_BYTES {
            return Err(EngineError::Invalid("nointro too big".into()));
        }
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry.by_ref().take(MAX_PACKAGE_BYTES + 1).read_to_end(&mut data)?;
        if data.len() as u64 > MAX_PACKAGE_BYTES {
            return Err(EngineError::Invalid("nointro too big".into()));
        }
        if data.len() < 4 || &data[..4] != b"DBPF" {
            return Err(EngineError::Invalid("nointro bad package".into()));
        }
        found = Some((name, data));
        break;
    }
    let (name, data) = found.ok_or_else(|| EngineError::Invalid("nointro no package".into()))?;
    fs::create_dir_all(mods_dir)?;
    let target = mods_dir.join(&name);
    let temp = mods_dir.join(format!("{name}.plumb-tmp"));
    fs::write(&temp, &data)?;
    if let Err(e) = fs::rename(&temp, &target) {
        let _ = fs::remove_file(&temp);
        return Err(e.into());
    }
    Ok(name)
}

pub fn record(instance_dir: &Path, file_name: &str) -> Result<()> {
    fs::write(instance_dir.join(MARKER), file_name)?;
    Ok(())
}

pub fn installed_file(instance_dir: &Path, mods_dir: &Path) -> Option<PathBuf> {
    if let Some(marker) = find_child(instance_dir, MARKER) {
        if let Ok(name) = fs::read_to_string(marker) {
            if let Some(found) = find_child(mods_dir, name.trim()) {
                return Some(found);
            }
        }
    }
    fs::read_dir(mods_dir).ok()?.flatten().map(|e| e.path()).find(|p| {
        p.is_file()
            && p.file_name().map(|n| n.to_string_lossy().to_lowercase()).is_some_and(|n| {
                let flat: String = n.chars().filter(|c| c.is_alphanumeric()).collect();
                n.ends_with(".package") && flat.contains("nointro")
            })
    })
}

pub fn remove_installed(instance_dir: &Path, mods_dir: &Path) -> Result<bool> {
    let Some(file) = installed_file(instance_dir, mods_dir) else { return Ok(false) };
    fs::remove_file(file)?;
    if let Some(marker) = find_child(instance_dir, MARKER) {
        let _ = fs::remove_file(marker);
    }
    Ok(true)
}
