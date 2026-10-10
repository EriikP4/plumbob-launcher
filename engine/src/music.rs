use crate::error::{EngineError, Result};
use crate::fsutil::find_child;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const MUSIC_DIR: &str = "Custom Music";
pub const AUDIO_EXT: [&str; 5] = ["mp3", "wav", "flac", "ogg", "m4a"];

pub const GENRES: [&str; 38] = [
    "Alternative", "Americana", "Backyard", "Baroque", "Batuu Radio", "Blues", "Carnaval Beats", "Classical",
    "Cottage Core", "Dinner Jazz", "DJ R3X", "Easy Listening", "Electronica", "Focus", "Hip Hop", "Island Radio",
    "Japanese Folk", "Kids", "Latin", "Latin Pop", "Lullaby", "Metal", "New Age", "NuDisco", "Pop", "R&B", "Ranch",
    "Romance", "Sim Retro", "Singer-Songwriter", "Soul", "Spooky", "Spop", "Strange Tunes", "Summer Strut",
    "Tween Pop", "Winter Holiday", "World",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub name: String,
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Genre {
    pub name: String,
    pub tracks: Vec<Track>,
}

fn is_audio(p: &Path) -> bool {
    p.extension().is_some_and(|e| AUDIO_EXT.iter().any(|a| e.eq_ignore_ascii_case(a)))
}

fn check_genre(genre: &str) -> Result<()> {
    if genre.is_empty()
        || genre.len() > 64
        || genre.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
        || genre.starts_with('.')
    {
        return Err(EngineError::Invalid(format!("genre: {genre}")));
    }
    Ok(())
}

pub fn list_music(instance_dir: &Path) -> Vec<Genre> {
    let Some(root) = find_child(instance_dir, MUSIC_DIR) else { return Vec::new() };
    let mut out = Vec::new();
    for entry in fs::read_dir(&root).into_iter().flatten().flatten() {
        let dir = entry.path();
        if !dir.is_dir() || crate::fsutil::is_link(&dir) {
            continue;
        }
        let mut tracks: Vec<Track> = fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|f| f.path().is_file() && is_audio(&f.path()))
            .map(|f| Track {
                name: f.file_name().to_string_lossy().into_owned(),
                path: f.path().to_string_lossy().into_owned(),
                bytes: f.metadata().map(|m| m.len()).unwrap_or(0),
            })
            .collect();
        if tracks.is_empty() {
            continue;
        }
        tracks.sort_by_key(|t| t.name.to_lowercase());
        out.push(Genre { name: entry.file_name().to_string_lossy().into_owned(), tracks });
    }
    out.sort_by_key(|g| g.name.to_lowercase());
    out
}

fn free_name(dir: &Path, file: &str) -> PathBuf {
    let first = dir.join(file);
    if !first.exists() {
        return first;
    }
    let p = Path::new(file);
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    (2..)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|c| !c.exists())
        .unwrap_or(first)
}

pub fn add_music(instance_dir: &Path, genre: &str, files: &[PathBuf], progress: &dyn Fn(u64, u64)) -> Result<u32> {
    check_genre(genre)?;
    if !instance_dir.is_dir() {
        return Err(EngineError::NotFound(instance_dir.display().to_string()));
    }
    let root = find_child(instance_dir, MUSIC_DIR).unwrap_or_else(|| instance_dir.join(MUSIC_DIR));
    let dir = find_child(&root, genre).unwrap_or_else(|| root.join(genre));
    fs::create_dir_all(&dir)?;
    let valid: Vec<&PathBuf> = files.iter().filter(|f| f.is_file() && is_audio(f)).collect();
    if valid.is_empty() {
        return Err(EngineError::Invalid("no audio files".into()));
    }
    let total = valid.len() as u64;
    progress(0, total);
    let mut copied = 0;
    for (i, src) in valid.iter().enumerate() {
        let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        fs::copy(src, free_name(&dir, &name))?;
        copied += 1;
        progress(i as u64 + 1, total);
    }
    Ok(copied)
}

pub fn resolve_track(instance_dir: &Path, path: &Path) -> Result<PathBuf> {
    let root = find_child(instance_dir, MUSIC_DIR).ok_or_else(|| EngineError::NotFound(MUSIC_DIR.into()))?;
    let canon_root = fs::canonicalize(&root)?;
    let canon = fs::canonicalize(path).map_err(|_| EngineError::NotFound(path.display().to_string()))?;
    let rel = canon.strip_prefix(&canon_root).map_err(|_| EngineError::Invalid("track outside music folder".into()))?;
    let ok = rel.components().count() == 2 && rel.components().all(|c| matches!(c, Component::Normal(_)));
    if ok && canon.is_file() && is_audio(&canon) {
        Ok(canon)
    } else {
        Err(EngineError::Invalid("track outside music folder".into()))
    }
}

pub fn delete_track(instance_dir: &Path, path: &Path) -> Result<()> {
    let target = resolve_track(instance_dir, path)?;
    trash::delete(&target).map_err(|e| EngineError::Invalid(format!("trash: {e}")))
}
