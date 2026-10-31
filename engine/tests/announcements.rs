use s4im_engine::{parse_announcements, parse_date, sanitize_link, unread_count, ANNOUNCEMENTS_MAX_BYTES};
use serde_json::json;

fn today() -> time::Date {
    parse_date("2026-11-01").unwrap()
}

fn item(id: &str, date: &str) -> serde_json::Value {
    json!({ "id": id, "date": date, "title": { "es": "Titulo", "en": "Title" }, "body": { "es": "Cuerpo", "en": "Body" } })
}

fn parse(value: serde_json::Value, lang: &str) -> Vec<s4im_engine::Announcement> {
    parse_announcements(&value.to_string(), lang, "0.3.0", today())
}

#[test]
fn invalid_items_are_dropped_without_breaking_the_rest() {
    let mut bad_kind = item("c", "2026-10-01");
    bad_kind["type"] = json!("nope");
    let items = json!({ "version": 1, "items": [
        item("a", "2026-10-01"),
        { "id": "b", "date": "garbage", "title": { "en": "x" }, "body": { "en": "y" } },
        bad_kind,
        { "date": "2026-10-01" },
        "text",
        { "id": "e", "date": "2026-10-01", "title": { "es": "" }, "body": {} },
        item("a", "2026-10-02"),
        item("f", "2026-02-30"),
    ] });
    let out = parse(items, "en");
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].id, "a");
    assert_eq!(out[0].date, "2026-10-01");
}

#[test]
fn unknown_fields_are_tolerated_and_broken_json_is_empty() {
    let mut a = item("a", "2026-10-01");
    a["extra"] = json!({ "x": 1 });
    assert_eq!(parse(json!({ "version": 1, "future": true, "items": [a] }), "es").len(), 1);
    assert!(parse_announcements("not json", "es", "0.3.0", today()).is_empty());
    assert!(parse_announcements("{\"items\": 5}", "es", "0.3.0", today()).is_empty());
    assert!(parse_announcements("{}", "es", "0.3.0", today()).is_empty());
}

fn only_en_again() -> serde_json::Value {
    json!({ "id": "a", "date": "2026-10-01", "title": { "en": "English" }, "body": { "en": "Only" } })
}

fn only_es_again() -> serde_json::Value {
    json!({ "id": "b", "date": "2026-10-01", "title": { "es": "Solo" }, "body": { "es": "Solo" } })
}

#[test]
fn language_falls_back_to_english() {
    let only_en = json!({ "id": "a", "date": "2026-10-01", "title": { "en": "English" }, "body": { "en": "Only" } });
    let only_es = json!({ "id": "b", "date": "2026-10-01", "title": { "es": "Solo" }, "body": { "es": "Solo" } });
    let out = parse(json!({ "items": [only_en, only_es, item("c", "2026-10-01")] }), "es");
    assert_eq!(out.len(), 3);
    assert_eq!(out.iter().find(|i| i.id == "a").unwrap().title, "English");
    assert_eq!(out.iter().find(|i| i.id == "c").unwrap().title, "Titulo");
    let fr = parse(json!({ "items": [only_en_again(), only_es_again(), item("c", "2026-10-01")] }), "fr");
    assert_eq!(fr.len(), 2);
    assert!(fr.iter().all(|i| i.title == "English" || i.title == "Title"));
}

#[test]
fn expiry_and_version_filters_apply() {
    let mut expired = item("old", "2026-01-01");
    expired["expires"] = json!("2026-10-31");
    let mut last_day = item("last", "2026-01-01");
    last_day["expires"] = json!("2026-11-01");
    let mut future = item("future", "2026-01-01");
    future["minVersion"] = json!("0.4.0");
    let mut retired = item("retired", "2026-01-01");
    retired["maxVersion"] = json!("0.2.9");
    let mut inside = item("inside", "2026-01-01");
    inside["minVersion"] = json!("0.3.0");
    inside["maxVersion"] = json!("0.3.9");
    let mut broken = item("broken", "2026-01-01");
    broken["minVersion"] = json!("abc");
    let ids: Vec<String> = parse(json!({ "items": [expired, last_day, future, retired, inside, broken] }), "en").into_iter().map(|i| i.id).collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"last".to_string()) && ids.contains(&"inside".to_string()));
}

#[test]
fn pinned_first_then_newest_first() {
    let mut pinned = item("pin", "2026-01-01");
    pinned["pinned"] = json!(true);
    let out = parse(json!({ "items": [item("a", "2026-03-01"), pinned, item("b", "2026-05-01")] }), "en");
    let ids: Vec<&str> = out.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(ids, ["pin", "b", "a"]);
}

#[test]
fn limits_are_enforced() {
    let mut long = item("long", "2026-10-01");
    long["title"] = json!({ "en": "t".repeat(500) });
    long["body"] = json!({ "en": "b".repeat(5000) });
    let out = parse(json!({ "items": [long] }), "en");
    assert_eq!(out[0].title.chars().count(), 120);
    assert_eq!(out[0].body.chars().count(), 2000);

    let many: Vec<_> = (0..60).map(|i| item(&format!("i{i}"), "2026-10-01")).collect();
    assert_eq!(parse(json!({ "items": many }), "en").len(), 30);

    let huge = format!("{{\"items\": [], \"pad\": \"{}\"}}", "x".repeat(ANNOUNCEMENTS_MAX_BYTES));
    assert!(parse_announcements(&huge, "en", "0.3.0", today()).is_empty());
}

#[test]
fn control_characters_are_removed() {
    let mut a = item("a", "2026-10-01");
    a["title"] = json!({ "en": "Hi\u{0007}\u{001b}[31m there" });
    a["body"] = json!({ "en": "Line1\r\nLine2\u{0000}" });
    let out = parse(json!({ "items": [a] }), "en");
    assert_eq!(out[0].title, "Hi[31m there");
    assert_eq!(out[0].body, "Line1\nLine2");
}

#[test]
fn only_https_links_survive() {
    assert!(sanitize_link("https://github.com/EriikP4/plumbob-launcher").is_some());
    for bad in ["http://github.com", "javascript:alert(1)", "file:///C:/x", "https://", "https://user@evil.com", "https://a b.com", "ftp://x.com", "//evil.com", ""] {
        assert!(sanitize_link(bad).is_none(), "{bad}");
    }
    let mut with_bad_link = item("a", "2026-10-01");
    with_bad_link["link"] = json!({ "url": "http://insecure.example", "label": { "en": "Go" } });
    assert!(parse(json!({ "items": [with_bad_link] }), "en").is_empty());
    let mut with_link = item("b", "2026-10-01");
    with_link["link"] = json!({ "url": "https://example.com/a", "label": { "es": "Ir" } });
    let out = parse(json!({ "items": [with_link] }), "en");
    let link = out[0].link.as_ref().unwrap();
    assert_eq!(link.url, "https://example.com/a");
    assert_eq!(link.label, None);
}

#[test]
fn unread_count_ignores_read_ids() {
    let out = parse(json!({ "items": [item("a", "2026-10-01"), item("b", "2026-10-02"), item("c", "2026-10-03")] }), "en");
    assert_eq!(unread_count(&out, &[]), 3);
    assert_eq!(unread_count(&out, &["a".to_string(), "zzz".to_string()]), 2);
    assert_eq!(unread_count(&out, &["a".to_string(), "b".to_string(), "c".to_string()]), 0);
}
