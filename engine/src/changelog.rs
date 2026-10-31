pub const CHANGELOG_BASE: &str = "https://raw.githubusercontent.com/EriikP4/plumbob-launcher/main/changelog/";
pub const MAX_CHANGELOG_BYTES: usize = 16 * 1024;
pub const MAX_CHANGELOG_CHARS: usize = 2000;

fn valid_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    (1..=3).contains(&parts.len()) && parts.iter().all(|p| !p.is_empty() && p.len() <= 6 && p.bytes().all(|b| b.is_ascii_digit()))
}

pub fn changelog_langs(lang: &str) -> Vec<&'static str> {
    match lang {
        "es" => vec!["es", "en"],
        _ => vec!["en"],
    }
}

pub fn changelog_url(base: &str, version: &str, lang: &str) -> Option<String> {
    let lang = changelog_langs(lang).into_iter().find(|l| *l == lang)?;
    valid_version(version).then(|| format!("{base}{version}.{lang}.md"))
}

pub fn clean_changelog(text: &str) -> Option<String> {
    let normalized = text.trim_start_matches('\u{feff}').replace("\r\n", "\n").replace('\r', "\n");
    let kept: String = normalized.chars().filter(|c| *c == '\n' || !c.is_control()).collect();
    let trimmed: String = kept.trim().chars().take(MAX_CHANGELOG_CHARS).collect();
    let trimmed = trimmed.trim().to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}

pub fn pick_changelog<F: FnMut(&str) -> Option<Vec<u8>>>(base: &str, version: &str, lang: &str, mut fetch: F) -> Option<String> {
    for l in changelog_langs(lang) {
        let Some(url) = changelog_url(base, version, l) else { continue };
        let Some(bytes) = fetch(&url) else { continue };
        if bytes.len() > MAX_CHANGELOG_BYTES {
            continue;
        }
        if let Some(text) = String::from_utf8(bytes).ok().and_then(|t| clean_changelog(&t)) {
            return Some(text);
        }
    }
    None
}
