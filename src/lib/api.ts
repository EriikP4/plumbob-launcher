import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  ArchivedInstance, CacheItem, ClearReport, Diagnostics, CodeCheck, GameFolderCheck, ModDiff, ModFile, ModScan, Role, DeleteStats, Environment, Genre, Health, RootCheck, SpaceEntry, ImportInput, Instance, InstanceInput, InstanceKind, LaunchMode, SaveInfo, SessionEnd, Settings, Shot,
  ShotsProgress, ShotsStatus, Snapshot, Status, Transfer, TrayDisable,
} from "./types";
import { INSTANCES, SAVES, SNAPSHOTS } from "./sample";

export interface Backend {
  isSample: boolean;
  getSettings(): Promise<Settings>;
  saveSettings(settings: Settings): Promise<void>;
  detectEnvironment(): Promise<Environment>;
  status(): Promise<Status>;
  listInstances(): Promise<Instance[]>;
  listSnapshots(instanceId: string): Promise<Snapshot[]>;
  listSaves(instanceId: string): Promise<SaveInfo[]>;
  createSnapshot(instanceId: string): Promise<void>;
  restoreSnapshot(instanceId: string, snapshotId: string): Promise<void>;
  updateInstance(code: string, name: string, description: string, role?: Role | null): Promise<void>;
  s4mpLeftovers(code: string): Promise<string[]>;
  convertInstance(code: string, kind: InstanceKind, removeS4mp: boolean): Promise<void>;
  duplicateInstance(source: string, name: string, description: string, code: string): Promise<void>;
  deleteInstance(code: string): Promise<DeleteStats>;
  onDeleteProgress(cb: (p: ShotsProgress) => void): Promise<() => void>;
  openFolder(code: string, which: "root" | "mods" | "saves"): Promise<void>;
  instanceHealth(code: string): Promise<Health>;
  spaceChildren(code: string, rel: string): Promise<SpaceEntry[]>;
  spaceTopFiles(code: string, limit: number): Promise<SpaceEntry[]>;
  checkGameRoot(path: string): Promise<RootCheck>;
  checkGameFolder(path: string): Promise<GameFolderCheck>;
  currentGameFolder(): Promise<string>;
  scanMods(code: string): Promise<ModScan>;
  onHashProgress(cb: (p: ShotsProgress) => void): Promise<() => void>;
  compareMods(host: ModFile[], local: ModFile[]): Promise<ModDiff>;
  checkCode(local: string, host: string): Promise<CodeCheck>;
  gameVersion(): Promise<string | null>;
  cacheItems(code: string): Promise<CacheItem[]>;
  clearCache(code: string, ids: string[]): Promise<ClearReport>;
  archiveInstance(code: string, cleanCache: boolean, compress: boolean): Promise<void>;
  listArchived(): Promise<ArchivedInstance[]>;
  restoreArchived(code: string): Promise<void>;
  onArchiveProgress(cb: (p: ShotsProgress) => void): Promise<() => void>;
  diagnose(): Promise<Diagnostics>;
  onCacheProgress(cb: (p: ShotsProgress) => void): Promise<() => void>;
  musicGenres(): Promise<string[]>;
  listMusic(code: string): Promise<Genre[]>;
  addMusic(code: string, genre: string, files: string[]): Promise<number>;
  deleteTrack(code: string, path: string): Promise<void>;
  onMusicProgress(cb: (p: ShotsProgress) => void): Promise<() => void>;
  openInstanceFile(code: string, rel: string): Promise<void>;
  deleteSnapshot(instanceId: string, snapshotId: string): Promise<void>;
  getNotes(code: string): Promise<{ json: string; legacy: string }>;
  saveNotes(code: string, json: string): Promise<void>;
  exportText(path: string, text: string): Promise<void>;
  readImport(path: string): Promise<string>;
  shotsStatus(): Promise<ShotsStatus>;
  shotsIndex(): Promise<number>;
  listShots(instance: string | null): Promise<Shot[]>;
  revealShot(path: string): Promise<void>;
  importShots(paths: string[], target: string | null): Promise<Transfer>;
  moveShots(paths: string[], target: string | null): Promise<Transfer>;
  deleteShots(paths: string[]): Promise<Transfer>;
  enableTraySync(code: string): Promise<void>;
  disableTraySync(code: string, mode: TrayDisable): Promise<void>;
  onShotsProgress(cb: (p: ShotsProgress) => void): Promise<() => void>;
  registerMain(input: Omit<InstanceInput, "kind">): Promise<void>;
  createInstance(input: InstanceInput): Promise<void>;
  adoptInstance(input: InstanceInput): Promise<void>;
  importInstance(input: ImportInput): Promise<void>;
  play(code: string, mode: LaunchMode): Promise<void>;
  openS4mm(code: string | null): Promise<void>;
  s4mmRunning(): Promise<boolean>;
  watchGame(): Promise<void>;
  cancelWatch(): Promise<void>;
  onGameStarted(cb: () => void): Promise<() => void>;
  onSessionEnd(cb: (e: SessionEnd) => void): Promise<() => void>;
  saveOrder(ids: string[]): Promise<void>;
}

interface InstanceDto {
  id: string;
  name: string;
  kind: "solo" | "mp";
  role?: Role | null;
  description: string;
  legacy: boolean;
  modCount: number;
  saveCount: number;
  trayCount: number;
  traySynced: boolean;
  trayOld: boolean;
  sizeBytes: number;
  lastModified: number;
  lastPlayed: number | null;
  playSeconds: number;
  gameVersion: string | null;
  topMods: string[];
}

interface ArchivedDto extends InstanceDto {
  compressed: boolean;
  archivedAt: number;
  archiveBytes: number;
}

interface SnapshotDto {
  id: string;
  label: string;
  kind: "manual" | "auto";
  created: number;
}

const PALETTE: [string, string][] = [
  ["#2f8f5b", "#0f3b2a"],
  ["#c9822b", "#4a2a0c"],
  ["#6b5bd6", "#1b1745"],
  ["#d65b8c", "#4a1530"],
  ["#e1b93a", "#4a3a08"],
  ["#3aa7d6", "#0c3347"],
];

const hash = (s: string) => [...s].reduce((a, c) => (a * 31 + c.charCodeAt(0)) >>> 0, 7);

const toInstance = (d: InstanceDto): Instance => ({
  id: d.id,
  name: d.name,
  kind: d.kind,
  role: d.role ?? null,
  description: d.description,
  colors: PALETTE[hash(d.id) % PALETTE.length],
  modCount: d.modCount,
  sizeBytes: d.sizeBytes,
  lastPlayedAt: d.lastPlayed ?? d.lastModified,
  saveCount: d.saveCount,
  trayCount: d.trayCount,
  traySynced: d.traySynced,
  trayOld: d.trayOld,
  conflicts: 0,
  topMods: d.topMods,
  legacy: d.legacy,
  playSeconds: d.playSeconds,
  gameVersion: d.gameVersion,
});

const toArchived = (d: ArchivedDto): ArchivedInstance => ({
  ...toInstance(d),
  compressed: d.compressed,
  archivedAt: d.archivedAt,
  archiveBytes: d.archiveBytes,
});

const toSnapshot = (d: SnapshotDto): Snapshot => ({
  id: d.id,
  label: d.label,
  created: d.created,
  trigger: d.kind,
});

const message = (e: unknown) => (typeof e === "string" ? e : e instanceof Error ? e.message : String(e));

const call = async <T>(cmd: string, args?: Record<string, unknown>) => {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw new Error(message(e));
  }
};

const unavailable = async () => {
  throw new Error("err.onlyDesktop");
};

const sampleListeners = new Set<(e: SessionEnd) => void>();
const sampleProgress = new Set<(p: ShotsProgress) => void>();
let sampleIndexed = false;

const sampleShot = (i: number): Shot => {
  const hue = (i * 47) % 360;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="480" height="270"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${hue},60%,45%)"/><stop offset="1" stop-color="hsl(${(hue + 60) % 360},60%,22%)"/></linearGradient></defs><rect width="480" height="270" fill="url(#g)"/></svg>`;
  const url = `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
  const sizes = [[854, 480], [1280, 720], [1920, 1080], [2560, 1440], [3840, 2160], [7680, 4320]];
  const [width, height] = sizes[i % sizes.length];
  return { id: `s${i}`, instance: i % 7 === 0 ? "~imported" : i % 3 === 0 ? "mp1" : "solo", path: url, name: `captura-${i}.png`, size: 1_500_000, modified: Math.floor(Date.now() / 1000) - i * 5400, thumb: url, width, height };
};

const sampleSettings: Settings = {
  language: "es",
  setupDone: true,
  platform: "steam",
  steamId: null,
  installDir: null,
  customCommand: null,
  gameRoot: null,
  snapshotsRoot: null,
  s4mm: { enabled: true, path: null, launchWithGame: false },
  multiplayer: { enabled: true, launcher: null },
  theme: { preset: "plumbob", accent: null, animations: true, custom: {} },
  boot: { animation: true, sound: true },
  shots: { qualityBadges: false },
  traySyncDefault: false,
  autoSnapshots: true,
  hideUpdateNotice: false,
  gameFolder: null,
  musicWelcomeSeen: false,
};

const GB_SAMPLE = 1024 ** 3;

export const sampleBackend: Backend = {
  isSample: true,
  getSettings: async () => structuredClone(sampleSettings),
  saveSettings: async (s) => void Object.assign(sampleSettings, structuredClone(s)),
  detectEnvironment: async () => ({ s4mm: null, steam: null, ea: null, epic: null }),
  status: async () => ({ needsSetup: false, activeId: "solo", gameRunning: false, gameFolderChoose: false }),
  listInstances: async () => INSTANCES,
  listSnapshots: async () => SNAPSHOTS,
  listSaves: async () => SAVES,
  createSnapshot: async () => undefined,
  restoreSnapshot: async () => undefined,
  updateInstance: unavailable,
  s4mpLeftovers: async () => ["S4MP_logs", "S4MP_release.ts4script", "s4mpconfig"],
  convertInstance: unavailable,
  duplicateInstance: unavailable,
  openFolder: async () => undefined,
  instanceHealth: async () => ({
    mods: [
      { id: "betterexceptions", name: "BetterExceptions", installed: false, version: null },
      { id: "mccc", name: "MC Command Center", installed: true, version: "2026.5.0" },
      { id: "xmlinjector", name: "XML Injector", installed: true, version: "4.2" },
      { id: "wickedwhims", name: "WickedWhims", installed: true, version: "185k" },
    ],
    warnings: [{ kind: "archives", count: 1, items: ["pack.zip"] }],
    lastException: Math.floor(Date.now() / 1000) - 86400,
    lastExceptionFile: "BE-ExceptionReport.html",
    lastExceptionSource: "betterexceptions",
    modFiles: 830,
  }),
  spaceChildren: async (_code, rel) => rel ? [
    { name: "Animations", rel: rel + "/Animations", bytes: 1.4 * GB_SAMPLE, files: 20, isDir: true },
    { name: "Addons", rel: rel + "/Addons", bytes: 0.2 * GB_SAMPLE, files: 9, isDir: true },
  ] : [
    { name: "Mods", rel: "Mods", bytes: 3.6 * GB_SAMPLE, files: 830, isDir: true },
    { name: "saves", rel: "saves", bytes: 0.7 * GB_SAMPLE, files: 40, isDir: true },
    { name: "Custom Music", rel: "Custom Music", bytes: 0.4 * GB_SAMPLE, files: 12, isDir: true },
  ],
  spaceTopFiles: async () => [
    { name: "WW_Fruitydelicious_Animations.package", rel: "Mods/WickedWhimsMod/Animations/WW_Fruitydelicious_Animations.package", bytes: 0.24 * GB_SAMPLE, files: 1, isDir: false },
  ],
  gameVersion: async () => "1.128.90.1030",
  scanMods: async () => ({ files: [], code: "PB1-0-00000-00000-00000-00000", packages: 0, scripts: 0 }),
  onHashProgress: async () => () => undefined,
  compareMods: async () => ({ missing: [], extra: [], different: [] }),
  checkCode: async (local, host) => ({ valid: /^PB1-/i.test(host.trim()), equal: local.trim().toUpperCase() === host.trim().toUpperCase(), hostCount: 0 }),
  cacheItems: async () => [
    { id: "textures", bytes: 98 * 1024 ** 2, files: 1 },
    { id: "thumbs", bytes: 41 * 1024 ** 2, files: 1 },
    { id: "onlinethumbs", bytes: 2.5 * 1024 ** 2, files: 30 },
    { id: "cache", bytes: 0.3 * 1024 ** 2, files: 8 },
    { id: "cachestr", bytes: 0.01 * 1024 ** 2, files: 1 },
    { id: "errors", bytes: 0.4 * 1024 ** 2, files: 5 },
    { id: "logs", bytes: 0.15 * 1024 ** 2, files: 11 },
    { id: "avatar", bytes: 0.07 * 1024 ** 2, files: 1 },
  ],
  clearCache: unavailable,
  archiveInstance: unavailable,
  listArchived: async () => [
    { ...INSTANCES[INSTANCES.length - 1], id: "old1", name: "Old save", compressed: true, archivedAt: Math.floor(Date.now() / 1000) - 86400 * 12, archiveBytes: 1.1 * GB_SAMPLE },
  ],
  restoreArchived: unavailable,
  onArchiveProgress: async () => () => undefined,
  diagnose: async () => ({ gameRoot: "C:\Users\You\OneDrive\Documents\Electronic Arts", gameDir: "C:\Users\You\OneDrive\Documents\Electronic Arts\The Sims 4", gameDirExists: true, oneDrive: true, cloudFiles: 3, lockedFiles: 0, lockedSample: null, writeError: null, gameRunning: false }),
  onCacheProgress: async () => () => undefined,
  musicGenres: async () => ["Alternative", "Blues", "Classical", "Electronica", "Hip Hop", "Pop"],
  listMusic: async () => [],
  addMusic: unavailable,
  deleteTrack: unavailable,
  onMusicProgress: async () => () => undefined,
  currentGameFolder: async () => "C:\Users\You\Documents\Electronic Arts\The Sims 4",
  checkGameFolder: async (path) => ({ state: path.trim() ? "ok" : "invalid", root: path.replace(/[\\/][^\\/]*$/, ""), name: path.replace(/^.*[\\/]/, "") }),
  checkGameRoot: async (path) => ({ state: /Game[\\/]Bin/i.test(path) ? "inside" : path.trim() ? "ok" : "invalid", root: path.replace(/[\\/]Game([\\/]Bin)?[\\/]?$/i, ""), version: "1.128.90.1030" }),
  openInstanceFile: async () => undefined,
  deleteSnapshot: async () => undefined,
  getNotes: async () => ({ json: "", legacy: "" }),
  saveNotes: async () => undefined,
  exportText: unavailable,
  readImport: unavailable,
  deleteInstance: async (code) => {
    for (let i = 1; i <= 20; i += 1) {
      await new Promise((r) => setTimeout(r, 80));
      sampleProgress.forEach((cb) => cb({ done: i, total: 20 }));
    }
    const found = INSTANCES.find((i) => i.id === code);
    return { name: found?.name ?? code, bytes: (found?.sizeBytes ?? 0), files: 1234, mods: found?.modCount ?? 0, saves: found?.saveCount ?? 0, playSeconds: found?.playSeconds ?? 0, keptSnapshots: 2 };
  },
  onDeleteProgress: async (cb) => {
    sampleProgress.add(cb);
    return () => void sampleProgress.delete(cb);
  },
  shotsStatus: async () => ({ indexed: sampleIndexed, count: sampleIndexed ? 24 : 0 }),
  shotsIndex: async () => {
    for (let i = 1; i <= 24; i += 1) {
      await new Promise((r) => setTimeout(r, 90));
      sampleProgress.forEach((cb) => cb({ done: i, total: 24 }));
    }
    sampleIndexed = true;
    return 24;
  },
  listShots: async (instance) => Array.from({ length: 24 }, (_, i) => sampleShot(i)).filter((s) => !instance || s.instance === instance),
  revealShot: async () => undefined,
  importShots: async (paths) => ({ done: paths.length, skipped: 0 }),
  moveShots: async (paths) => ({ done: paths.length, skipped: 0 }),
  deleteShots: async (paths) => ({ done: paths.length, skipped: 0 }),
  enableTraySync: async () => undefined,
  disableTraySync: async () => undefined,
  onShotsProgress: async (cb) => {
    sampleProgress.add(cb);
    return () => void sampleProgress.delete(cb);
  },
  registerMain: unavailable,
  createInstance: unavailable,
  adoptInstance: unavailable,
  importInstance: unavailable,
  play: async (code) => {
    setTimeout(() => sampleListeners.forEach((cb) => cb({ code, started: true, error: null })), 3000);
  },
  openS4mm: async () => undefined,
  s4mmRunning: async () => false,
  watchGame: async () => undefined,
  cancelWatch: async () => undefined,
  onGameStarted: async () => () => undefined,
  onSessionEnd: async (cb) => {
    sampleListeners.add(cb);
    return () => void sampleListeners.delete(cb);
  },
  saveOrder: async () => undefined,
};

export const tauriBackend: Backend = {
  isSample: false,
  getSettings: () => call<Settings>("get_settings"),
  saveSettings: async (settings) => void (await call("save_settings", { settings })),
  detectEnvironment: () => call<Environment>("detect_environment"),
  status: () => call<Status>("get_status"),
  listInstances: async () => (await call<InstanceDto[]>("list_instances")).map(toInstance),
  listSnapshots: async (instanceId) =>
    (await call<SnapshotDto[]>("list_snapshots", { instanceId })).map(toSnapshot),
  listSaves: (instanceId) => call<SaveInfo[]>("list_saves", { instanceId }),
  createSnapshot: async (instanceId) => void (await call("create_snapshot", { instanceId })),
  restoreSnapshot: async (instanceId, snapshotId) => void (await call("restore_snapshot", { instanceId, snapshotId })),
  updateInstance: async (code, name, description, role) => void (await call("update_instance", { code, name, description, role: role ?? null })),
  s4mpLeftovers: (code) => call<string[]>("s4mp_leftovers", { code }),
  convertInstance: async (code, kind, removeS4mp) => void (await call("convert_instance", { code, kind, removeS4mp })),
  duplicateInstance: async (source, name, description, code) => void (await call("duplicate_instance", { source, name, description, code })),
  openFolder: async (code, which) => void (await call("open_folder", { code, which })),
  instanceHealth: (code) => call<Health>("instance_health", { code }),
  spaceChildren: (code, rel) => call<SpaceEntry[]>("space_children", { code, rel }),
  spaceTopFiles: (code, limit) => call<SpaceEntry[]>("space_top_files", { code, limit }),
  checkGameRoot: (path) => call<RootCheck>("check_game_root", { path }),
  checkGameFolder: (path) => call<GameFolderCheck>("check_game_folder", { path }),
  currentGameFolder: () => call<string>("current_game_folder"),
  gameVersion: () => call<string | null>("game_version"),
  scanMods: (code) => call<ModScan>("scan_mods", { code }),
  onHashProgress: (cb) => listen<ShotsProgress>("hash-progress", (e) => cb(e.payload)),
  compareMods: (host, local) => call<ModDiff>("compare_mods", { host, local }),
  checkCode: (local, host) => call<CodeCheck>("check_code", { local, host }),
  cacheItems: (code) => call<CacheItem[]>("cache_items", { code }),
  clearCache: (code, ids) => call<ClearReport>("clear_cache", { code, ids }),
  archiveInstance: async (code, cleanCache, compress) => void (await call("archive_instance", { code, cleanCache, compress })),
  listArchived: async () => (await call<ArchivedDto[]>("list_archived")).map(toArchived),
  restoreArchived: async (code) => void (await call("restore_archived", { code })),
  onArchiveProgress: (cb) => listen<ShotsProgress>("archive-progress", (e) => cb(e.payload)),
  diagnose: () => call<Diagnostics>("diagnose"),
  onCacheProgress: (cb) => listen<ShotsProgress>("cache-progress", (e) => cb(e.payload)),
  musicGenres: () => call<string[]>("music_genres"),
  listMusic: (code) => call<Genre[]>("list_music", { code }),
  addMusic: (code, genre, files) => call<number>("add_music", { code, genre, files }),
  deleteTrack: async (code, path) => void (await call("delete_track", { code, path })),
  onMusicProgress: (cb) => listen<ShotsProgress>("music-progress", (e) => cb(e.payload)),
  openInstanceFile: async (code, rel) => void (await call("open_instance_file", { code, rel })),
  deleteSnapshot: async (instanceId, snapshotId) => void (await call("delete_snapshot", { instanceId, snapshotId })),
  getNotes: (code) => call<{ json: string; legacy: string }>("get_notes", { code }),
  saveNotes: async (code, json) => void (await call("save_notes", { code, json })),
  exportText: async (path, text) => void (await call("export_text", { path, text })),
  readImport: (path) => call<string>("read_import", { path }),
  deleteInstance: (code) => call<DeleteStats>("delete_instance", { code }),
  onDeleteProgress: (cb) => listen<ShotsProgress>("delete-progress", (e) => cb(e.payload)),
  shotsStatus: () => call<ShotsStatus>("shots_status"),
  shotsIndex: () => call<number>("shots_index"),
  listShots: (instance) => call<Shot[]>("list_shots", { instance }),
  revealShot: async (path) => void (await call("reveal_shot", { path })),
  importShots: (paths, target) => call<Transfer>("import_shots", { paths, target }),
  moveShots: (paths, target) => call<Transfer>("move_shots", { paths, target }),
  deleteShots: (paths) => call<Transfer>("delete_shots", { paths }),
  enableTraySync: async (code) => void (await call("enable_tray_sync", { code })),
  disableTraySync: async (code, mode) => void (await call("disable_tray_sync", { code, mode })),
  onShotsProgress: (cb) => listen<ShotsProgress>("shots-progress", (e) => cb(e.payload)),
  registerMain: async (i) => void (await call("register_main", i)),
  createInstance: async (i) => void (await call("create_instance", { ...i })),
  adoptInstance: async (i) => void (await call("adopt_instance", { ...i })),
  importInstance: async (i) => void (await call("import_instance", { ...i })),
  play: async (code, mode) => void (await call("play", { code, mode })),
  openS4mm: async (code) => void (await call("open_s4mm", { code })),
  s4mmRunning: () => call<boolean>("s4mm_running"),
  watchGame: async () => void (await call("watch_game")),
  cancelWatch: async () => void (await call("cancel_watch")),
  onGameStarted: (cb) => listen<string>("game-started", () => cb()),
  onSessionEnd: (cb) => listen<SessionEnd>("game-ended", (e) => cb(e.payload)),
  saveOrder: async (ids) => void (await call("save_order", { ids })),
};

export const backend: Backend = "__TAURI_INTERNALS__" in window ? tauriBackend : sampleBackend;
