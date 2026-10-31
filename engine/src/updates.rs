use base64::{engine::general_purpose::STANDARD, Engine};
use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const UPDATER_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEE2RTMyODU1N0ExMjRDNTMKUldSVFRCSjZWU2pqcHBObGtzZ0kwdW9VRWpWdkt5V25WL0RmeklXTUpjNi9FWU82aVBMVjB2ejcK";
pub const RELEASES_API: &str = "https://api.github.com/repos/EriikP4/plumbob-launcher/releases?per_page=20";
pub const DOWNLOAD_PREFIX: &str = "https://github.com/EriikP4/plumbob-launcher/releases/download/";
const INSTALLER_SUFFIX: &str = "-windows-x64-setup.exe";
const NOTES_LINES: usize = 15;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAsset {
    pub name: String,
    pub url: String,
    pub sig_url: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub tag: String,
    pub name: String,
    pub notes: String,
    pub page: String,
    pub published: Option<String>,
    pub installer: Option<UpdateAsset>,
}

pub fn parse_version(tag: &str) -> Option<(u32, u32, u32)> {
    let core = tag.trim().trim_start_matches(['v', 'V']);
    let core = core.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().map(|p| p.parse().ok()).unwrap_or(Some(0))?;
    let patch = parts.next().map(|p| p.parse().ok()).unwrap_or(Some(0))?;
    parts.next().is_none().then_some((major, minor, patch))
}

pub fn is_newer(candidate: &str, than: &str) -> bool {
    match (parse_version(candidate), parse_version(than)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

pub fn should_notify(version: &str, ignored: Option<&str>) -> bool {
    match ignored.filter(|i| !i.is_empty()) {
        Some(ignored) => is_newer(version, ignored),
        None => true,
    }
}

pub fn first_section(body: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        let heading = lower.trim_start_matches('#').trim();
        let is_full = trimmed.starts_with('#') && (heading.starts_with("full changelog") || heading.starts_with("changelog completo"));
        if trimmed == "---" || trimmed == "***" || is_full {
            return lines.join("\n").trim().to_string();
        }
        lines.push(line);
        if lines.len() >= NOTES_LINES {
            break;
        }
    }
    lines.join("\n").trim().to_string()
}

pub fn is_trusted_download(url: &str) -> bool {
    url.starts_with(DOWNLOAD_PREFIX) && !url.contains("..") && !url.contains('@')
}

pub fn pick_update(releases: &Value, current: &str) -> Option<UpdateInfo> {
    let current_version = parse_version(current)?;
    let mut best: Option<((u32, u32, u32), &Value)> = None;
    for release in releases.as_array()? {
        if release.get("draft").and_then(Value::as_bool).unwrap_or(false) {
            continue;
        }
        let Some(tag) = release.get("tag_name").and_then(Value::as_str) else { continue };
        let Some(version) = parse_version(tag) else { continue };
        if version <= current_version {
            continue;
        }
        if best.as_ref().map_or(true, |(b, _)| version > *b) {
            best = Some((version, release));
        }
    }
    let (version, release) = best?;
    let text = |key: &str| release.get(key).and_then(Value::as_str).map(str::to_string);
    let tag = text("tag_name")?;
    let assets = release.get("assets").and_then(Value::as_array).cloned().unwrap_or_default();
    let asset_url = |name: &str| {
        assets
            .iter()
            .find(|a| a.get("name").and_then(Value::as_str) == Some(name))
            .and_then(|a| a.get("browser_download_url").and_then(Value::as_str))
            .filter(|u| is_trusted_download(u))
            .map(str::to_string)
    };
    let installer = assets.iter().find_map(|a| {
        let name = a.get("name").and_then(Value::as_str)?;
        if !name.ends_with(INSTALLER_SUFFIX) {
            return None;
        }
        Some(UpdateAsset {
            name: name.to_string(),
            url: asset_url(name)?,
            sig_url: asset_url(&format!("{name}.sig"))?,
            size: a.get("size").and_then(Value::as_u64).unwrap_or(0),
        })
    });
    Some(UpdateInfo {
        version: format!("{}.{}.{}", version.0, version.1, version.2),
        name: text("name").filter(|n| !n.trim().is_empty()).unwrap_or_else(|| tag.clone()),
        notes: first_section(release.get("body").and_then(Value::as_str).unwrap_or("")),
        page: text("html_url").unwrap_or_default(),
        published: text("published_at"),
        tag,
        installer,
    })
}

pub fn verify_signature(pubkey_base64: &str, signature_base64: &str, data: &[u8]) -> bool {
    let decode = |text: &str| STANDARD.decode(text.trim()).ok().and_then(|b| String::from_utf8(b).ok());
    let (Some(key_text), Some(sig_text)) = (decode(pubkey_base64), decode(signature_base64)) else { return false };
    let (Ok(key), Ok(sig)) = (PublicKey::decode(&key_text), Signature::decode(&sig_text)) else { return false };
    key.verify(data, &sig, true).is_ok()
}
