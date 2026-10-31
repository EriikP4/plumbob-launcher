use crate::fsutil::find_child;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RootState {
    Ok,
    Inside,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootCheck {
    pub state: RootState,
    pub root: Option<PathBuf>,
    pub version: Option<String>,
}

const BIN_DIRS: [&str; 2] = ["Bin", "Bin_LE"];

fn bin_dir(root: &Path) -> Option<PathBuf> {
    let game = find_child(root, "Game")?;
    BIN_DIRS.iter().find_map(|name| {
        let bin = find_child(&game, name)?;
        (find_child(&bin, "TS4_x64.exe").is_some() || find_child(&bin, "Default.ini").is_some()).then_some(bin)
    })
}

pub fn game_exe(root: &Path) -> Option<PathBuf> {
    find_child(&bin_dir(root)?, "TS4_x64.exe")
}

pub fn resolve_game_root(path: &Path) -> Option<PathBuf> {
    check_game_root(path).root
}

pub fn read_install_version(root: &Path) -> Option<String> {
    let bin = bin_dir(root)?;
    let ini = fs::read(find_child(&bin, "Default.ini")?).ok()?;
    String::from_utf8_lossy(&ini).lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        k.trim().eq_ignore_ascii_case("gameversion").then(|| v.trim().to_string())
    })
}

pub fn check_game_root(path: &Path) -> RootCheck {
    if bin_dir(path).is_some() {
        return RootCheck { state: RootState::Ok, root: Some(path.to_path_buf()), version: read_install_version(path) };
    }
    for ancestor in path.ancestors().skip(1) {
        if bin_dir(ancestor).is_some() {
            return RootCheck { state: RootState::Inside, root: Some(ancestor.to_path_buf()), version: read_install_version(ancestor) };
        }
    }
    RootCheck { state: RootState::Invalid, root: None, version: None }
}
