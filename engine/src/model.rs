use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InstanceKind {
    Solo,
    Mp,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Host,
    Client,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImportMode {
    Move,
    Copy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceMeta {
    pub name: String,
    pub kind: InstanceKind,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_played: Option<i64>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub play_seconds: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSummary {
    pub id: String,
    pub name: String,
    pub kind: InstanceKind,
    pub role: Option<Role>,
    pub description: String,
    pub path: PathBuf,
    pub legacy: bool,
    pub mod_count: u32,
    pub save_count: u32,
    pub tray_count: u32,
    pub tray_synced: bool,
    pub tray_old: bool,
    pub size_bytes: u64,
    pub last_modified: i64,
    pub last_played: Option<i64>,
    pub play_seconds: u64,
    pub game_version: Option<String>,
    pub top_mods: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "target", rename_all = "lowercase")]
pub enum ActiveState {
    Missing,
    Folder,
    Junction(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub game_root: PathBuf,
    pub game_dir: PathBuf,
    pub snapshots_root: PathBuf,
    pub order_file: PathBuf,
    pub settings_file: PathBuf,
    pub shots_index: PathBuf,
    pub thumbs_dir: PathBuf,
    pub save_thumbs: PathBuf,
    pub imported_shots: PathBuf,
}

impl Config {
    pub fn apply(mut self, settings: &crate::settings::Settings) -> Config {
        if let Some(root) = &settings.game_root {
            self.game_root = root.clone();
        }
        self.game_dir = self.game_root.join(crate::gamefolder::game_folder_name());
        if let Some(snaps) = &settings.snapshots_root {
            self.snapshots_root = snaps.clone();
        }
        self
    }

    pub fn detect() -> Option<Config> {
        let game_root = dirs::document_dir()?.join("Electronic Arts");
        let data = dirs::data_local_dir()?.join("Plumbob Launcher");
        Some(Config {
            game_dir: game_root.join(crate::gamefolder::game_folder_name()),
            game_root,
            snapshots_root: data.join("snapshots"),
            order_file: data.join("order.json"),
            settings_file: data.join("settings.json"),
            shots_index: data.join("screenshots").join("index.json"),
            thumbs_dir: data.join("screenshots").join("thumbs"),
            save_thumbs: data.join("save-thumbs"),
            imported_shots: data.join("screenshots").join("imported"),
        })
    }
}
