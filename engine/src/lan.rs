use crate::derive::{group_of_path, mods_dir, safe_rel};
use crate::discover::read_game_version;
use crate::error::{EngineError, Result};
use crate::fsutil::is_link;
use crate::instances::{active_instance_id, game_running};
use crate::manage::registered;
use crate::modhash::{compare_mods, scan_mods, ModFile};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub const DEFAULT_PORT: u16 = 47321;
const PORT_SPAN: u16 = 20;
const PROTOCOL: u32 = 2;
const MAX_FRAME: usize = 64 << 20;
const MAX_PENDING: usize = 8;
const IO_TIMEOUT: Duration = Duration::from_secs(30);
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(90);
const STAGING: &str = ".plumb-sync-tmp";
const BACKUPS: &str = "ModsBackup";
const TRUSTED_FILE: &str = "lan-trusted.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanAddress {
    pub ip: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanHostInfo {
    pub addresses: Vec<LanAddress>,
    pub listen: String,
    pub port: u16,
    pub name: String,
    pub files: u32,
    pub bytes: u64,
    pub game_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanPending {
    pub ip: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanHostStatus {
    pub clients: u32,
    pub served_bytes: u64,
    pub pending: Vec<LanPending>,
    pub trusted: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanItem {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub hash: String,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanChange {
    pub host: LanItem,
    pub local: LanItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanDiff {
    pub host_name: String,
    pub host_version: Option<String>,
    pub local_version: Option<String>,
    pub host_files: u32,
    pub local_files: u32,
    pub missing: Vec<LanItem>,
    pub different: Vec<LanChange>,
    pub extra: Vec<LanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanDownload {
    pub host_path: String,
    pub hash: String,
    pub size: u64,
    pub dest: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LanPlan {
    pub download: Vec<LanDownload>,
    pub remove: Vec<String>,
    pub backup: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanReport {
    pub added: u32,
    pub replaced: u32,
    pub removed: u32,
    pub bytes: u64,
    pub backup_dir: Option<String>,
}

struct Waiting {
    id: u64,
    ip: String,
    name: String,
    decision: Option<(bool, bool)>,
}

struct Shared {
    stop: AtomicBool,
    clients: AtomicU32,
    served: AtomicU64,
    next_id: AtomicU64,
    waiting: Mutex<Vec<Waiting>>,
    session: Mutex<HashSet<String>>,
    trusted: Mutex<Vec<String>>,
    trusted_file: PathBuf,
}

impl Shared {
    fn is_allowed(&self, ip: &str) -> bool {
        self.session.lock().map(|s| s.contains(ip)).unwrap_or(false) || self.trusted.lock().map(|t| t.iter().any(|x| x == ip)).unwrap_or(false)
    }

    fn save_trusted(&self) {
        if let Ok(t) = self.trusted.lock() {
            if let Ok(text) = serde_json::to_string(&*t) {
                let _ = fs::write(&self.trusted_file, text);
            }
        }
    }
}

pub struct LanHost {
    info: LanHostInfo,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl LanHost {
    pub fn info(&self) -> LanHostInfo {
        self.info.clone()
    }

    pub fn status(&self) -> LanHostStatus {
        let mut pending: Vec<LanPending> = Vec::new();
        if let Ok(list) = self.shared.waiting.lock() {
            for w in list.iter().filter(|w| w.decision.is_none()) {
                if !pending.iter().any(|p| p.ip == w.ip) {
                    pending.push(LanPending { ip: w.ip.clone(), name: w.name.clone() });
                }
            }
        }
        LanHostStatus {
            clients: self.shared.clients.load(Ordering::SeqCst),
            served_bytes: self.shared.served.load(Ordering::SeqCst),
            pending,
            trusted: self.shared.trusted.lock().map(|t| t.clone()).unwrap_or_default(),
        }
    }

    pub fn answer(&self, ip: &str, allow: bool, remember: bool) {
        if let Ok(mut list) = self.shared.waiting.lock() {
            for w in list.iter_mut().filter(|w| w.ip == ip && w.decision.is_none()) {
                w.decision = Some((allow, remember));
            }
        }
    }

    pub fn forget(&self, ip: &str) {
        if let Ok(mut t) = self.shared.trusted.lock() {
            t.retain(|x| x != ip);
        }
        if let Ok(mut s) = self.shared.session.lock() {
            s.remove(ip);
        }
        self.shared.save_trusted();
    }

    pub fn stop(mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for LanHost {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalNet {
    pub name: String,
    pub ip: Ipv4Addr,
    pub mask: Ipv4Addr,
}

pub fn ip_in_nets(ip: IpAddr, nets: &[LocalNet]) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || nets.iter().any(|n| {
                    let mask = u32::from(n.mask);
                    mask != 0 && (u32::from(v4) & mask) == (u32::from(n.ip) & mask)
                })
        }
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

fn valid_mask(mask: Ipv4Addr) -> bool {
    let m = u32::from(mask);
    m != 0 && (!m).wrapping_add(1) & !m == 0
}

pub fn parse_adapters(text: &str) -> Vec<LocalNet> {
    let mut out: Vec<LocalNet> = Vec::new();
    let mut name = String::new();
    let mut pending: Option<Ipv4Addr> = None;
    let flush = |pending: &mut Option<Ipv4Addr>, name: &str, out: &mut Vec<LocalNet>| {
        if let Some(ip) = pending.take() {
            out.push(LocalNet { name: name.to_string(), ip, mask: Ipv4Addr::new(255, 255, 255, 0) });
        }
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let indented = line.starts_with(' ') || line.starts_with('\t');
        if !indented && trimmed.ends_with(':') {
            flush(&mut pending, &name, &mut out);
            name = trimmed.trim_end_matches(':').trim().to_string();
            continue;
        }
        let Some((key, value)) = line.split_once(':') else { continue };
        let clean = value.trim().trim_end_matches(|c: char| !c.is_ascii_digit() && c != '.').trim();
        let Ok(ip) = clean.parse::<Ipv4Addr>() else { continue };
        if key.contains("IPv4") {
            flush(&mut pending, &name, &mut out);
            pending = Some(ip);
        } else if pending.is_some() && valid_mask(ip) {
            if let Some(addr) = pending.take() {
                out.push(LocalNet { name: name.clone(), ip: addr, mask: ip });
            }
        }
    }
    flush(&mut pending, &name, &mut out);
    out.retain(|n| !n.ip.is_unspecified() && !n.ip.is_loopback());
    out
}

pub fn local_nets() -> Vec<LocalNet> {
    let mut out: Vec<LocalNet> = Vec::new();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if let Ok(res) = std::process::Command::new("ipconfig").creation_flags(0x0800_0000).output() {
            out = parse_adapters(&String::from_utf8_lossy(&res.stdout));
        }
    }
    if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
        if sock.connect("8.8.8.8:80").is_ok() {
            if let Ok(SocketAddr::V4(addr)) = sock.local_addr() {
                let ip = *addr.ip();
                if !ip.is_unspecified() && !out.iter().any(|n| n.ip == ip) {
                    out.push(LocalNet { name: String::new(), ip, mask: Ipv4Addr::new(255, 255, 255, 0) });
                }
                if let Some(i) = out.iter().position(|n| n.ip == ip) {
                    let first = out.remove(i);
                    out.insert(0, first);
                }
            }
        }
    }
    out
}

pub fn local_addresses() -> Vec<LanAddress> {
    addresses_of(&local_nets())
}

fn addresses_of(nets: &[LocalNet]) -> Vec<LanAddress> {
    let mut out: Vec<LanAddress> = nets.iter().map(|n| LanAddress { ip: n.ip.to_string(), name: n.name.clone() }).collect();
    if out.is_empty() {
        out.push(LanAddress { ip: "127.0.0.1".to_string(), name: String::new() });
    }
    out
}

fn send(stream: &mut TcpStream, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(&bytes)?;
    Ok(())
}

fn recv(stream: &mut TcpStream) -> Result<Value> {
    let mut head = [0u8; 4];
    stream.read_exact(&mut head)?;
    let len = u32::from_be_bytes(head) as usize;
    if len > MAX_FRAME {
        return Err(EngineError::Invalid("lan frame".into()));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;
    Ok(serde_json::from_slice(&buf)?)
}

struct HostCtx {
    mods: PathBuf,
    files: HashMap<String, ModFile>,
    manifest: Value,
    nets: Vec<LocalNet>,
    shared: Arc<Shared>,
}

fn computer_name() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_default().chars().take(40).collect()
}

fn remove_waiting(ctx: &HostCtx, id: u64) {
    if let Ok(mut list) = ctx.shared.waiting.lock() {
        list.retain(|w| w.id != id);
    }
}

fn admit(stream: &mut TcpStream, ctx: &HostCtx, ip: &str, name: &str) -> bool {
    if ctx.shared.is_allowed(ip) {
        return true;
    }
    let id = ctx.shared.next_id.fetch_add(1, Ordering::SeqCst);
    {
        let Ok(mut list) = ctx.shared.waiting.lock() else { return false };
        if list.len() >= MAX_PENDING {
            let _ = send(stream, &json!({ "t": "deny", "why": "busy" }));
            return false;
        }
        list.push(Waiting { id, ip: ip.to_string(), name: name.to_string(), decision: None });
    }
    if send(stream, &json!({ "t": "wait" })).is_err() {
        remove_waiting(ctx, id);
        return false;
    }
    let started = Instant::now();
    let decision = loop {
        if ctx.shared.stop.load(Ordering::SeqCst) || started.elapsed() >= APPROVAL_TIMEOUT {
            break None;
        }
        let answered = ctx.shared.waiting.lock().ok().and_then(|l| l.iter().find(|w| w.id == id).and_then(|w| w.decision));
        if answered.is_some() {
            break answered;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    remove_waiting(ctx, id);
    match decision {
        Some((true, remember)) => {
            if let Ok(mut s) = ctx.shared.session.lock() {
                s.insert(ip.to_string());
            }
            if remember {
                if let Ok(mut t) = ctx.shared.trusted.lock() {
                    if !t.iter().any(|x| x == ip) {
                        t.push(ip.to_string());
                    }
                }
                ctx.shared.save_trusted();
            }
            true
        }
        Some((false, _)) => {
            let _ = send(stream, &json!({ "t": "deny", "why": "denied" }));
            false
        }
        None => {
            let _ = send(stream, &json!({ "t": "deny", "why": "expired" }));
            false
        }
    }
}

fn serve(mut stream: TcpStream, ctx: Arc<HostCtx>) {
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_nodelay(true);
    let Ok(peer) = stream.peer_addr() else { return };
    if !ip_in_nets(peer.ip(), &ctx.nets) {
        let _ = send(&mut stream, &json!({ "t": "deny", "why": "range" }));
        return;
    }
    if send(&mut stream, &json!({ "t": "hello", "v": PROTOCOL })).is_err() {
        return;
    }
    let Ok(join) = recv(&mut stream) else { return };
    if join.get("t").and_then(Value::as_str) != Some("join") {
        return;
    }
    let name: String = join.get("name").and_then(Value::as_str).unwrap_or("").chars().take(40).collect();
    let ip = peer.ip().to_string();
    if !admit(&mut stream, &ctx, &ip, &name) {
        return;
    }
    if send(&mut stream, &ctx.manifest).is_err() {
        return;
    }
    ctx.shared.clients.fetch_add(1, Ordering::SeqCst);
    loop {
        if ctx.shared.stop.load(Ordering::SeqCst) || !ctx.shared.is_allowed(&ip) {
            break;
        }
        let Ok(msg) = recv(&mut stream) else { break };
        if msg.get("t").and_then(Value::as_str) != Some("get") {
            break;
        }
        let key = msg.get("path").and_then(Value::as_str).unwrap_or("").to_lowercase();
        let Some(file) = ctx.files.get(&key) else {
            if send(&mut stream, &json!({ "t": "err" })).is_err() {
                break;
            }
            continue;
        };
        let Ok(rel) = safe_rel(&file.path) else { break };
        let full = ctx.mods.join(rel);
        let opened = if is_link(&full) { None } else { fs::File::open(&full).ok() };
        let Some(mut handle) = opened else {
            if send(&mut stream, &json!({ "t": "err" })).is_err() {
                break;
            }
            continue;
        };
        if handle.metadata().map(|m| m.len()).unwrap_or(u64::MAX) != file.size {
            if send(&mut stream, &json!({ "t": "err" })).is_err() {
                break;
            }
            continue;
        }
        if send(&mut stream, &json!({ "t": "file", "size": file.size })).is_err() {
            break;
        }
        let mut buf = vec![0u8; 256 * 1024];
        let mut left = file.size;
        let mut broken = false;
        while left > 0 {
            let want = buf.len().min(left as usize);
            match handle.read(&mut buf[..want]) {
                Ok(0) | Err(_) => {
                    broken = true;
                    break;
                }
                Ok(n) => {
                    if stream.write_all(&buf[..n]).is_err() {
                        broken = true;
                        break;
                    }
                    left -= n as u64;
                    ctx.shared.served.fetch_add(n as u64, Ordering::SeqCst);
                }
            }
        }
        if broken {
            break;
        }
    }
    ctx.shared.clients.fetch_sub(1, Ordering::SeqCst);
}

fn bind_listener() -> Result<TcpListener> {
    for offset in 0..PORT_SPAN {
        if let Ok(l) = TcpListener::bind(("0.0.0.0", DEFAULT_PORT + offset)) {
            return Ok(l);
        }
    }
    Ok(TcpListener::bind("0.0.0.0:0")?)
}

fn read_trusted(file: &Path) -> Vec<String> {
    fs::read_to_string(file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn start_host(
    game_root: &Path,
    code: &str,
    cache_file: Option<&Path>,
    progress: &(dyn Fn(u64, u64) + Sync),
) -> Result<LanHost> {
    let (dir, meta) = registered(game_root, code)?;
    let scan = scan_mods(&dir, cache_file, progress)?;
    let mods = mods_dir(&dir).unwrap_or_else(|| dir.join("Mods"));
    let version = read_game_version(&dir);
    let bytes: u64 = scan.files.iter().map(|f| f.size).sum();
    let manifest = json!({
        "t": "ok",
        "name": meta.name,
        "gameVersion": version,
        "files": scan.files,
    });
    let files: HashMap<String, ModFile> = scan.files.iter().map(|f| (f.path.to_lowercase(), f.clone())).collect();
    let listener = bind_listener()?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    let trusted_file = dir.join(TRUSTED_FILE);
    let shared = Arc::new(Shared {
        stop: AtomicBool::new(false),
        clients: AtomicU32::new(0),
        served: AtomicU64::new(0),
        next_id: AtomicU64::new(1),
        waiting: Mutex::new(Vec::new()),
        session: Mutex::new(HashSet::new()),
        trusted: Mutex::new(read_trusted(&trusted_file)),
        trusted_file,
    });
    let nets = local_nets();
    let addresses = addresses_of(&nets);
    let ctx = Arc::new(HostCtx { mods, files, manifest, nets, shared: shared.clone() });
    let loop_shared = shared.clone();
    let thread = std::thread::spawn(move || {
        while !loop_shared.stop.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    let ctx = ctx.clone();
                    std::thread::spawn(move || serve(stream, ctx));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(60)),
                Err(_) => std::thread::sleep(Duration::from_millis(200)),
            }
        }
    });
    Ok(LanHost {
        info: LanHostInfo {
            addresses,
            listen: format!("0.0.0.0:{port}"),
            port,
            name: meta.name,
            files: scan.files.len() as u32,
            bytes,
            game_version: version,
        },
        shared,
        thread: Some(thread),
    })
}

pub struct Remote {
    stream: TcpStream,
    pub name: String,
    pub game_version: Option<String>,
    pub files: Vec<ModFile>,
}

pub fn parse_target(addr: &str) -> Result<(String, Option<u16>)> {
    let mut text = addr.trim();
    for prefix in ["tcp://", "http://", "https://"] {
        if let Some(rest) = text.strip_prefix(prefix) {
            text = rest;
        }
    }
    let text = text.trim_end_matches('/');
    let parts: Vec<&str> = text.split(|c: char| c == ':' || c == ',' || c == ';' || c.is_whitespace()).filter(|p| !p.is_empty()).collect();
    match parts.as_slice() {
        [host] => Ok((host.to_string(), None)),
        [host, port] => {
            let port: u16 = port.parse().map_err(|_| EngineError::Invalid("lan address".into()))?;
            if port == 0 {
                return Err(EngineError::Invalid("lan address".into()));
            }
            Ok((host.to_string(), Some(port)))
        }
        _ => Err(EngineError::Invalid("lan address".into())),
    }
}

fn resolve(host: &str, nets: &[LocalNet]) -> Result<Ipv4Addr> {
    let candidates: Vec<Ipv4Addr> = (host, 0u16)
        .to_socket_addrs()
        .map_err(|_| EngineError::Invalid("lan address".into()))?
        .filter_map(|a| match a.ip() {
            IpAddr::V4(v4) => Some(v4),
            IpAddr::V6(_) => None,
        })
        .collect();
    if candidates.is_empty() {
        return Err(EngineError::Invalid("lan address".into()));
    }
    candidates.into_iter().find(|ip| ip_in_nets(IpAddr::V4(*ip), nets)).ok_or_else(|| EngineError::Invalid("lan range".into()))
}

pub fn connect_error(e: &std::io::Error) -> EngineError {
    use std::io::ErrorKind;
    let tag = match e.kind() {
        ErrorKind::ConnectionRefused => "lan refused",
        ErrorKind::TimedOut | ErrorKind::WouldBlock => "lan timeout",
        _ => match e.raw_os_error() {
            Some(10061) => "lan refused",
            Some(10060) => "lan timeout",
            Some(10051) | Some(10065) | Some(10050) => "lan unreachable",
            _ => "lan timeout",
        },
    };
    EngineError::Invalid(tag.into())
}

fn open_stream(ip: Ipv4Addr, port: Option<u16>) -> Result<TcpStream> {
    let ports: Vec<u16> = match port {
        Some(p) => vec![p],
        None => (0..PORT_SPAN).map(|i| DEFAULT_PORT + i).collect(),
    };
    let mut last = EngineError::Invalid("lan refused".into());
    for p in ports {
        match TcpStream::connect_timeout(&SocketAddr::new(IpAddr::V4(ip), p), Duration::from_secs(4)) {
            Ok(s) => return Ok(s),
            Err(e) => {
                let err = connect_error(&e);
                if !matches!(&err, EngineError::Invalid(m) if m == "lan refused") {
                    return Err(err);
                }
                last = err;
            }
        }
    }
    Err(last)
}

fn deny_error(msg: &Value) -> EngineError {
    let tag = match msg.get("why").and_then(Value::as_str).unwrap_or("") {
        "range" => "lan peer",
        "denied" => "lan denied",
        "expired" => "lan expired",
        "busy" => "lan busy",
        _ => "lan denied",
    };
    EngineError::Invalid(tag.into())
}

pub fn connect(addr: &str, on_wait: &dyn Fn()) -> Result<Remote> {
    let nets = local_nets();
    let (host, port) = parse_target(addr)?;
    let ip = resolve(&host, &nets)?;
    let mut stream = open_stream(ip, port)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    let _ = stream.set_nodelay(true);
    let hello = recv(&mut stream).map_err(|e| match e {
        EngineError::Io(io) if matches!(io.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock) => EngineError::Invalid("lan timeout".into()),
        _ => EngineError::Invalid("lan protocol".into()),
    })?;
    match hello.get("t").and_then(Value::as_str) {
        Some("deny") => return Err(deny_error(&hello)),
        Some("hello") => {}
        _ => return Err(EngineError::Invalid("lan protocol".into())),
    }
    if hello.get("v").and_then(Value::as_u64) != Some(PROTOCOL as u64) {
        return Err(EngineError::Invalid("lan version".into()));
    }
    send(&mut stream, &json!({ "t": "join", "name": computer_name() }))?;
    let mut reply = recv(&mut stream).map_err(|_| EngineError::Invalid("lan protocol".into()))?;
    if reply.get("t").and_then(Value::as_str) == Some("wait") {
        on_wait();
        stream.set_read_timeout(Some(APPROVAL_TIMEOUT + Duration::from_secs(15)))?;
        reply = recv(&mut stream).map_err(|_| EngineError::Invalid("lan expired".into()))?;
        stream.set_read_timeout(Some(IO_TIMEOUT))?;
    }
    match reply.get("t").and_then(Value::as_str) {
        Some("ok") => {}
        Some("deny") => return Err(deny_error(&reply)),
        _ => return Err(EngineError::Invalid("lan protocol".into())),
    }
    let name = reply.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    let game_version = reply.get("gameVersion").and_then(Value::as_str).map(str::to_string);
    let files: Vec<ModFile> = serde_json::from_value(reply.get("files").cloned().unwrap_or(Value::Null)).map_err(|_| EngineError::Invalid("lan frame".into()))?;
    for f in &files {
        safe_rel(&f.path)?;
    }
    Ok(Remote { stream, name, game_version, files })
}

impl Remote {
    fn fetch(&mut self, host_path: &str, size: u64, hash: &str, to: &Path, on_bytes: &mut dyn FnMut(u64)) -> Result<()> {
        send(&mut self.stream, &json!({ "t": "get", "path": host_path }))?;
        let head = recv(&mut self.stream)?;
        if head.get("t").and_then(Value::as_str) != Some("file") || head.get("size").and_then(Value::as_u64) != Some(size) {
            return Err(EngineError::Invalid("lan hash".into()));
        }
        let mut out = fs::File::create(to)?;
        let mut hasher = blake3::Hasher::new();
        let mut buf = vec![0u8; 256 * 1024];
        let mut left = size;
        while left > 0 {
            let want = buf.len().min(left as usize);
            let n = self.stream.read(&mut buf[..want])?;
            if n == 0 {
                return Err(EngineError::Invalid("lan timeout".into()));
            }
            hasher.update(&buf[..n]);
            out.write_all(&buf[..n])?;
            left -= n as u64;
            on_bytes(n as u64);
        }
        out.flush()?;
        if hasher.finalize().to_hex().to_string() != hash {
            return Err(EngineError::Invalid("lan hash".into()));
        }
        Ok(())
    }

    pub fn close(mut self) {
        let _ = send(&mut self.stream, &json!({ "t": "bye" }));
    }
}

fn item_of(f: &ModFile) -> LanItem {
    LanItem { path: f.path.clone(), name: f.name.clone(), size: f.size, hash: f.hash.clone(), group: group_of_path(&f.path).map(str::to_string) }
}

pub fn lan_diff(
    game_root: &Path,
    code: &str,
    addr: &str,
    cache_file: Option<&Path>,
    progress: &(dyn Fn(u64, u64) + Sync),
    on_wait: &dyn Fn(),
) -> Result<LanDiff> {
    let (dir, _) = registered(game_root, code)?;
    let remote = connect(addr, on_wait)?;
    let host_name = remote.name.clone();
    let host_version = remote.game_version.clone();
    let host_files = remote.files.clone();
    remote.close();
    let local = scan_mods(&dir, cache_file, progress)?;
    let diff = compare_mods(&host_files, &local.files);
    Ok(LanDiff {
        host_name,
        host_version,
        local_version: read_game_version(&dir),
        host_files: host_files.len() as u32,
        local_files: local.files.len() as u32,
        missing: diff.missing.iter().map(item_of).collect(),
        different: diff.different.iter().map(|c| LanChange { host: item_of(&c.host), local: item_of(&c.local) }).collect(),
        extra: diff.extra.iter().map(item_of).collect(),
    })
}

fn is_mod_path(rel: &Path) -> bool {
    rel.extension().is_some_and(|e| e.eq_ignore_ascii_case("package") || e.eq_ignore_ascii_case("ts4script"))
}

fn no_links_in(mods: &Path, rel: &Path) -> bool {
    let mut cur = mods.to_path_buf();
    for part in rel.components() {
        cur.push(part);
        if is_link(&cur) {
            return false;
        }
    }
    true
}

fn move_aside(from: &Path, backup: &Path, rel: &Path) -> Result<()> {
    let mut to = backup.join(rel);
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut n = 1;
    while to.exists() {
        let name = to.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        to.set_file_name(format!("{name}.{n}"));
        n += 1;
    }
    if fs::rename(from, &to).is_err() {
        fs::copy(from, &to)?;
        fs::remove_file(from)?;
    }
    Ok(())
}

fn stamp() -> String {
    let now = time::OffsetDateTime::now_utc();
    let fmt = time::macros::format_description!("[year][month][day]-[hour][minute][second]");
    now.format(&fmt).unwrap_or_else(|_| "backup".into())
}

pub fn sync_from_host(
    game_root: &Path,
    code: &str,
    addr: &str,
    plan: &LanPlan,
    progress: &mut dyn FnMut(&str, u64, u64),
    on_wait: &dyn Fn(),
) -> Result<LanReport> {
    let (dir, _) = registered(game_root, code)?;
    if game_running() && active_instance_id(game_root).is_some_and(|a| a.eq_ignore_ascii_case(code)) {
        return Err(EngineError::GameRunning);
    }
    let mods = mods_dir(&dir).unwrap_or_else(|| dir.join("Mods"));
    let mut downloads: Vec<(LanDownload, PathBuf)> = Vec::new();
    for d in &plan.download {
        safe_rel(&d.host_path)?;
        let dest = safe_rel(&d.dest)?;
        if !is_mod_path(&dest) || !no_links_in(&mods, &dest) {
            return Err(EngineError::Invalid("derive path".into()));
        }
        downloads.push((d.clone(), dest));
    }
    let mut removals: Vec<PathBuf> = Vec::new();
    for r in &plan.remove {
        let rel = safe_rel(r)?;
        if !is_mod_path(&rel) || !no_links_in(&mods, &rel) {
            return Err(EngineError::Invalid("derive path".into()));
        }
        removals.push(rel);
    }

    let staging = dir.join(STAGING);
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)?;
    let total: u64 = downloads.iter().map(|(d, _)| d.size).sum();
    let downloaded = (|| -> Result<()> {
        let mut remote = connect(addr, on_wait)?;
        let mut done = 0u64;
        progress("download", 0, total);
        for (i, (d, _)) in downloads.iter().enumerate() {
            let part = staging.join(format!("{i}.part"));
            let base = done;
            let mut last = Instant::now() - Duration::from_secs(1);
            let mut got = 0u64;
            remote.fetch(&d.host_path, d.size, &d.hash, &part, &mut |n| {
                got += n;
                if last.elapsed() >= Duration::from_millis(80) {
                    last = Instant::now();
                    progress("download", base + got, total);
                }
            })?;
            done += d.size;
            progress("download", done, total);
        }
        remote.close();
        Ok(())
    })();
    if let Err(e) = downloaded {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }

    let backup_dir = plan.backup.then(|| dir.join(BACKUPS).join(stamp()));
    let mut report = LanReport { backup_dir: None, ..Default::default() };
    let steps = (downloads.len() + removals.len()) as u64;
    let mut step = 0u64;
    progress("apply", 0, steps);
    let applied = (|| -> Result<()> {
        for (i, (d, dest)) in downloads.iter().enumerate() {
            let target = mods.join(dest);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let existed = target.is_file();
            if existed {
                match &backup_dir {
                    Some(b) => {
                        move_aside(&target, b, dest)?;
                        report.backup_dir = Some(b.display().to_string());
                    }
                    None => fs::remove_file(&target)?,
                }
            }
            let part = staging.join(format!("{i}.part"));
            if fs::rename(&part, &target).is_err() {
                fs::copy(&part, &target)?;
                let _ = fs::remove_file(&part);
            }
            if existed {
                report.replaced += 1;
            } else {
                report.added += 1;
            }
            report.bytes += d.size;
            step += 1;
            progress("apply", step, steps);
        }
        for rel in &removals {
            let target = mods.join(rel);
            if target.is_file() {
                match &backup_dir {
                    Some(b) => {
                        move_aside(&target, b, rel)?;
                        report.backup_dir = Some(b.display().to_string());
                    }
                    None => fs::remove_file(&target)?,
                }
                report.removed += 1;
            }
            step += 1;
            progress("apply", step, steps);
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(&staging);
    applied?;
    Ok(report)
}
