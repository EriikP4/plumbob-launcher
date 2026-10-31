use s4im_engine::{changelog_url, clean_changelog, pick_changelog, CHANGELOG_BASE};
use std::collections::HashMap;

#[test]
fn urls_follow_version_and_language() {
    assert_eq!(changelog_url(CHANGELOG_BASE, "0.3.0", "es").unwrap(), "https://raw.githubusercontent.com/EriikP4/plumbob-launcher/main/changelog/0.3.0.es.md");
    assert_eq!(changelog_url(CHANGELOG_BASE, "0.3.0", "en").unwrap(), "https://raw.githubusercontent.com/EriikP4/plumbob-launcher/main/changelog/0.3.0.en.md");
    assert!(changelog_url(CHANGELOG_BASE, "0.3.0", "fr").is_none());
    for bad in ["", "../x", "0.3.0/../..", "0.3.0-alpha", "a.b.c", "1.2.3.4", "0.3.0?x=1"] {
        assert!(changelog_url(CHANGELOG_BASE, bad, "en").is_none(), "{bad}");
    }
}

#[test]
fn spanish_falls_back_to_english_then_to_nothing() {
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();
    files.insert(changelog_url(CHANGELOG_BASE, "0.3.0", "en").unwrap(), b"- English".to_vec());
    let got = pick_changelog(CHANGELOG_BASE, "0.3.0", "es", |u| files.get(u).cloned());
    assert_eq!(got.as_deref(), Some("- English"));
    files.insert(changelog_url(CHANGELOG_BASE, "0.3.0", "es").unwrap(), "- Español".as_bytes().to_vec());
    let got = pick_changelog(CHANGELOG_BASE, "0.3.0", "es", |u| files.get(u).cloned());
    assert_eq!(got.as_deref(), Some("- Español"));
    let none = pick_changelog(CHANGELOG_BASE, "0.4.0", "es", |u| files.get(u).cloned());
    assert!(none.is_none());
    let english = pick_changelog(CHANGELOG_BASE, "0.3.0", "fr", |u| files.get(u).cloned());
    assert_eq!(english.as_deref(), Some("- English"));
}

#[test]
fn invalid_files_are_skipped() {
    let mut calls = Vec::new();
    let got = pick_changelog(CHANGELOG_BASE, "0.3.0", "es", |u| {
        calls.push(u.to_string());
        if u.ends_with(".es.md") { Some(vec![b'x'; 20_000]) } else { Some(b"  \n ".to_vec()) }
    });
    assert!(got.is_none());
    assert_eq!(calls.len(), 2);
    assert!(pick_changelog(CHANGELOG_BASE, "0.3.0", "en", |_| Some(vec![0xff, 0xfe, 0x00])).is_none());
}

#[test]
fn text_is_cleaned_and_limited() {
    assert_eq!(clean_changelog("\u{feff}- a\r\n- b\u{0007}\r\n").unwrap(), "- a\n- b");
    assert!(clean_changelog("   ").is_none());
    assert_eq!(clean_changelog(&"x".repeat(5000)).unwrap().chars().count(), 2000);
}
