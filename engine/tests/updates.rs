use s4im_engine::*;
use serde_json::json;
use std::fs;

const BASE: &str = "https://github.com/EriikP4/plumbob-launcher/releases/download";

fn asset(tag: &str, name: &str) -> serde_json::Value {
    json!({ "name": name, "size": 3213637, "browser_download_url": format!("{BASE}/{tag}/{name}") })
}

fn release(tag: &str, body: &str, draft: bool) -> serde_json::Value {
    let version = tag.trim_start_matches('v').split('-').next().unwrap().to_string();
    let setup = format!("Plumbob-Launcher-{version}-windows-x64-setup.exe");
    json!({
        "tag_name": tag,
        "name": format!("Plumbob Launcher {version}"),
        "draft": draft,
        "prerelease": true,
        "html_url": format!("https://github.com/EriikP4/plumbob-launcher/releases/tag/{tag}"),
        "published_at": "2026-11-01T10:00:00Z",
        "body": body,
        "assets": [
            asset(tag, &setup),
            asset(tag, &format!("{setup}.sig")),
            asset(tag, &format!("Plumbob-Launcher-{version}-windows-x64-portable.zip")),
        ],
    })
}

#[test]
fn versions_compare_ignoring_the_alpha_suffix() {
    assert_eq!(parse_version("v0.3.0-alpha"), Some((0, 3, 0)));
    assert_eq!(parse_version("0.2.1"), Some((0, 2, 1)));
    assert_eq!(parse_version("V1.10.2+build5"), Some((1, 10, 2)));
    assert_eq!(parse_version("v1.2"), Some((1, 2, 0)));
    assert_eq!(parse_version("nightly"), None);
    assert_eq!(parse_version("1.2.3.4"), None);
    assert!(is_newer("v0.3.0-alpha", "0.2.1"));
    assert!(is_newer("0.10.0", "0.9.9"));
    assert!(!is_newer("v0.2.1-alpha", "0.2.1"));
    assert!(!is_newer("0.2.0", "0.2.1"));
    assert!(!is_newer("garbage", "0.2.1"));
}

#[test]
fn the_newest_non_draft_release_is_picked_with_its_installer_and_signature() {
    let releases = json!([
        release("v0.4.0-alpha", "draft", true),
        release("v0.2.1-alpha", "same version", false),
        release("v0.3.0-alpha", "New\n- a thing", false),
        release("v0.2.0-alpha", "old", false),
    ]);
    let update = pick_update(&releases, "0.2.1").unwrap();
    assert_eq!(update.version, "0.3.0");
    assert_eq!(update.tag, "v0.3.0-alpha");
    assert_eq!(update.page, "https://github.com/EriikP4/plumbob-launcher/releases/tag/v0.3.0-alpha");
    let installer = update.installer.unwrap();
    assert_eq!(installer.name, "Plumbob-Launcher-0.3.0-windows-x64-setup.exe");
    assert_eq!(installer.url, format!("{BASE}/v0.3.0-alpha/Plumbob-Launcher-0.3.0-windows-x64-setup.exe"));
    assert_eq!(installer.sig_url, format!("{}.sig", installer.url));
    assert_eq!(installer.size, 3213637);
    assert!(pick_update(&releases, "0.3.0").is_none());
    assert!(pick_update(&json!([]), "0.2.1").is_none());
    assert!(pick_update(&json!({ "message": "rate limited" }), "0.2.1").is_none());
}

#[test]
fn a_release_without_signature_or_with_foreign_assets_has_no_installer() {
    let mut unsigned = release("v0.3.0-alpha", "x", false);
    unsigned["assets"] = json!([asset("v0.3.0-alpha", "Plumbob-Launcher-0.3.0-windows-x64-setup.exe")]);
    let update = pick_update(&json!([unsigned]), "0.2.1").unwrap();
    assert!(update.installer.is_none());

    let mut foreign = release("v0.3.0-alpha", "x", false);
    foreign["assets"] = json!([
        { "name": "Plumbob-Launcher-0.3.0-windows-x64-setup.exe", "size": 1, "browser_download_url": "https://evil.example/Plumbob-Launcher-0.3.0-windows-x64-setup.exe" },
        { "name": "Plumbob-Launcher-0.3.0-windows-x64-setup.exe.sig", "size": 1, "browser_download_url": "https://evil.example/x.sig" },
    ]);
    assert!(pick_update(&json!([foreign]), "0.2.1").unwrap().installer.is_none());
}

#[test]
fn only_the_first_section_of_the_changelog_is_kept() {
    let body = "## New\n- one\n- two\n\n---\n\n## Older\n- three";
    assert_eq!(first_section(body), "## New\n- one\n- two");
    let full = "## New\n- one\n\n## Full Changelog\nhttps://github.com/x/compare";
    assert_eq!(first_section(full), "## New\n- one");
    let long: String = (1..=40).map(|n| format!("- line {n}\n")).collect();
    let cut = first_section(&long);
    assert_eq!(cut.lines().count(), 15);
    assert!(cut.starts_with("- line 1"));
    assert_eq!(first_section(""), "");
}

#[test]
fn downloads_are_only_trusted_from_this_repository() {
    assert!(is_trusted_download(&format!("{BASE}/v0.3.0-alpha/a.exe")));
    assert!(!is_trusted_download("https://github.com/OtherUser/plumbob-launcher/releases/download/v1/a.exe"));
    assert!(!is_trusted_download("http://github.com/EriikP4/plumbob-launcher/releases/download/v1/a.exe"));
    assert!(!is_trusted_download("https://github.com/EriikP4/plumbob-launcher/releases/download/../../x"));
    assert!(!is_trusted_download("https://github.com.evil.example/EriikP4/plumbob-launcher/releases/download/v1/a.exe"));
}

#[test]
fn signatures_verify_only_with_the_matching_key_and_untampered_data() {
    let dir = format!("{}/tests/fixtures/updates", env!("CARGO_MANIFEST_DIR"));
    let payload = fs::read(format!("{dir}/payload.bin")).unwrap();
    let sig = fs::read_to_string(format!("{dir}/payload.bin.sig")).unwrap();
    let test_key = fs::read_to_string(format!("{dir}/test.key.pub")).unwrap();
    let test_key_b64 = test_key.trim().to_string();
    assert!(verify_signature(&test_key_b64, &sig, &payload));

    let mut tampered = payload.clone();
    tampered[0] ^= 1;
    assert!(!verify_signature(&test_key_b64, &sig, &tampered));
    assert!(!verify_signature(UPDATER_PUBKEY, &sig, &payload));
    assert!(!verify_signature(&test_key_b64, "bm90IGEgc2lnbmF0dXJl", &payload));
    assert!(!verify_signature(&test_key_b64, "%%%", &payload));
    assert!(!verify_signature("", &sig, &payload));
}

#[test]
fn an_ignored_version_stays_quiet_until_a_newer_one_appears() {
    assert!(should_notify("0.3.0", None));
    assert!(should_notify("0.3.0", Some("")));
    assert!(!should_notify("0.3.0", Some("0.3.0")));
    assert!(!should_notify("0.3.0", Some("v0.3.0-alpha")));
    assert!(!should_notify("0.3.0", Some("0.4.0")));
    assert!(should_notify("0.3.1", Some("0.3.0")));
    assert!(should_notify("0.4.0", Some("0.3.5")));
}

#[test]
fn the_embedded_public_key_matches_the_release_signing_key() {
    let dir = format!("{}/tests/fixtures/updates", env!("CARGO_MANIFEST_DIR"));
    let data = fs::read(format!("{dir}/real.bin")).unwrap();
    let sig = fs::read_to_string(format!("{dir}/real.bin.sig")).unwrap();
    assert!(verify_signature(UPDATER_PUBKEY, &sig, &data));
    let mut other = data.clone();
    other.push(b'x');
    assert!(!verify_signature(UPDATER_PUBKEY, &sig, &other));
}
