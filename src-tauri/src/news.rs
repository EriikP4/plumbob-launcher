use crate::netfetch::{fetch_conditional, Fetched};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const TTL: Duration = Duration::from_secs(600);
const TIMEOUT: Duration = Duration::from_secs(6);

struct Cached {
    at: Instant,
    etag: Option<String>,
    body: String,
}

static CACHE: Mutex<Option<Cached>> = Mutex::new(None);

fn disk_file() -> Option<PathBuf> {
    crate::settings_file().ok().and_then(|f| f.parent().map(|p| p.join("announcements-cache.json")))
}

fn read_disk() -> Option<String> {
    let text = std::fs::read_to_string(disk_file()?).ok()?;
    (text.len() <= s4im_engine::ANNOUNCEMENTS_MAX_BYTES).then_some(text)
}

fn write_disk(body: &str) {
    if let Some(file) = disk_file() {
        let _ = std::fs::write(file, body);
    }
}

fn raw_body(force: bool) -> Option<String> {
    let (etag, fresh_body) = match CACHE.lock() {
        Ok(cache) => match cache.as_ref() {
            Some(c) if c.at.elapsed() < TTL && !force => return Some(c.body.clone()),
            Some(c) => (c.etag.clone(), Some(c.body.clone())),
            None => (None, None),
        },
        Err(_) => (None, None),
    };
    let known = fresh_body.or_else(read_disk);
    let sent_etag = if known.is_some() { etag.as_deref() } else { None };
    let result = fetch_conditional(s4im_engine::ANNOUNCEMENTS_URL, s4im_engine::ANNOUNCEMENTS_MAX_BYTES as u64, sent_etag, TIMEOUT);
    let (body, new_etag) = match result {
        Ok(Fetched::Fresh(data, tag)) => match String::from_utf8(data) {
            Ok(text) => {
                write_disk(&text);
                (text, tag)
            }
            Err(_) => return known,
        },
        Ok(Fetched::NotModified) => (known?, etag),
        Err(_) => return known,
    };
    if let Ok(mut cache) = CACHE.lock() {
        *cache = Some(Cached { at: Instant::now(), etag: new_etag, body: body.clone() });
    }
    Some(body)
}

#[tauri::command]
pub async fn fetch_announcements(lang: String, force: bool) -> Result<Vec<s4im_engine::Announcement>, String> {
    crate::blocking(move || {
        if std::env::var_os("PLUMBOB_FAKE_ANNOUNCEMENTS").is_some() {
            return Ok(s4im_engine::sample_announcements(&lang));
        }
        if !crate::settings()?.announcements_enabled {
            return Ok(Vec::new());
        }
        let Some(body) = raw_body(force) else { return Ok(Vec::new()) };
        Ok(s4im_engine::parse_announcements(&body, &lang, env!("CARGO_PKG_VERSION"), s4im_engine::announcements_today()))
    })
    .await
}

static NOTES: Mutex<Vec<(String, String, Option<String>)>> = Mutex::new(Vec::new());

fn fetch_text(url: &str) -> Option<Vec<u8>> {
    match fetch_conditional(url, s4im_engine::MAX_CHANGELOG_BYTES as u64, None, Duration::from_secs(4)) {
        Ok(Fetched::Fresh(data, _)) => Some(data),
        _ => None,
    }
}

pub fn notes_with(base: &str, version: &str, lang: &str, fallback: &str) -> String {
    if let Ok(cache) = NOTES.lock() {
        if let Some((_, _, found)) = cache.iter().find(|(v, l, _)| v == version && l == lang) {
            return found.clone().unwrap_or_else(|| fallback.to_string());
        }
    }
    let found = s4im_engine::pick_changelog(base, version, lang, fetch_text);
    if let Ok(mut cache) = NOTES.lock() {
        cache.retain(|(v, l, _)| !(v == version && l == lang));
        cache.push((version.to_string(), lang.to_string(), found.clone()));
        let excess = cache.len().saturating_sub(8);
        cache.drain(..excess);
    }
    found.unwrap_or_else(|| fallback.to_string())
}

pub fn localized_notes(version: &str, lang: &str, fallback: &str) -> String {
    notes_with(s4im_engine::CHANGELOG_BASE, version, lang, fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn serve(routes: &'static [(&'static str, &'static str)]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut stream = stream;
                let mut buf = [0u8; 2048];
                let n = stream.read(&mut buf).unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = request.split_whitespace().nth(1).unwrap_or("").to_string();
                let found = routes.iter().find(|(p, _)| *p == path);
                let (status, body) = match found {
                    Some((_, b)) => ("200 OK", *b),
                    None => ("404 Not Found", ""),
                };
                let head = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(body.as_bytes());
            }
        });
        format!("http://127.0.0.1:{port}/changelog/")
    }

    #[test]
    fn the_language_file_is_used_when_it_exists() {
        let base = serve(&[("/changelog/9.1.0.es.md", "- Hola"), ("/changelog/9.1.0.en.md", "- Hello")]);
        assert_eq!(notes_with(&base, "9.1.0", "es", "release body"), "- Hola");
        assert_eq!(notes_with(&base, "9.1.0", "en", "release body"), "- Hello");
    }

    #[test]
    fn spanish_falls_back_to_english_and_then_to_the_release_body() {
        let base = serve(&[("/changelog/9.2.0.en.md", "- Hello")]);
        assert_eq!(notes_with(&base, "9.2.0", "es", "release body"), "- Hello");
        let empty = serve(&[]);
        assert_eq!(notes_with(&empty, "9.3.0", "es", "release body"), "release body");
    }
}
