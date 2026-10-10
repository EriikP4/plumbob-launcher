use crate::error::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub fn find_child(dir: &Path, name: &str) -> Option<PathBuf> {
    fs::read_dir(dir).ok()?.flatten().find_map(|e| {
        e.file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(name)
            .then(|| e.path())
    })
}

pub const SHARED_TRAY_NAME: &str = "SharedTray";

pub fn shared_tray_path(game_root: &Path) -> PathBuf {
    game_root.join(SHARED_TRAY_NAME)
}

pub fn is_link(path: &Path) -> bool {
    let Ok(m) = fs::symlink_metadata(path) else { return false };
    if m.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        m.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    false
}

pub fn copy_dir(src: &Path, dst: &Path) -> Result<u64> {
    fs::create_dir_all(dst)?;
    let mut total = 0;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        if is_link(&from) {
            continue;
        }
        let to = dst.join(entry.file_name());
        let ft = entry.file_type()?;
        if ft.is_dir() {
            total += copy_dir(&from, &to)?;
        } else if ft.is_file() {
            total += fs::copy(&from, &to)?;
        }
    }
    Ok(total)
}

pub fn walk_files(dir: &Path, f: &mut dyn FnMut(&Path, &fs::Metadata)) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let p = entry.path();
        if is_link(&p) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            walk_files(&p, f);
        } else if meta.is_file() {
            f(&p, &meta);
        }
    }
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && !id.starts_with('.')
        && id
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' '))
}

pub fn dir_stats(dir: &Path) -> (u64, u64) {
    let mut files = 0;
    let mut bytes = 0;
    walk_files(dir, &mut |_, m| {
        files += 1;
        bytes += m.len();
    });
    (files, bytes)
}
