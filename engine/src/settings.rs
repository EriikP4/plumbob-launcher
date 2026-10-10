use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    #[default]
    Steam,
    Ea,
    Epic,
    Custom,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct S4mmSettings {
    pub enabled: bool,
    pub path: Option<PathBuf>,
    pub launch_with_game: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MultiplayerSettings {
    pub enabled: bool,
    pub launcher: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShotsSettings {
    pub quality_badges: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BootSettings {
    pub animation: bool,
    pub sound: bool,
}

impl Default for BootSettings {
    fn default() -> Self {
        BootSettings { animation: true, sound: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ThemeSettings {
    pub preset: String,
    pub accent: Option<String>,
    pub animations: bool,
    pub custom: BTreeMap<String, String>,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        ThemeSettings {
            preset: "plumbob".into(),
            accent: None,
            animations: true,
            custom: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub language: String,
    pub setup_done: bool,
    pub platform: Platform,
    pub steam_id: Option<String>,
    pub install_dir: Option<PathBuf>,
    pub custom_command: Option<String>,
    pub game_root: Option<PathBuf>,
    pub snapshots_root: Option<PathBuf>,
    pub s4mm: S4mmSettings,
    pub multiplayer: MultiplayerSettings,
    pub theme: ThemeSettings,
    pub boot: BootSettings,
    pub shots: ShotsSettings,
    pub tray_sync_default: bool,
    pub auto_snapshots: bool,
    pub hide_update_notice: bool,
    pub music_welcome_seen: bool,
    pub game_folder: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: "es".into(),
            setup_done: false,
            platform: Platform::Steam,
            steam_id: None,
            install_dir: None,
            custom_command: None,
            game_root: None,
            snapshots_root: None,
            s4mm: S4mmSettings::default(),
            multiplayer: MultiplayerSettings::default(),
            theme: ThemeSettings::default(),
            boot: BootSettings::default(),
            shots: ShotsSettings::default(),
            tray_sync_default: false,
            auto_snapshots: true,
            hide_update_notice: false,
            music_welcome_seen: false,
            game_folder: None,
        }
    }
}

pub fn load_settings(file: &Path) -> Settings {
    fs::read_to_string(file)
        .ok()
        .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default()
}

pub fn save_settings(file: &Path, settings: &Settings) -> Result<()> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = file.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    fs::rename(&tmp, file)?;
    Ok(())
}
