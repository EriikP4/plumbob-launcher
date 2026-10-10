#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use s4im_engine::{
    ArchivedInstance, Config, Diagnostics, EngineError, ImportMode, InstanceKind, InstanceSummary, Platform, SaveInfo, SessionEnd,
    DeleteStats, GameFolderCheck, GameOptions, ModDiff, ModFile, ModScan, Role, Genre, Health, RootCheck, CacheItem, ClearReport, MergeReport, SpaceEntry, Settings, Shot, SnapshotInfo, SnapshotKind, Transfer, TrayDisable,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

static WATCHING: AtomicBool = AtomicBool::new(false);
static CANCEL_WATCH: AtomicBool = AtomicBool::new(false);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    needs_setup: bool,
    active_id: Option<String>,
    game_running: bool,
    game_folder_choose: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionEvent {
    code: String,
    started: bool,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ShotsStatus {
    indexed: bool,
    count: usize,
}

#[derive(Clone, Serialize)]
struct ShotsProgress {
    done: usize,
    total: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Environment {
    s4mm: Option<PathBuf>,
    steam: Option<PathBuf>,
    ea: Option<PathBuf>,
    epic: Option<PathBuf>,
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

fn settings_file() -> Result<PathBuf, String> {
    Config::detect()
        .map(|c| c.settings_file)
        .ok_or_else(|| "err.documents".to_string())
}

fn settings() -> Result<Settings, String> {
    Ok(s4im_engine::load_settings(&settings_file()?))
}

fn config() -> Result<Config, String> {
    let base = Config::detect().ok_or_else(|| "err.documents".to_string())?;
    let s = s4im_engine::load_settings(&base.settings_file);
    s4im_engine::set_auto_snapshots(s.auto_snapshots);
    let root = s.game_root.clone().unwrap_or_else(|| base.game_root.clone());
    let name = s4im_engine::resolve_game_folder(&root, s.game_folder.as_deref());
    s4im_engine::set_game_folder_name(name.as_deref().unwrap_or(s4im_engine::DEFAULT_GAME_FOLDER));
    Ok(base.apply(&s))
}

fn text(e: EngineError) -> String {
    match e {
        EngineError::GameRunning => "err.gameRunning".into(),
        EngineError::NotFound(p) => format!("err.notFound:{p}"),
        EngineError::Invalid(m) if m.starts_with("code already in use") => "err.codeInUse".into(),
        EngineError::Invalid(m) if m.starts_with("code") => "err.codeInvalid".into(),
        EngineError::Invalid(m) if m == "name" => "err.nameRequired".into(),
        EngineError::Invalid(m) if m.starts_with("not registered") => "err.notRegistered".into(),
        EngineError::Invalid(m) if m.starts_with("s4mp") => "err.s4mpMissing".into(),
        EngineError::Invalid(m) if m.starts_with("s4mm") => "err.s4mmMissing".into(),
        EngineError::Invalid(m) if m.starts_with("launch id") => "err.launchInvalid".into(),
        EngineError::Invalid(m) if m.starts_with("custom command") => "err.customMissing".into(),
        EngineError::Invalid(m) if m.starts_with("install dir") => "err.installMissing".into(),
        EngineError::Invalid(m) if m.starts_with("delete refused") => "err.deleteRefused".into(),
        EngineError::Invalid(m) if m.starts_with("tray is a link") => "err.trayIsLink".into(),
        EngineError::Invalid(m) if m.starts_with("tray not synced") => "err.trayNotSynced".into(),
        EngineError::Invalid(m) if m.starts_with("no tray backup") => "err.noTrayBackup".into(),
        EngineError::Invalid(m) if m.starts_with("source not allowed") => "err.sourceNotAllowed".into(),
        EngineError::Invalid(m) if m.starts_with("not a game folder") => "err.notGameFolder".into(),
        EngineError::Invalid(m) if m.starts_with("access denied: ") => format!("err.accessDenied:{}", &m["access denied: ".len()..]),
        EngineError::Invalid(m) if m.starts_with("archive active") => "err.archiveActive".into(),
        EngineError::Invalid(m) if m.starts_with("archive exists") => "err.archiveExists".into(),
        EngineError::Invalid(m) if m.starts_with("archive incomplete") => "err.archiveIncomplete".into(),
        EngineError::Invalid(m) if m.starts_with("archive corrupt") => "err.archiveCorrupt".into(),
        EngineError::Invalid(m) if m.starts_with("copy mismatch") => "err.copyMismatch".into(),
        EngineError::Invalid(m) if m.starts_with("move incomplete") => "err.moveIncomplete".into(),
        other => other.to_string(),
    }
}

#[tauri::command]
fn get_settings() -> Result<Settings, String> {
    settings()
}

#[tauri::command]
fn save_settings(settings: Settings) -> Result<(), String> {
    s4im_engine::save_settings(&settings_file()?, &settings).map_err(text)
}

#[tauri::command]
fn detect_environment() -> Environment {
    Environment {
        s4mm: s4im_engine::detect_s4mm(),
        steam: s4im_engine::detect_install_dir(Platform::Steam),
        ea: s4im_engine::detect_install_dir(Platform::Ea),
        epic: s4im_engine::detect_install_dir(Platform::Epic),
    }
}

#[tauri::command]
fn get_status() -> Result<Status, String> {
    let cfg = config()?;
    let s = settings()?;
    Ok(Status {
        game_folder_choose: s4im_engine::resolve_game_folder(&cfg.game_root, s.game_folder.as_deref()).is_none(),
        needs_setup: s4im_engine::needs_setup(&cfg.game_root),
        active_id: s4im_engine::active_instance_id(&cfg.game_root),
        game_running: s4im_engine::game_running(),
    })
}

#[tauri::command]
fn list_instances() -> Result<Vec<InstanceSummary>, String> {
    let cfg = config()?;
    let mut list = s4im_engine::list_instances(&cfg.game_root).map_err(text)?;
    s4im_engine::apply_order(&mut list, &s4im_engine::load_order(&cfg.order_file));
    Ok(list)
}

#[tauri::command]
fn save_order(ids: Vec<String>) -> Result<(), String> {
    let cfg = config()?;
    s4im_engine::save_order(&cfg.order_file, &ids).map_err(text)
}

#[tauri::command]
async fn list_saves(app: AppHandle, instance_id: String) -> Result<Vec<SaveInfo>, String> {
    blocking(move || {
        let cfg = config()?;
        allow_assets(&app, &cfg);
        Ok(s4im_engine::list_saves(
            &s4im_engine::instance_dir(&cfg.game_root, &instance_id),
            Some(&cfg.save_thumbs),
        ))
    })
    .await
}

#[tauri::command]
fn list_snapshots(instance_id: String) -> Result<Vec<SnapshotInfo>, String> {
    let cfg = config()?;
    s4im_engine::list_snapshots(&cfg.snapshots_root, &instance_id).map_err(text)
}

fn apply_tray_default(cfg: &Config, summary: InstanceSummary) -> Result<InstanceSummary, String> {
    let wanted = settings().map(|s| s.tray_sync_default).unwrap_or(false);
    if !wanted {
        return Ok(summary);
    }
    let _ = s4im_engine::enable_tray_sync(&cfg.game_root, &cfg.snapshots_root, &summary.id, true);
    Ok(s4im_engine::list_instances(&cfg.game_root)
        .map_err(text)?
        .into_iter()
        .find(|i| i.id == summary.id)
        .unwrap_or(summary))
}

#[tauri::command]
async fn register_main(name: String, description: String, code: String) -> Result<InstanceSummary, String> {
    blocking(move || {
        let cfg = config()?;
        let summary = s4im_engine::register_main(&cfg.game_root, &cfg.snapshots_root, &name, &description, &code)
            .map_err(text)?;
        apply_tray_default(&cfg, summary)
    })
    .await
}

#[tauri::command]
async fn enable_tray_sync(code: String) -> Result<MergeReport, String> {
    blocking(move || {
        let cfg = config()?;
        s4im_engine::enable_tray_sync(&cfg.game_root, &cfg.snapshots_root, &code, true).map_err(text)
    })
    .await
}

#[tauri::command]
async fn disable_tray_sync(code: String, mode: TrayDisable) -> Result<MergeReport, String> {
    blocking(move || {
        let cfg = config()?;
        s4im_engine::disable_tray_sync(&cfg.game_root, &code, mode, true).map_err(text)
    })
    .await
}

#[tauri::command]
async fn import_instance(
    path: String,
    mode: ImportMode,
    name: String,
    description: String,
    code: String,
    kind: InstanceKind,
    role: Option<Role>,
) -> Result<InstanceSummary, String> {
    blocking(move || {
        let cfg = config()?;
        let summary = s4im_engine::import_instance(
            &cfg.game_root,
            std::path::Path::new(&path),
            mode,
            &name,
            &description,
            &code,
            kind,
        )
        .map_err(text)?;
        let summary = if role.is_some() {
            s4im_engine::set_instance_role(&cfg.game_root, &summary.id, role).map_err(text)?
        } else {
            summary
        };
        apply_tray_default(&cfg, summary)
    })
    .await
}

#[tauri::command]
async fn create_instance(
    name: String,
    description: String,
    code: String,
    kind: InstanceKind,
    options: Option<GameOptions>,
    role: Option<Role>,
) -> Result<InstanceSummary, String> {
    blocking(move || {
        let cfg = config()?;
        let summary = s4im_engine::create_instance(&cfg.game_root, &name, &description, &code, kind).map_err(text)?;
        if let Some(opts) = options {
            s4im_engine::apply_game_options(&summary.path, &opts).map_err(text)?;
        }
        let summary = if role.is_some() {
            s4im_engine::set_instance_role(&cfg.game_root, &summary.id, role).map_err(text)?
        } else {
            summary
        };
        apply_tray_default(&cfg, summary)
    })
    .await
}

#[tauri::command]
async fn adopt_instance(
    code: String,
    name: String,
    description: String,
    kind: InstanceKind,
    role: Option<Role>,
) -> Result<InstanceSummary, String> {
    blocking(move || {
        let cfg = config()?;
        let summary = s4im_engine::adopt_instance(&cfg.game_root, &code, &name, &description, kind).map_err(text)?;
        let summary = if role.is_some() {
            s4im_engine::set_instance_role(&cfg.game_root, &summary.id, role).map_err(text)?
        } else {
            summary
        };
        apply_tray_default(&cfg, summary)
    })
    .await
}

#[tauri::command]
async fn create_snapshot(instance_id: String) -> Result<SnapshotInfo, String> {
    blocking(move || {
        let cfg = config()?;
        let dir = s4im_engine::instance_dir(&cfg.game_root, &instance_id);
        s4im_engine::create_snapshot(
            &dir,
            &cfg.snapshots_root,
            &instance_id,
            "snapshot.manual",
            SnapshotKind::Manual,
            false,
        )
        .map_err(text)
    })
    .await
}

#[tauri::command]
async fn restore_snapshot(instance_id: String, snapshot_id: String) -> Result<(), String> {
    blocking(move || {
        let cfg = config()?;
        let active = s4im_engine::active_instance_id(&cfg.game_root);
        if s4im_engine::game_running() && active.is_some_and(|a| a.eq_ignore_ascii_case(&instance_id)) {
            return Err(text(EngineError::GameRunning));
        }
        let dir = s4im_engine::instance_dir(&cfg.game_root, &instance_id);
        s4im_engine::restore_snapshot(&dir, &cfg.snapshots_root, &instance_id, &snapshot_id)
            .map(|_| ())
            .map_err(text)
    })
    .await
}

#[tauri::command]
async fn delete_snapshot(instance_id: String, snapshot_id: String) -> Result<(), String> {
    blocking(move || {
        let cfg = config()?;
        s4im_engine::delete_snapshot(&cfg.snapshots_root, &instance_id, &snapshot_id).map_err(text)
    })
    .await
}

#[tauri::command]
fn update_instance(code: String, name: String, description: String, role: Option<Role>) -> Result<InstanceSummary, String> {
    let cfg = config()?;
    s4im_engine::update_instance(&cfg.game_root, &code, &name, &description).map_err(text)?;
    s4im_engine::set_instance_role(&cfg.game_root, &code, role).map_err(text)
}

#[tauri::command]
fn s4mp_leftovers(code: String) -> Result<Vec<String>, String> {
    let cfg = config()?;
    Ok(s4im_engine::s4mp_leftovers(&cfg.game_root, &code))
}

#[tauri::command]
async fn convert_instance(code: String, kind: InstanceKind, remove_s4mp: bool) -> Result<InstanceSummary, String> {
    blocking(move || {
        let cfg = config()?;
        s4im_engine::convert_instance(&cfg.game_root, &code, kind, remove_s4mp).map_err(text)
    })
    .await
}

#[tauri::command]
async fn duplicate_instance(
    source: String,
    name: String,
    description: String,
    code: String,
) -> Result<InstanceSummary, String> {
    blocking(move || {
        let cfg = config()?;
        s4im_engine::duplicate_instance(&cfg.game_root, &source, &name, &description, &code).map_err(text)
    })
    .await
}

#[tauri::command]
async fn delete_instance(app: AppHandle, code: String) -> Result<DeleteStats, String> {
    blocking(move || {
        let cfg = config()?;
        let emitter = app.clone();
        let report = move |done: u64, total: u64| {
            let _ = emitter.emit("delete-progress", ShotsProgress { done: done as usize, total: total as usize });
        };
        s4im_engine::delete_instance(&cfg.game_root, &cfg.snapshots_root, &code, true, &report).map_err(text)
    })
    .await
}

#[tauri::command]
async fn archive_instance(app: AppHandle, code: String, clean_cache: bool, compress: bool) -> Result<ArchivedInstance, String> {
    blocking(move || {
        let cfg = config()?;
        guard_instance(&cfg, &code)?;
        let emitter = app.clone();
        s4im_engine::archive_instance(&cfg.game_root, &code, clean_cache, compress, &move |done, total| {
            let _ = emitter.emit("archive-progress", ShotsProgress { done: done as usize, total: total as usize });
        })
        .map_err(text)
    })
    .await
}

#[tauri::command]
async fn list_archived() -> Result<Vec<ArchivedInstance>, String> {
    blocking(move || {
        let cfg = config()?;
        Ok(s4im_engine::list_archived(&cfg.game_root))
    })
    .await
}

#[tauri::command]
async fn restore_archived(app: AppHandle, code: String) -> Result<InstanceSummary, String> {
    blocking(move || {
        let cfg = config()?;
        let emitter = app.clone();
        s4im_engine::restore_archived(&cfg.game_root, &code, &move |done, total| {
            let _ = emitter.emit("archive-progress", ShotsProgress { done: done as usize, total: total as usize });
        })
        .map_err(text)
    })
    .await
}

#[tauri::command]
async fn diagnose() -> Result<Diagnostics, String> {
    blocking(move || {
        let cfg = config()?;
        Ok(s4im_engine::diagnose(&cfg.game_root, &cfg.game_dir))
    })
    .await
}

#[tauri::command]
async fn instance_health(code: String) -> Result<Health, String> {
    blocking(move || {
        let cfg = config()?;
        Ok(s4im_engine::analyze_health(&s4im_engine::instance_dir(&cfg.game_root, &code)))
    })
    .await
}

#[tauri::command]
fn game_version() -> Result<Option<String>, String> {
    let s = settings()?;
    let root = s.install_dir.clone().or_else(|| s4im_engine::detect_install_dir(s.platform));
    Ok(root.and_then(|r| s4im_engine::read_install_version(&r)))
}

fn guard_instance(cfg: &Config, code: &str) -> Result<PathBuf, String> {
    if !s4im_engine::valid_code(code) {
        return Err("err.codeInvalid".into());
    }
    let active = s4im_engine::active_instance_id(&cfg.game_root);
    if s4im_engine::game_running() && active.is_some_and(|a| a.eq_ignore_ascii_case(code)) {
        return Err(text(EngineError::GameRunning));
    }
    Ok(s4im_engine::instance_dir(&cfg.game_root, code))
}

#[tauri::command]
async fn cache_items(code: String) -> Result<Vec<CacheItem>, String> {
    blocking(move || {
        let cfg = config()?;
        if !s4im_engine::valid_code(&code) {
            return Err("err.codeInvalid".into());
        }
        Ok(s4im_engine::cache_items(&s4im_engine::instance_dir(&cfg.game_root, &code)))
    })
    .await
}

#[tauri::command]
async fn clear_cache(app: AppHandle, code: String, ids: Vec<String>) -> Result<ClearReport, String> {
    blocking(move || {
        let cfg = config()?;
        let dir = guard_instance(&cfg, &code)?;
        let emitter = app.clone();
        s4im_engine::clear_cache(&dir, &ids, &move |done, total| {
            let _ = emitter.emit("cache-progress", ShotsProgress { done: done as usize, total: total as usize });
        })
        .map_err(text)
    })
    .await
}

#[tauri::command]
fn music_genres() -> Vec<&'static str> {
    s4im_engine::GENRES.to_vec()
}

#[tauri::command]
async fn list_music(app: AppHandle, code: String) -> Result<Vec<Genre>, String> {
    blocking(move || {
        let cfg = config()?;
        if !s4im_engine::valid_code(&code) {
            return Err("err.codeInvalid".into());
        }
        allow_assets(&app, &cfg);
        Ok(s4im_engine::list_music(&s4im_engine::instance_dir(&cfg.game_root, &code)))
    })
    .await
}

#[tauri::command]
async fn add_music(app: AppHandle, code: String, genre: String, files: Vec<String>) -> Result<u32, String> {
    blocking(move || {
        let cfg = config()?;
        if !s4im_engine::valid_code(&code) {
            return Err("err.codeInvalid".into());
        }
        let emitter = app.clone();
        let paths: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
        s4im_engine::add_music(&s4im_engine::instance_dir(&cfg.game_root, &code), &genre, &paths, &move |done, total| {
            let _ = emitter.emit("music-progress", ShotsProgress { done: done as usize, total: total as usize });
        })
        .map_err(text)
    })
    .await
}

#[tauri::command]
async fn delete_track(code: String, path: String) -> Result<(), String> {
    blocking(move || {
        let cfg = config()?;
        if !s4im_engine::valid_code(&code) {
            return Err("err.codeInvalid".into());
        }
        s4im_engine::delete_track(&s4im_engine::instance_dir(&cfg.game_root, &code), &PathBuf::from(path)).map_err(text)
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CodeCheck {
    valid: bool,
    equal: bool,
    host_count: Option<u32>,
}

#[tauri::command]
async fn scan_mods(app: AppHandle, code: String) -> Result<ModScan, String> {
    blocking(move || {
        let cfg = config()?;
        if !s4im_engine::valid_code(&code) {
            return Err("err.codeInvalid".into());
        }
        let cache = cfg.order_file.parent().map(|p| p.join("hashcache").join(format!("{code}.json")));
        s4im_engine::scan_mods(&s4im_engine::instance_dir(&cfg.game_root, &code), cache.as_deref(), &|done, total| {
            let _ = app.emit("hash-progress", ShotsProgress { done: done as usize, total: total as usize });
        })
        .map_err(text)
    })
    .await
}

#[tauri::command]
fn compare_mods(host: Vec<ModFile>, local: Vec<ModFile>) -> ModDiff {
    s4im_engine::compare_mods(&host, &local)
}

#[tauri::command]
fn check_code(local: String, host: String) -> CodeCheck {
    let parsed = s4im_engine::parse_code(&host);
    CodeCheck { valid: parsed.is_some(), equal: s4im_engine::codes_match(&local, &host), host_count: parsed.map(|(n, _)| n) }
}

#[tauri::command]
fn current_game_folder() -> Result<String, String> {
    Ok(config()?.game_dir.to_string_lossy().into_owned())
}

#[tauri::command]
fn check_game_folder(path: String) -> GameFolderCheck {
    s4im_engine::check_game_folder(&PathBuf::from(path))
}

#[tauri::command]
fn check_game_root(path: String) -> RootCheck {
    s4im_engine::check_game_root(&PathBuf::from(path))
}

#[tauri::command]
fn open_instance_file(code: String, rel: String) -> Result<(), String> {
    let cfg = config()?;
    let file = s4im_engine::resolve_instance_file(&s4im_engine::instance_dir(&cfg.game_root, &code), &rel).map_err(text)?;
    let native = file.to_string_lossy().replace('/', "\\");
    std::process::Command::new("explorer").arg(native).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn space_children(code: String, rel: String) -> Result<Vec<SpaceEntry>, String> {
    blocking(move || {
        let cfg = config()?;
        s4im_engine::space_children(&s4im_engine::instance_dir(&cfg.game_root, &code), &rel).map_err(text)
    })
    .await
}

#[tauri::command]
async fn space_top_files(code: String, limit: usize) -> Result<Vec<SpaceEntry>, String> {
    blocking(move || {
        let cfg = config()?;
        Ok(s4im_engine::space_top_files(&s4im_engine::instance_dir(&cfg.game_root, &code), limit.min(200)))
    })
    .await
}

fn notes_file(code: &str) -> Result<PathBuf, String> {
    if !s4im_engine::valid_code(code) {
        return Err("err.codeInvalid".into());
    }
    let cfg = config()?;
    let dir = cfg.order_file.parent().map(|p| p.join("notes")).ok_or("err.documents")?;
    Ok(dir.join(format!("{code}.txt")))
}

#[derive(Serialize)]
struct NotesData {
    json: String,
    legacy: String,
}

#[tauri::command]
fn get_notes(code: String) -> Result<NotesData, String> {
    let txt = notes_file(&code)?;
    Ok(NotesData {
        json: std::fs::read_to_string(txt.with_extension("json")).unwrap_or_default(),
        legacy: std::fs::read_to_string(&txt).unwrap_or_default(),
    })
}

#[tauri::command]
fn save_notes(code: String, json: String) -> Result<(), String> {
    let file = notes_file(&code)?.with_extension("json");
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &file).map_err(|e| e.to_string())
}

fn text_ext(path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    let ok = p
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|e| matches!(e.as_str(), "md" | "txt" | "json"));
    if ok {
        Ok(p)
    } else {
        Err("err.notesFile".into())
    }
}

#[tauri::command]
fn export_text(path: String, text: String) -> Result<(), String> {
    std::fs::write(text_ext(&path)?, text).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_import(path: String) -> Result<String, String> {
    let p = text_ext(&path)?;
    let len = std::fs::metadata(&p).map_err(|e| e.to_string())?.len();
    if len > 5 * 1024 * 1024 {
        return Err("err.notesFile".into());
    }
    std::fs::read_to_string(&p).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_folder(code: String, which: String) -> Result<(), String> {
    let cfg = config()?;
    let base = s4im_engine::instance_dir(&cfg.game_root, &code);
    let target = match which.as_str() {
        "mods" => s4im_engine::find_child(&base, "Mods").unwrap_or_else(|| base.clone()),
        "saves" => s4im_engine::find_child(&base, "saves").unwrap_or_else(|| base.clone()),
        _ => base.clone(),
    };
    if !target.is_dir() {
        return Err(text(EngineError::NotFound(target.display().to_string())));
    }
    std::process::Command::new("explorer").arg(&target).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

fn allow_assets(app: &AppHandle, cfg: &Config) {
    let scope = app.asset_protocol_scope();
    let _ = scope.allow_directory(&cfg.game_root, true);
    let _ = scope.allow_directory(&cfg.thumbs_dir, true);
    let _ = scope.allow_directory(&cfg.save_thumbs, true);
    let _ = scope.allow_directory(&cfg.imported_shots, true);
}

#[tauri::command]
fn shots_status() -> Result<ShotsStatus, String> {
    let cfg = config()?;
    let indexed = s4im_engine::index_exists(&cfg.shots_index);
    let count = if indexed { s4im_engine::list_shots(&cfg.shots_index, None).len() } else { 0 };
    Ok(ShotsStatus { indexed, count })
}

#[tauri::command]
async fn shots_index(app: AppHandle) -> Result<usize, String> {
    blocking(move || {
        let cfg = config()?;
        allow_assets(&app, &cfg);
        let emitter = app.clone();
        let report = move |done: usize, total: usize| {
            let _ = emitter.emit("shots-progress", ShotsProgress { done, total });
        };
        s4im_engine::update_index(&cfg.game_root, &cfg.imported_shots, &cfg.shots_index, &cfg.thumbs_dir, &report)
            .map_err(text)
    })
    .await
}

#[tauri::command]
fn list_shots(app: AppHandle, instance: Option<String>) -> Result<Vec<Shot>, String> {
    let cfg = config()?;
    allow_assets(&app, &cfg);
    Ok(s4im_engine::list_shots(&cfg.shots_index, instance.as_deref()))
}

#[tauri::command]
async fn import_shots(paths: Vec<String>, target: Option<String>) -> Result<Transfer, String> {
    blocking(move || {
        let cfg = config()?;
        let sources: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        s4im_engine::import_shots(&cfg.game_root, &cfg.imported_shots, &sources, target.as_deref()).map_err(text)
    })
    .await
}

#[tauri::command]
async fn move_shots(paths: Vec<String>, target: Option<String>) -> Result<Transfer, String> {
    blocking(move || {
        let cfg = config()?;
        let sources: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        s4im_engine::move_shots(&cfg.game_root, &cfg.imported_shots, &sources, target.as_deref()).map_err(text)
    })
    .await
}

#[tauri::command]
async fn delete_shots(paths: Vec<String>) -> Result<Transfer, String> {
    blocking(move || {
        let cfg = config()?;
        let sources: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        Ok(s4im_engine::delete_shots(&cfg.game_root, &cfg.imported_shots, &sources))
    })
    .await
}

#[tauri::command]
fn reveal_shot(path: String) -> Result<(), String> {
    let cfg = config()?;
    let root = std::fs::canonicalize(&cfg.game_root).map_err(|e| e.to_string())?;
    let imported = std::fs::canonicalize(&cfg.imported_shots).ok();
    let file = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
    let inside = file.starts_with(&root) || imported.is_some_and(|i| file.starts_with(i));
    if !inside || !file.is_file() {
        return Err(text(EngineError::Invalid("source not allowed".into())));
    }
    let shown = file.display().to_string();
    let shown = shown.strip_prefix("\\\\?\\").unwrap_or(&shown).to_string();
    std::process::Command::new("explorer")
        .arg(format!("/select,{shown}"))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn watch(app: AppHandle, cfg: Config, code: String, appear: Duration, launcher: Option<String>) {
    if WATCHING.swap(true, Ordering::SeqCst) {
        return;
    }
    CANCEL_WATCH.store(false, Ordering::SeqCst);
    std::thread::spawn(move || {
        let started_at = std::sync::Mutex::new(None::<std::time::Instant>);
        let end = s4im_engine::wait_for_session_ex(
            s4im_engine::GAME_PROCESS,
            launcher.as_deref(),
            appear,
            Duration::from_secs(20),
            Duration::from_secs(2),
            &CANCEL_WATCH,
            || {
                *started_at.lock().unwrap() = Some(std::time::Instant::now());
                let _ = app.emit("game-started", code.clone());
            },
        );
        let elapsed = started_at.lock().unwrap().map(|s| s.elapsed().as_secs()).unwrap_or(0);
        let started = end == SessionEnd::Exited;
        let error = if started {
            s4im_engine::close_session(&cfg.game_root, &cfg.snapshots_root, &code, elapsed)
                .err()
                .map(text)
        } else {
            None
        };
        WATCHING.store(false, Ordering::SeqCst);
        let _ = app.emit("game-ended", SessionEvent { code, started, error });
    });
}

#[tauri::command]
async fn play(app: AppHandle, code: String, mode: String) -> Result<(), String> {
    blocking(move || play_blocking(app, code, mode)).await
}

fn play_blocking(app: AppHandle, code: String, mode: String) -> Result<(), String> {
    let cfg = config()?;
    let settings = settings()?;
    if WATCHING.load(Ordering::SeqCst) {
        return Err("err.sessionBusy".into());
    }
    let multiplayer = mode == "s4mp" && settings.multiplayer.enabled;
    if multiplayer && settings.multiplayer.launcher.is_none() {
        return Err(text(EngineError::Invalid("s4mp".into())));
    }
    s4im_engine::activate_instance(&cfg.game_root, &cfg.snapshots_root, &code, true).map_err(text)?;
    if multiplayer {
        s4im_engine::launch_s4mp(&settings).map_err(text)?;
    } else {
        s4im_engine::launch_game(&settings).map_err(text)?;
    }
    if settings.s4mm.enabled && settings.s4mm.launch_with_game {
        let _ = s4im_engine::launch_s4mm(&settings);
    }
    let launcher = multiplayer
        .then(|| settings.multiplayer.launcher.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()))
        .flatten();
    watch(app, cfg, code, Duration::from_secs(180), launcher);
    Ok(())
}

#[tauri::command]
fn s4mm_running() -> bool {
    s4im_engine::process_running(s4im_engine::S4MM_PROCESS)
}

#[tauri::command]
async fn open_s4mm(code: Option<String>) -> Result<(), String> {
    blocking(move || open_s4mm_blocking(code)).await
}

fn open_s4mm_blocking(code: Option<String>) -> Result<(), String> {
    let cfg = config()?;
    let settings = settings()?;
    if !settings.s4mm.enabled {
        return Err(text(EngineError::Invalid("s4mm".into())));
    }
    if let Some(code) = code {
        s4im_engine::activate_instance(&cfg.game_root, &cfg.snapshots_root, &code, true)
            .map_err(text)?;
    }
    s4im_engine::launch_s4mm(&settings).map_err(text)
}

#[tauri::command]
fn watch_game(app: AppHandle) -> Result<(), String> {
    let cfg = config()?;
    if let (true, Some(code)) = (
        s4im_engine::game_running(),
        s4im_engine::active_instance_id(&cfg.game_root),
    ) {
        watch(app, cfg, code, Duration::from_secs(5), None);
    }
    Ok(())
}

#[tauri::command]
fn cancel_watch() {
    CANCEL_WATCH.store(true, Ordering::SeqCst);
}

fn log_panics() {
    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        let Ok(base) = std::env::var("LOCALAPPDATA") else { return };
        let dir = PathBuf::from(base).join("Plumbob Launcher");
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("crash.log")) {
            let _ = writeln!(f, "{info}\n{}\n", std::backtrace::Backtrace::force_capture());
        }
    }));
}

fn main() {
    log_panics();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            detect_environment,
            get_status,
            list_instances,
            save_order,
            list_snapshots,
            list_saves,
            create_snapshot,
            restore_snapshot,
            delete_snapshot,
            update_instance,
            s4mp_leftovers,
            convert_instance,
            duplicate_instance,
            delete_instance,
            open_folder,
            instance_health,
            open_instance_file,
            check_game_root,
            check_game_folder,
            current_game_folder,
            scan_mods,
            compare_mods,
            check_code,
            game_version,
            cache_items,
            clear_cache,
            archive_instance,
            list_archived,
            restore_archived,
            diagnose,
            music_genres,
            list_music,
            add_music,
            delete_track,
            space_children,
            space_top_files,
            get_notes,
            save_notes,
            export_text,
            read_import,
            shots_status,
            shots_index,
            list_shots,
            reveal_shot,
            import_shots,
            move_shots,
            delete_shots,
            enable_tray_sync,
            disable_tray_sync,
            register_main,
            import_instance,
            create_instance,
            adopt_instance,
            play,
            open_s4mm,
            s4mm_running,
            watch_game,
            cancel_watch
        ])
        .run(tauri::generate_context!())
        .expect("error while running Plumbob Launcher");
}
