use serde::Deserialize;
use serde_json::{json, Value};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CLIENT_ID: &str = "1561091550904324267";
const POLL: Duration = Duration::from_secs(4);
const COUNT_TTL: Duration = Duration::from_secs(30);

static STATE: Mutex<Option<UiState>> = Mutex::new(None);

type Counts = Option<(Instant, String, u64, u64)>;

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UiState {
    pub section: String,
    pub mods: u64,
    pub bytes: u64,
    pub shots: u64,
    pub shot_bytes: u64,
    pub songs: u64,
}

#[derive(Clone, PartialEq)]
struct Activity {
    details: String,
    state: Option<String>,
    small: Option<&'static str>,
    start: Option<u64>,
}

pub fn set_state(state: UiState) {
    *STATE.lock().unwrap() = Some(state);
}

pub fn start() {
    std::thread::spawn(run);
}

fn run() {
    let mut conn: Option<File> = None;
    let mut sent: Option<Option<Activity>> = None;
    let mut s4mm_since: Option<u64> = None;
    let mut counts: Counts = None;
    let mut nonce = 0u64;
    loop {
        let settings = crate::settings().unwrap_or_default();
        let wanted = if settings.presence.enabled {
            let es = settings.language != "en";
            let hide = settings.presence.hide_numbers;
            if s4im_engine::game_running() {
                None
            } else if s4im_engine::process_running(s4im_engine::S4MM_PROCESS) {
                let start = *s4mm_since.get_or_insert_with(now);
                Some(s4mm_activity(es, hide, start, &mut counts))
            } else {
                s4mm_since = None;
                counts = None;
                Some(ui_activity(es, hide))
            }
        } else {
            s4mm_since = None;
            None
        };
        if settings.presence.enabled || conn.is_some() {
            if conn.is_none() {
                conn = connect();
                sent = None;
            }
            if let Some(file) = conn.as_mut() {
                if sent.as_ref() != Some(&wanted) {
                    nonce += 1;
                    if update(file, wanted.as_ref(), nonce) {
                        sent = Some(wanted);
                    } else {
                        conn = None;
                    }
                }
            }
            if !settings.presence.enabled && sent == Some(None) {
                conn = None;
            }
        }
        std::thread::sleep(POLL);
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn pick<'a>(es: bool, a: &'a str, b: &'a str) -> &'a str {
    if es { a } else { b }
}

fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn size(es: bool, bytes: u64) -> String {
    let dec = |v: f64| {
        let s = format!("{v:.1}");
        if es { s.replace('.', ",") } else { s }
    };
    let b = bytes as f64;
    let gb = b / 1024f64.powi(3);
    let mb = b / 1024f64.powi(2);
    let kb = b / 1024.0;
    if gb >= 1.0 {
        format!("{} GB", dec(gb))
    } else if mb >= 1.0 {
        if mb < 10.0 { format!("{} MB", dec(mb)) } else { format!("{} MB", mb.round() as u64) }
    } else if kb >= 1.0 {
        format!("{} KB", kb.round() as u64)
    } else {
        format!("{bytes} B")
    }
}

fn ui_activity(es: bool, hide: bool) -> Activity {
    let ui = STATE.lock().unwrap().clone().unwrap_or_default();
    let numbered = |details: &str, state: String, small: &'static str| Activity {
        details: details.into(),
        state: (!hide).then_some(state),
        small: Some(small),
        start: None,
    };
    match ui.section.as_str() {
        "instance" => numbered(
            pick(es, "Gestionando una estancia...", "Managing an instance..."),
            format!("{} — {}", plural(ui.mods, "mod", "mods"), size(es, ui.bytes)),
            "instance",
        ),
        "settings" => Activity {
            details: pick(es, "Cambiando ajustes...", "Changing settings...").into(),
            state: crate::game_version().ok().flatten().map(|v| format!("{} {v}", pick(es, "Versión del juego:", "Game version:"))),
            small: Some("settings"),
            start: None,
        },
        "screenshots" => {
            let count = if es { plural(ui.shots, "captura", "capturas") } else { plural(ui.shots, "screenshot", "screenshots") };
            numbered(
                pick(es, "Revisando capturas de pantalla...", "Browsing screenshots..."),
                format!("{count} — {}{}", size(es, ui.shot_bytes), pick(es, " total de todas las capturas", " total across all screenshots")),
                "screenshots",
            )
        }
        "music" => numbered(
            pick(es, "Gestionando música...", "Managing music..."),
            if es { plural(ui.songs, "canción", "canciones") } else { plural(ui.songs, "song", "songs") },
            "music",
        ),
        _ => Activity {
            details: pick(es, "En el menú principal", "In the main menu").into(),
            state: None,
            small: None,
            start: None,
        },
    }
}

fn s4mm_activity(es: bool, hide: bool, start: u64, counts: &mut Counts) -> Activity {
    let state = if hide {
        None
    } else {
        crate::config().ok().and_then(|cfg| {
            let id = s4im_engine::active_instance_id(&cfg.game_root)?;
            let stale = counts.as_ref().is_none_or(|(at, cached, _, _)| cached != &id || at.elapsed() > COUNT_TTL);
            if stale {
                let (packages, scripts) = count_mods(&s4im_engine::instance_dir(&cfg.game_root, &id).join("Mods"));
                *counts = Some((Instant::now(), id, packages, scripts));
            }
            let (_, _, packages, scripts) = counts.clone()?;
            Some(format!("{packages} CC/Mods — {}", plural(scripts, "Script", "Scripts")))
        })
    };
    Activity {
        details: pick(es, "Gestionando mods en S4MM...", "Managing mods in S4MM...").into(),
        state,
        small: Some("s4mm"),
        start: Some(start),
    }
}

fn count_mods(dir: &Path) -> (u64, u64) {
    let (mut packages, mut scripts) = (0, 0);
    let Ok(read) = std::fs::read_dir(dir) else { return (0, 0) };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let (p, s) = count_mods(&path);
            packages += p;
            scripts += s;
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("package") {
                packages += 1;
            } else if ext.eq_ignore_ascii_case("ts4script") {
                scripts += 1;
            }
        }
    }
    (packages, scripts)
}

fn frame(file: &mut File, op: u32, body: &Value) -> bool {
    let data = body.to_string().into_bytes();
    let mut buf = Vec::with_capacity(8 + data.len());
    buf.extend_from_slice(&op.to_le_bytes());
    buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
    buf.extend_from_slice(&data);
    file.write_all(&buf).and_then(|_| file.flush()).is_ok()
}

fn read_frame(file: &mut File) -> bool {
    let mut head = [0u8; 8];
    if file.read_exact(&mut head).is_err() {
        return false;
    }
    let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
    if len > 1 << 20 {
        return false;
    }
    let mut body = vec![0u8; len];
    file.read_exact(&mut body).is_ok()
}

fn connect() -> Option<File> {
    for n in 0..10 {
        let Ok(mut file) = OpenOptions::new().read(true).write(true).open(format!(r"\\.\pipe\discord-ipc-{n}")) else { continue };
        if frame(&mut file, 0, &json!({ "v": 1, "client_id": CLIENT_ID })) && read_frame(&mut file) {
            return Some(file);
        }
    }
    None
}

fn update(file: &mut File, activity: Option<&Activity>, nonce: u64) -> bool {
    let value = activity.map(|a| {
        let mut assets = json!({ "large_image": "main", "large_text": format!("Plumbob Launcher v{}", env!("CARGO_PKG_VERSION")) });
        if let Some(small) = a.small {
            assets["small_image"] = json!(small);
        }
        let mut v = json!({ "details": a.details, "assets": assets });
        if let Some(state) = &a.state {
            v["state"] = json!(state);
        }
        if let Some(start) = a.start {
            v["timestamps"] = json!({ "start": start });
        }
        v
    });
    let body = json!({
        "cmd": "SET_ACTIVITY",
        "args": { "pid": std::process::id(), "activity": value },
        "nonce": nonce.to_string(),
    });
    frame(file, 1, &body) && read_frame(file)
}
