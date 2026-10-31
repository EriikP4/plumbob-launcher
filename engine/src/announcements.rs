use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use time::{Date, Month, OffsetDateTime};

use crate::updates::parse_version;

pub const ANNOUNCEMENTS_URL: &str = "https://raw.githubusercontent.com/EriikP4/plumbob-launcher/main/announcements/announcements.json";
pub const MAX_JSON_BYTES: usize = 64 * 1024;
pub const MAX_ITEMS: usize = 30;
pub const MAX_ID: usize = 64;
pub const MAX_TITLE: usize = 120;
pub const MAX_BODY: usize = 2000;
pub const MAX_LABEL: usize = 60;
pub const MAX_LINK: usize = 300;
const KINDS: [&str; 4] = ["info", "update", "event", "warning"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnouncementLink {
    pub url: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Announcement {
    pub id: String,
    pub date: String,
    pub kind: String,
    pub pinned: bool,
    pub title: String,
    pub body: String,
    pub link: Option<AnnouncementLink>,
}

#[derive(Deserialize)]
struct RawLink {
    url: String,
    label: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawItem {
    id: String,
    date: String,
    #[serde(rename = "type")]
    kind: Option<String>,
    pinned: Option<bool>,
    title: HashMap<String, String>,
    body: HashMap<String, String>,
    link: Option<RawLink>,
    expires: Option<String>,
    min_version: Option<String>,
    max_version: Option<String>,
}

pub fn today() -> Date {
    OffsetDateTime::now_utc().date()
}

pub fn parse_date(text: &str) -> Option<Date> {
    let mut parts = text.trim().split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u8 = parts.next()?.parse().ok()?;
    let day: u8 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Date::from_calendar_date(year, Month::try_from(month).ok()?, day).ok()
}

pub fn sanitize_link(url: &str) -> Option<String> {
    let url = url.trim();
    let rest = url.strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    let ok = !host.is_empty()
        && !host.contains('@')
        && url.len() <= MAX_LINK
        && !url.chars().any(|c| c.is_whitespace() || c.is_control());
    ok.then(|| url.to_string())
}

fn clean(text: &str, max: usize) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let kept: String = normalized.chars().filter(|c| *c == '\n' || !c.is_control()).collect();
    kept.trim().chars().take(max).collect::<String>().trim().to_string()
}

fn pick(map: &HashMap<String, String>, lang: &str, max: usize) -> Option<String> {
    [lang, "en"]
        .iter()
        .filter_map(|l| map.get(*l))
        .map(|t| clean(t, max))
        .find(|t| !t.is_empty())
}

fn version_ok(version: (u32, u32, u32), min: &Option<String>, max: &Option<String>) -> Option<bool> {
    if let Some(m) = min {
        if version < parse_version(m)? {
            return Some(false);
        }
    }
    if let Some(m) = max {
        if version > parse_version(m)? {
            return Some(false);
        }
    }
    Some(true)
}

fn convert(raw: RawItem, lang: &str, version: (u32, u32, u32), today: Date) -> Option<Announcement> {
    let id = raw.id.trim().to_string();
    if id.is_empty() || id.chars().count() > MAX_ID || id.chars().any(|c| c.is_control()) {
        return None;
    }
    let date = parse_date(&raw.date)?;
    let kind = match raw.kind.as_deref() {
        None => "info".to_string(),
        Some(k) if KINDS.contains(&k) => k.to_string(),
        Some(_) => return None,
    };
    if let Some(e) = &raw.expires {
        if today > parse_date(e)? {
            return None;
        }
    }
    if !version_ok(version, &raw.min_version, &raw.max_version)? {
        return None;
    }
    let title = pick(&raw.title, lang, MAX_TITLE)?;
    let body = pick(&raw.body, lang, MAX_BODY).unwrap_or_default();
    let link = match raw.link {
        Some(l) => Some(AnnouncementLink { url: sanitize_link(&l.url)?, label: l.label.and_then(|m| pick(&m, lang, MAX_LABEL)) }),
        None => None,
    };
    Some(Announcement { id, date: date.to_string(), kind, pinned: raw.pinned.unwrap_or(false), title, body, link })
}

pub fn parse_announcements(json: &str, lang: &str, version: &str, today: Date) -> Vec<Announcement> {
    if json.len() > MAX_JSON_BYTES {
        return Vec::new();
    }
    let Some(version) = parse_version(version) else { return Vec::new() };
    let Ok(root) = serde_json::from_str::<Value>(json.trim_start_matches('\u{feff}')) else { return Vec::new() };
    let Some(items) = root.get("items").and_then(Value::as_array) else { return Vec::new() };
    let mut seen = BTreeSet::new();
    let mut out: Vec<Announcement> = Vec::new();
    for value in items.iter().take(MAX_ITEMS) {
        let Ok(raw) = serde_json::from_value::<RawItem>(value.clone()) else { continue };
        let Some(item) = convert(raw, lang, version, today) else { continue };
        if seen.insert(item.id.clone()) {
            out.push(item);
        }
    }
    out.sort_by(|a, b| b.pinned.cmp(&a.pinned).then_with(|| b.date.cmp(&a.date)));
    out
}

pub fn unread_count(items: &[Announcement], read: &[String]) -> usize {
    items.iter().filter(|i| !read.contains(&i.id)).count()
}

pub fn sample_announcements(lang: &str) -> Vec<Announcement> {
    let es = lang == "es";
    let text = |a: &str, b: &str| if es { a.to_string() } else { b.to_string() };
    let day = |n: i64| (today() - time::Duration::days(n)).to_string();
    let long_es = "Un evento de la comunidad.\n\nCon un párrafo largo para comprobar que se pliega y se expande correctamente en el panel de novedades. ".repeat(6);
    let long_en = "A community event.\n\nWith a long paragraph to check that it folds and expands correctly in the news panel. ".repeat(6);
    let item = |id: &str, n: i64, kind: &str, pinned: bool, title: String, body: String, link: Option<AnnouncementLink>| Announcement {
        id: id.into(),
        date: day(n),
        kind: kind.into(),
        pinned,
        title,
        body,
        link,
    };
    vec![
        item(
            "fake-pinned",
            0,
            "warning",
            true,
            text("Aviso importante de ejemplo", "Sample important notice"),
            text("Este anuncio está fijado.\n- Primer punto\n- Segundo punto con **negrita**", "This announcement is pinned.\n- First point\n- Second point with **bold**"),
            None,
        ),
        item(
            "fake-update",
            1,
            "update",
            false,
            text("Novedad de actualización de ejemplo", "Sample update news"),
            text("Hay una versión nueva con mejoras.", "A new version with improvements is available."),
            Some(AnnouncementLink { url: "https://github.com/EriikP4/plumbob-launcher/releases".into(), label: Some(text("Ver versiones", "View releases")) }),
        ),
        item("fake-event", 2, "event", false, text("Evento de ejemplo", "Sample event"), text(&long_es, &long_en), None),
        item("fake-info", 3, "info", false, text("Información de ejemplo", "Sample information"), text("Un mensaje informativo breve.", "A short informative message."), None),
    ]
}
