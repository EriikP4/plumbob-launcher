use crate::fsutil::{find_child, is_link};
use crate::savethumb::save_thumbnail;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveInfo {
    pub slot: u32,
    pub size_bytes: u64,
    pub modified: i64,
    pub versions: u32,
    pub thumb: Option<PathBuf>,
}

fn parse_slot(name: &str) -> Option<(u32, bool)> {
    let rest = name.strip_prefix("Slot_")?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() || digits.len() > 9 {
        return None;
    }
    let tail = &rest[digits.len()..];
    if !tail.starts_with(".save") {
        return None;
    }
    Some((digits.parse().ok()?, tail == ".save"))
}

pub fn list_saves(instance_dir: &Path, thumbs_dir: Option<&Path>) -> Vec<SaveInfo> {
    let Some(dir) = find_child(instance_dir, "saves") else { return Vec::new() };
    let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
    let mut slots: BTreeMap<u32, (SaveInfo, bool, Option<(PathBuf, u64, i64)>)> = BTreeMap::new();
    for entry in rd.flatten() {
        let path = entry.path();
        if is_link(&path) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let Some((slot, main)) = parse_slot(&entry.file_name().to_string_lossy()) else { continue };
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let item = slots.entry(slot).or_insert((
            SaveInfo { slot, size_bytes: 0, modified: 0, versions: 0, thumb: None },
            false,
            None,
        ));
        item.0.size_bytes += meta.len();
        item.0.modified = item.0.modified.max(modified);
        if main {
            item.1 = true;
            item.2 = Some((path.clone(), meta.len(), modified));
        } else {
            item.0.versions += 1;
        }
    }
    let mut out: Vec<SaveInfo> = slots
        .into_values()
        .filter(|(_, main, _)| *main)
        .map(|(mut info, _, file)| {
            if let (Some(dir), Some((path, size, modified))) = (thumbs_dir, file) {
                info.thumb = save_thumbnail(&path, size, modified, dir);
            }
            info
        })
        .collect();
    out.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.slot.cmp(&b.slot)));
    out
}
