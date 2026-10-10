use crate::error::{EngineError, Result};
use crate::fsutil::is_link;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn create_junction(link: &Path, target: &Path) -> Result<()> {
    if fs::symlink_metadata(link).is_ok() {
        return Err(EngineError::Invalid(format!("already exists: {}", link.display())));
    }
    if !target.is_dir() {
        return Err(EngineError::NotFound(target.display().to_string()));
    }
    let out = Command::new("cmd")
        .raw_arg(format!(
            "/S /C \"mklink /J \"{}\" \"{}\"\"",
            link.display(),
            target.display()
        ))
        .output()?;
    if out.status.success() {
        Ok(())
    } else {
        Err(EngineError::Invalid(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ))
    }
}

pub fn junction_target(link: &Path) -> Result<PathBuf> {
    if !is_link(link) {
        return Err(EngineError::Invalid(format!("not a link: {}", link.display())));
    }
    Ok(fs::read_link(link)?)
}

pub fn remove_junction(link: &Path) -> Result<()> {
    if !is_link(link) {
        return Err(EngineError::Invalid(format!("not a link: {}", link.display())));
    }
    fs::remove_dir(link)?;
    Ok(())
}
