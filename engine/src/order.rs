use crate::error::Result;
use crate::model::InstanceSummary;
use std::fs;
use std::path::Path;

pub fn load_order(file: &Path) -> Vec<String> {
    fs::read(file)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save_order(file: &Path, ids: &[String]) -> Result<()> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = file.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(ids)?)?;
    fs::rename(&tmp, file)?;
    Ok(())
}

pub fn apply_order(list: &mut [InstanceSummary], order: &[String]) {
    let rank = |id: &str| {
        order
            .iter()
            .position(|o| o.eq_ignore_ascii_case(id))
            .unwrap_or(usize::MAX)
    };
    list.sort_by_key(|i| rank(&i.id));
}
