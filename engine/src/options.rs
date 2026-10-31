use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const SPLINES: &[u8] = include_bytes!("../assets/ReticulatedSplinesView");
const MUSIC_MAX: u32 = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicChoice {
    #[default]
    Keep,
    Half,
    Off,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GameOptions {
    pub mods: bool,
    pub skip_dlc_intro: bool,
    pub no_telemetry: bool,
    pub music: MusicChoice,
    pub windowed: bool,
    pub skip_ea_intro: bool,
}

fn updates(opts: &GameOptions) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> = Vec::new();
    if opts.mods {
        out.push(("modsdisabled", "0".into()));
        out.push(("scriptmodsenabled", "1".into()));
    }
    if opts.skip_dlc_intro {
        for key in ["packguidanceenabled", "guidancepack", "tutorialenabled", "guidanceautoenabled"] {
            out.push((key, "0".into()));
        }
    }
    if opts.no_telemetry {
        out.push(("enabletelemetry", "0".into()));
    }
    let level = match opts.music {
        MusicChoice::Keep => None,
        MusicChoice::Half => Some(MUSIC_MAX.div_ceil(2)),
        MusicChoice::Off => Some(0),
    };
    if let Some(level) = level {
        out.push(("musiclevel", level.to_string()));
        out.push(("menumusiclevel", level.to_string()));
    }
    if opts.windowed {
        out.push(("fullscreen", "1".into()));
        out.push(("windowedfullscreen", "1".into()));
    }
    out
}

fn merge(existing: &str, changes: &[(&str, String)]) -> String {
    let mut lines: Vec<String> = existing.lines().map(|l| l.to_string()).collect();
    if !lines.iter().any(|l| l.trim().eq_ignore_ascii_case("[options]")) {
        lines.insert(0, "[options]".into());
    }
    for (key, value) in changes {
        let found = lines.iter().position(|l| {
            l.split_once('=').is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case(key))
        });
        match found {
            Some(i) => lines[i] = format!("{key} = {value}"),
            None => {
                let at = lines
                    .iter()
                    .position(|l| l.trim().eq_ignore_ascii_case("[options]"))
                    .map_or(lines.len(), |i| i + 1);
                lines.insert(at, format!("{key} = {value}"));
            }
        }
    }
    let mut out = lines.join("\r\n");
    out.push_str("\r\n");
    out
}

pub fn apply_game_options(instance_dir: &Path, opts: &GameOptions) -> Result<()> {
    let changes = updates(opts);
    if !changes.is_empty() {
        let file = instance_dir.join("Options.ini");
        let existing = fs::read_to_string(&file).unwrap_or_default();
        fs::write(&file, merge(&existing, &changes))?;
    }
    if opts.skip_dlc_intro {
        let splines = instance_dir.join("ReticulatedSplinesView");
        if !splines.exists() {
            fs::write(splines, SPLINES)?;
        }
    }
    Ok(())
}
