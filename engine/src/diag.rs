use crate::fsutil::walk_files;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

const ATTR_OFFLINE: u32 = 0x1000;
const ATTR_RECALL_ON_OPEN: u32 = 0x4_0000;
const ATTR_RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;
const ACCESS_DELETE: u32 = 0x1_0000;
const SHARE_ALL: u32 = 7;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub game_root: PathBuf,
    pub game_dir: PathBuf,
    pub game_dir_exists: bool,
    pub one_drive: bool,
    pub cloud_files: u64,
    pub locked_files: u64,
    pub locked_sample: Option<PathBuf>,
    pub write_error: Option<String>,
    pub game_running: bool,
}

fn under_one_drive(path: &Path) -> bool {
    let mut candidates = vec![path.to_path_buf()];
    if let Ok(canon) = fs::canonicalize(path) {
        candidates.push(canon);
    }
    let roots: Vec<PathBuf> = ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"]
        .iter()
        .filter_map(|v| std::env::var_os(v))
        .map(PathBuf::from)
        .collect();
    candidates.iter().any(|c| {
        c.to_string_lossy().to_ascii_lowercase().contains("\\onedrive")
            || roots.iter().any(|r| c.starts_with(r))
    })
}

fn write_probe(root: &Path) -> Option<String> {
    let pid = std::process::id();
    let a = root.join(format!(".plumbob-probe-{pid}-a"));
    let b = root.join(format!(".plumbob-probe-{pid}-b"));
    let _ = fs::remove_dir_all(&a);
    let _ = fs::remove_dir_all(&b);
    let result = fs::create_dir(&a).and_then(|_| fs::rename(&a, &b));
    let _ = fs::remove_dir_all(&a);
    let _ = fs::remove_dir_all(&b);
    result.err().map(|e| e.to_string())
}

pub fn diagnose(game_root: &Path, game_dir: &Path) -> Diagnostics {
    let mut cloud_files = 0u64;
    let mut locked_files = 0u64;
    let mut locked_sample = None;
    let exists = game_dir.is_dir();
    if exists {
        walk_files(game_dir, &mut |p, m| {
            let attrs = m.file_attributes();
            if attrs & (ATTR_OFFLINE | ATTR_RECALL_ON_OPEN | ATTR_RECALL_ON_DATA_ACCESS) != 0 {
                cloud_files += 1;
                return;
            }
            let free = OpenOptions::new().access_mode(ACCESS_DELETE).share_mode(SHARE_ALL).open(p).is_ok();
            if !free {
                locked_files += 1;
                if locked_sample.is_none() {
                    locked_sample = Some(p.to_path_buf());
                }
            }
        });
    }
    Diagnostics {
        game_root: game_root.to_path_buf(),
        game_dir: game_dir.to_path_buf(),
        game_dir_exists: exists,
        one_drive: under_one_drive(game_root),
        cloud_files,
        locked_files,
        locked_sample,
        write_error: write_probe(game_root),
        game_running: crate::instances::game_running(),
    }
}

pub fn summary_line(d: &Diagnostics) -> String {
    let mut parts = vec![
        format!("OneDrive={}", if d.one_drive { "yes" } else { "no" }),
        format!("cloud={}", d.cloud_files),
        format!("locked={}", d.locked_files),
        format!("game={}", if d.game_running { "running" } else { "closed" }),
    ];
    if let Some(p) = &d.locked_sample {
        parts.push(format!("first={}", p.display()));
    }
    if let Some(e) = &d.write_error {
        parts.push(format!("probe={e}"));
    }
    parts.join(" · ")
}
