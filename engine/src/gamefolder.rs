use crate::fsutil::{find_child, is_link};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

pub const DEFAULT_GAME_FOLDER: &str = "Los Sims 4";

static NAME: RwLock<String> = RwLock::new(String::new());

pub fn game_folder_name() -> String {
    let name = NAME.read().map(|g| g.clone()).unwrap_or_default();
    if name.is_empty() {
        DEFAULT_GAME_FOLDER.to_string()
    } else {
        name
    }
}

pub fn set_game_folder_name(name: &str) {
    if let Ok(mut guard) = NAME.write() {
        *guard = name.trim().to_string();
    }
}

pub fn legacy_prefix() -> String {
    format!("{}_", game_folder_name())
}

const MARKERS: [&str; 4] = ["Options.ini", "GameVersion.txt", "Mods", "saves"];

fn looks_like_game_data(dir: &Path) -> bool {
    is_link(dir) || MARKERS.iter().any(|m| find_child(dir, m).is_some())
}

pub fn detect_game_folders(game_root: &Path) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(game_root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let path = e.path();
            (!name.contains('_') && !name.starts_with('.') && path.is_dir() && looks_like_game_data(&path)).then_some(name)
        })
        .collect();
    found.sort();
    found
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameFolderState {
    Ok,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFolderCheck {
    pub state: GameFolderState,
    pub root: Option<PathBuf>,
    pub name: Option<String>,
}

pub fn check_game_folder(path: &Path) -> GameFolderCheck {
    let invalid = GameFolderCheck { state: GameFolderState::Invalid, root: None, name: None };
    let (Some(name), Some(root)) = (path.file_name(), path.parent()) else { return invalid };
    let name = name.to_string_lossy().into_owned();
    if name.contains('_') || !path.is_dir() || !looks_like_game_data(path) {
        return invalid;
    }
    GameFolderCheck { state: GameFolderState::Ok, root: Some(root.to_path_buf()), name: Some(name) }
}

pub fn resolve_game_folder(game_root: &Path, chosen: Option<&str>) -> Option<String> {
    if let Some(name) = chosen.filter(|n| !n.trim().is_empty()) {
        return Some(name.to_string());
    }
    let mut found = detect_game_folders(game_root);
    (found.len() == 1).then(|| found.remove(0))
}
