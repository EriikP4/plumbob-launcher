#![cfg(windows)]

use s4im_engine::*;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn setup(root: &Path) {
    let host = root.join("Los Sims 4_host");
    write(&host.join("Mods/a.package"), "host-a");
    write(&host.join("Mods/Sub/b.package"), "host-b");
    write(&host.join("Mods/WickedWhims/ww.ts4script"), "host-ww");
    write(&host.join("Mods/same.package"), "same");
    write(&host.join("Mods/readme.txt"), "ignored");
    write(&host.join("saves/Slot_00000001.save"), "host-save");
    adopt_instance(root, "host", "Host", "", InstanceKind::Mp).unwrap();
    let client = root.join("Los Sims 4_client");
    write(&client.join("Mods/a.package"), "client-a");
    write(&client.join("Mods/old.package"), "old");
    write(&client.join("Mods/same.package"), "same");
    write(&client.join("saves/Slot_00000001.save"), "client-save");
    adopt_instance(root, "client", "Client", "", InstanceKind::Mp).unwrap();
}

fn tree(dir: &Path) -> Vec<(String, u64)> {
    fn walk(dir: &Path, out: &mut Vec<(String, u64)>) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push((p.to_string_lossy().into_owned(), fs::metadata(&p).unwrap().len()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

fn plan_from(diff: &LanDiff, backup: bool, remove_extra: bool) -> LanPlan {
    let mut plan = LanPlan { backup, ..Default::default() };
    for m in &diff.missing {
        plan.download.push(LanDownload { host_path: m.path.clone(), hash: m.hash.clone(), size: m.size, dest: m.path.clone() });
    }
    for c in &diff.different {
        plan.download.push(LanDownload { host_path: c.host.path.clone(), hash: c.host.hash.clone(), size: c.host.size, dest: c.local.path.clone() });
    }
    if remove_extra {
        plan.remove = diff.extra.iter().map(|e| e.path.clone()).collect();
    }
    plan
}

fn open_host(root: &Path) -> (Arc<LanHost>, String) {
    let host = start_host(root, "host", None, &|_, _| {}).unwrap();
    let addr = format!("127.0.0.1:{}", host.info().port);
    (Arc::new(host), addr)
}

struct Approver {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Approver {
    fn start(host: Arc<LanHost>, allow: bool, remember: bool) -> Approver {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = std::thread::spawn(move || {
            while !flag.load(Ordering::SeqCst) {
                for p in host.status().pending {
                    host.answer(&p.ip, allow, remember);
                }
                std::thread::sleep(Duration::from_millis(30));
            }
        });
        Approver { stop, thread: Some(thread) }
    }

    fn finish(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            t.join().unwrap();
        }
    }
}

fn err_of<T>(r: Result<T>) -> String {
    match r {
        Ok(_) => String::new(),
        Err(e) => e.to_string(),
    }
}

fn diff_of(root: &Path, addr: &str) -> Result<LanDiff> {
    lan_diff(root, "client", addr, None, &|_, _| {}, &|| {})
}

fn wait_pending(host: &LanHost) -> Vec<LanPending> {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(5) {
        let p = host.status().pending;
        if !p.is_empty() {
            return p;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    Vec::new()
}

fn stop_host(host: Arc<LanHost>) {
    Arc::try_unwrap(host).ok().unwrap().stop();
}

#[test]
fn diff_reports_missing_different_and_extra_with_groups() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    setup(&root);
    let (host, addr) = open_host(&root);
    let approver = Approver::start(host.clone(), true, false);
    let diff = diff_of(&root, &addr).unwrap();
    let missing: Vec<&str> = diff.missing.iter().map(|m| m.path.as_str()).collect();
    assert_eq!(missing, vec!["Sub/b.package", "WickedWhims/ww.ts4script"]);
    assert_eq!(diff.missing[1].group.as_deref(), Some("wickedwhims"));
    assert_eq!(diff.different.len(), 1);
    assert_eq!(diff.different[0].host.path, "a.package");
    assert_eq!(diff.extra.iter().map(|e| e.path.as_str()).collect::<Vec<_>>(), vec!["old.package"]);
    assert_eq!(diff.host_name, "Host");
    approver.finish();
    stop_host(host);
}

#[test]
fn an_approved_session_syncs_with_backup_without_asking_again_and_never_touches_the_host() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    setup(&root);
    let host_before = tree(&root.join("Los Sims 4_host"));
    let (host, addr) = open_host(&root);
    let approver = Approver::start(host.clone(), true, false);
    let diff = diff_of(&root, &addr).unwrap();
    approver.finish();
    let plan = plan_from(&diff, true, true);
    let mut phases = Vec::new();
    let report = sync_from_host(&root, "client", &addr, &plan, &mut |p, d, t| phases.push((p.to_string(), d, t)), &|| panic!("asked again")).unwrap();
    let mods = root.join("Los Sims 4_client/Mods");
    assert_eq!(fs::read_to_string(mods.join("a.package")).unwrap(), "host-a");
    assert_eq!(fs::read_to_string(mods.join("Sub/b.package")).unwrap(), "host-b");
    assert_eq!(fs::read_to_string(mods.join("WickedWhims/ww.ts4script")).unwrap(), "host-ww");
    assert!(!mods.join("old.package").exists());
    assert!(!mods.join("readme.txt").exists());
    assert_eq!(report.added, 2);
    assert_eq!(report.replaced, 1);
    assert_eq!(report.removed, 1);
    let backup = std::path::PathBuf::from(report.backup_dir.unwrap());
    assert_eq!(fs::read_to_string(backup.join("a.package")).unwrap(), "client-a");
    assert_eq!(fs::read_to_string(backup.join("old.package")).unwrap(), "old");
    assert!(!root.join("Los Sims 4_client/.plumb-sync-tmp").exists());
    assert_eq!(fs::read_to_string(root.join("Los Sims 4_client/saves/Slot_00000001.save")).unwrap(), "client-save");
    assert_eq!(phases.last().map(|p| p.0.as_str()), Some("apply"));
    assert_eq!(tree(&root.join("Los Sims 4_host")), host_before);
    stop_host(host);
}

#[test]
fn sync_without_backup_replaces_directly_and_keeps_extras_unless_marked() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    setup(&root);
    let (host, addr) = open_host(&root);
    let approver = Approver::start(host.clone(), true, false);
    let diff = diff_of(&root, &addr).unwrap();
    let plan = plan_from(&diff, false, false);
    let report = sync_from_host(&root, "client", &addr, &plan, &mut |_, _, _| {}, &|| {}).unwrap();
    approver.finish();
    assert!(report.backup_dir.is_none());
    assert!(!root.join("Los Sims 4_client/ModsBackup").exists());
    assert!(root.join("Los Sims 4_client/Mods/old.package").is_file());
    assert_eq!(fs::read_to_string(root.join("Los Sims 4_client/Mods/a.package")).unwrap(), "host-a");
    stop_host(host);
}

#[test]
fn nothing_is_served_until_the_host_approves_and_a_denial_is_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    setup(&root);
    let (host, addr) = open_host(&root);
    let waited = Arc::new(AtomicBool::new(false));
    let flag = waited.clone();
    let client_root = root.clone();
    let thread = std::thread::spawn(move || lan_diff(&client_root, "client", &addr, None, &|_, _| {}, &|| flag.store(true, Ordering::SeqCst)));
    let pending = wait_pending(&host);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].ip, "127.0.0.1");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(host.status().served_bytes, 0);
    assert_eq!(host.status().clients, 0);
    assert!(waited.load(Ordering::SeqCst));
    host.answer("127.0.0.1", false, true);
    let result = thread.join().unwrap();
    assert!(err_of(result).contains("lan denied"));
    assert!(host.status().trusted.is_empty());
    assert!(!root.join("Los Sims 4_host/lan-trusted.json").exists());
    stop_host(host);
}

#[test]
fn remembered_machines_connect_without_asking_until_they_are_forgotten() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    setup(&root);
    let (host, addr) = open_host(&root);
    let approver = Approver::start(host.clone(), true, true);
    diff_of(&root, &addr).unwrap();
    approver.finish();
    assert_eq!(host.status().trusted, vec!["127.0.0.1".to_string()]);
    assert!(root.join("Los Sims 4_host/lan-trusted.json").is_file());
    stop_host(host);

    let (host, addr) = open_host(&root);
    assert_eq!(host.status().trusted, vec!["127.0.0.1".to_string()]);
    lan_diff(&root, "client", &addr, None, &|_, _| {}, &|| panic!("asked although remembered")).unwrap();

    host.forget("127.0.0.1");
    assert!(host.status().trusted.is_empty());
    let client_root = root.clone();
    let thread = std::thread::spawn(move || lan_diff(&client_root, "client", &addr, None, &|_, _| {}, &|| {}));
    assert_eq!(wait_pending(&host).len(), 1);
    host.answer("127.0.0.1", false, false);
    assert!(err_of(thread.join().unwrap()).contains("lan denied"));
    stop_host(host);
}

#[test]
fn bad_paths_and_changed_files_abort_without_touching_anything() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    setup(&root);
    let (host, addr) = open_host(&root);
    let approver = Approver::start(host.clone(), true, false);
    let client_before = tree(&root.join("Los Sims 4_client"));
    let run = |plan: &LanPlan| sync_from_host(&root, "client", &addr, plan, &mut |_, _, _| {}, &|| {});
    let bad = LanPlan { download: vec![LanDownload { host_path: "a.package".into(), hash: "00".into(), size: 6, dest: r"..\x.package".into() }], ..Default::default() };
    assert!(run(&bad).is_err());
    let bad = LanPlan { remove: vec![r"..\saves\Slot_00000001.save".into()], ..Default::default() };
    assert!(run(&bad).is_err());
    let wrong_hash = LanPlan { download: vec![LanDownload { host_path: "a.package".into(), hash: "ab".repeat(32), size: 6, dest: "a.package".into() }], ..Default::default() };
    assert!(run(&wrong_hash).is_err());
    let unknown = LanPlan { download: vec![LanDownload { host_path: "../saves/Slot_00000001.save".into(), hash: "ab".repeat(32), size: 9, dest: "x.package".into() }], ..Default::default() };
    assert!(run(&unknown).is_err());
    approver.finish();
    assert_eq!(tree(&root.join("Los Sims 4_client")), client_before);
    stop_host(host);
}

#[test]
fn addresses_are_parsed_tolerantly_and_ports_are_optional() {
    assert_eq!(lan_parse_target("11.22.33.100:51234").unwrap(), ("11.22.33.100".to_string(), Some(51234)));
    assert_eq!(lan_parse_target("  11.22.33.100 : 51234  ").unwrap(), ("11.22.33.100".to_string(), Some(51234)));
    assert_eq!(lan_parse_target("11.22.33.100 51234").unwrap(), ("11.22.33.100".to_string(), Some(51234)));
    assert_eq!(lan_parse_target("tcp://11.22.33.100:51234/").unwrap(), ("11.22.33.100".to_string(), Some(51234)));
    assert_eq!(lan_parse_target("11.22.33.100,51234").unwrap(), ("11.22.33.100".to_string(), Some(51234)));
    assert_eq!(lan_parse_target("11.22.33.100").unwrap(), ("11.22.33.100".to_string(), None));
    assert!(lan_parse_target("11.22.33.100:0").is_err());
    assert!(lan_parse_target("11.22.33.100:99999").is_err());
    assert!(lan_parse_target("").is_err());
    assert!(lan_parse_target("a b c").is_err());
}

fn net(ip: &str, mask: &str) -> LocalNet {
    LocalNet { name: "test".into(), ip: ip.parse().unwrap(), mask: mask.parse().unwrap() }
}

#[test]
fn only_addresses_inside_a_local_subnet_or_loopback_are_allowed() {
    let nets = vec![net("192.168.1.20", "255.255.255.0"), net("11.22.33.100", "255.255.255.0")];
    assert!(ip_in_nets("127.0.0.1".parse().unwrap(), &nets));
    assert!(ip_in_nets("192.168.1.77".parse().unwrap(), &nets));
    assert!(ip_in_nets("11.22.33.9".parse().unwrap(), &nets));
    assert!(!ip_in_nets("11.22.34.9".parse().unwrap(), &nets));
    assert!(!ip_in_nets("8.8.8.8".parse().unwrap(), &nets));
    assert!(!ip_in_nets("192.168.2.1".parse().unwrap(), &nets));
    assert!(!ip_in_nets("8.8.8.8".parse().unwrap(), &[]));
    let wide = vec![net("10.8.0.5", "255.255.0.0")];
    assert!(ip_in_nets("10.8.200.1".parse().unwrap(), &wide));
    assert!(!ip_in_nets("10.9.0.1".parse().unwrap(), &wide));
}

#[test]
fn adapters_are_parsed_from_ipconfig_with_names_and_masks() {
    let text = "Configuración IP de Windows\r\n\r\nAdaptador de Ethernet Ethernet:\r\n\r\n   Sufijo DNS específico para la conexión. . :\r\n   Dirección IPv4. . . . . . . . . . . . . . : 192.168.1.20\r\n   Máscara de subred . . . . . . . . . . . . : 255.255.255.0\r\n   Puerta de enlace predeterminada . . . . . : 192.168.1.1\r\n\r\nAdaptador de Ethernet VPN Plumbob:\r\n\r\n   Dirección IPv4. . . . . . . . . . . . . . : 11.22.33.100(Preferido)\r\n   Máscara de subred . . . . . . . . . . . . : 255.255.0.0\r\n\r\nAdaptador de LAN inalámbrica Wi-Fi:\r\n\r\n   Estado de los medios. . . . . . . . . . . : medios desconectados\r\n";
    let nets = parse_adapters(text);
    assert_eq!(nets.len(), 2);
    assert_eq!(nets[0].ip.to_string(), "192.168.1.20");
    assert_eq!(nets[0].mask.to_string(), "255.255.255.0");
    assert!(nets[0].name.contains("Ethernet"));
    assert_eq!(nets[1].ip.to_string(), "11.22.33.100");
    assert_eq!(nets[1].mask.to_string(), "255.255.0.0");
    assert!(nets[1].name.contains("VPN"));
    assert!(ip_in_nets("11.22.200.9".parse().unwrap(), &nets));
}

#[test]
fn connection_failures_are_told_apart() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    setup(&root);
    let (host, addr) = open_host(&root);
    assert!(err_of(lan_connect("not an address", &|| {})).contains("lan address"));
    assert!(err_of(lan_connect("8.8.8.8:80", &|| {})).contains("lan range"));
    assert!(err_of(lan_connect("127.0.0.1:1", &|| {})).contains("lan refused"));
    let port = addr.split(':').nth(1).unwrap().to_string();
    let approver = Approver::start(host.clone(), true, false);
    assert!(lan_connect(&format!("127.0.0.1 {port}"), &|| {}).is_ok());
    assert!(lan_connect("127.0.0.1", &|| {}).is_ok());
    approver.finish();
    let timed_out = std::io::Error::from(std::io::ErrorKind::TimedOut);
    assert!(lan_connect_error(&timed_out).to_string().contains("lan timeout"));
    let unreachable = std::io::Error::from_raw_os_error(10065);
    assert!(lan_connect_error(&unreachable).to_string().contains("lan unreachable"));
    stop_host(host);
}

#[test]
fn incompatible_versions_and_foreign_servers_are_reported() {
    use std::io::Write;
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let thread = std::thread::spawn(move || {
        for round in 0..2 {
            let (mut s, _) = listener.accept().unwrap();
            if round == 0 {
                let body = br#"{"t":"hello","v":99}"#;
                s.write_all(&(body.len() as u32).to_be_bytes()).unwrap();
                s.write_all(body).unwrap();
            } else {
                s.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").unwrap();
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    });
    assert!(err_of(lan_connect(&format!("127.0.0.1:{port}"), &|| {})).contains("lan version"));
    assert!(err_of(lan_connect(&format!("127.0.0.1:{port}"), &|| {})).contains("lan protocol"));
    thread.join().unwrap();
}

#[test]
fn the_host_listens_on_a_fixed_port_and_every_local_address() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("EA");
    setup(&root);
    let (host, _) = open_host(&root);
    let info = host.info();
    assert_eq!(info.listen, format!("0.0.0.0:{}", info.port));
    assert!(info.port >= LAN_PORT && info.port < LAN_PORT + 20);
    assert!(!info.addresses.is_empty());
    let approver = Approver::start(host.clone(), true, false);
    let mut targets: Vec<String> = vec!["127.0.0.1".to_string()];
    targets.extend(info.addresses.iter().map(|a| a.ip.clone()));
    targets.dedup();
    for ip in targets {
        let remote = lan_connect(&format!("{ip}:{}", info.port), &|| {});
        assert!(remote.is_ok(), "{ip}: {}", err_of(remote.map(|_| ())));
    }
    approver.finish();
    stop_host(host);
}
