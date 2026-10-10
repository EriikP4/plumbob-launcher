export type InstanceKind = "solo" | "mp";

export interface Instance {
  id: string;
  name: string;
  kind: InstanceKind;
  role?: Role | null;
  description: string;
  colors: [string, string];
  modCount: number;
  sizeBytes: number;
  lastPlayedAt: number;
  saveCount: number;
  trayCount: number;
  traySynced: boolean;
  trayOld: boolean;
  conflicts: number;
  topMods: string[];
  legacy: boolean;
  playSeconds: number;
  gameVersion: string | null;
}

export interface SaveInfo {
  slot: number;
  sizeBytes: number;
  modified: number;
  versions: number;
  thumb: string | null;
}

export interface Status {
  needsSetup: boolean;
  activeId: string | null;
  gameRunning: boolean;
  gameFolderChoose: boolean;
}

export interface SessionEnd {
  code: string;
  started: boolean;
  error: string | null;
}

export type Role = "host" | "client";

export interface ModFile {
  name: string;
  path: string;
  size: number;
  hash: string;
}

export interface ModScan {
  files: ModFile[];
  code: string;
  packages: number;
  scripts: number;
}

export interface ModChange {
  host: ModFile;
  local: ModFile;
}

export interface ModDiff {
  missing: ModFile[];
  extra: ModFile[];
  different: ModChange[];
}

export interface CodeCheck {
  valid: boolean;
  equal: boolean;
  hostCount: number | null;
}

export type MusicChoice = "keep" | "half" | "off";

export interface GameOptions {
  mods: boolean;
  skipDlcIntro: boolean;
  noTelemetry: boolean;
  music: MusicChoice;
  windowed: boolean;
}

export interface InstanceInput {
  name: string;
  description: string;
  code: string;
  kind: InstanceKind;
  role?: Role | null;
  options?: GameOptions;
}

export interface Snapshot {
  id: string;
  label: string;
  created: number;
  trigger: "manual" | "auto";
}

export type LaunchMode = "offline" | "s4mp";

export type Platform = "steam" | "ea" | "epic" | "custom";

export interface ThemeSettings {
  preset: string;
  accent: string | null;
  animations: boolean;
  custom: Record<string, string>;
}

export interface Settings {
  language: string;
  setupDone: boolean;
  platform: Platform;
  steamId: string | null;
  installDir: string | null;
  customCommand: string | null;
  gameRoot: string | null;
  snapshotsRoot: string | null;
  s4mm: { enabled: boolean; path: string | null; launchWithGame: boolean };
  multiplayer: { enabled: boolean; launcher: string | null };
  theme: ThemeSettings;
  boot: { animation: boolean; sound: boolean };
  shots: { qualityBadges: boolean };
  traySyncDefault: boolean;
  autoSnapshots: boolean;
  hideUpdateNotice: boolean;
  gameFolder: string | null;
  musicWelcomeSeen: boolean;
}

export interface Environment {
  s4mm: string | null;
  steam: string | null;
  ea: string | null;
  epic: string | null;
}

export type ImportMode = "move" | "copy";

export interface ImportInput extends InstanceInput {
  path: string;
  mode: ImportMode;
}

export interface Shot {
  id: string;
  instance: string;
  path: string;
  name: string;
  size: number;
  modified: number;
  thumb: string | null;
  width: number;
  height: number;
}

export interface ShotsStatus {
  indexed: boolean;
  count: number;
}

export interface ShotsProgress {
  done: number;
  total: number;
}

export interface DeleteStats {
  name: string;
  bytes: number;
  files: number;
  mods: number;
  saves: number;
  playSeconds: number;
  keptSnapshots: number;
}

export interface Transfer {
  done: number;
  skipped: number;
}

export type TrayDisable = "restore" | "merge" | "empty";

export interface RootCheck {
  state: "ok" | "inside" | "invalid";
  root: string | null;
  version: string | null;
}

export interface ModStatus {
  id: string;
  name: string;
  installed: boolean;
  version: string | null;
}

export interface HealthWarning {
  kind: "deepScripts" | "deepPackages" | "archives" | "duplicates";
  count: number;
  items: string[];
}

export interface Health {
  mods: ModStatus[];
  warnings: HealthWarning[];
  lastException: number | null;
  lastExceptionFile: string | null;
  lastExceptionSource: "betterexceptions" | "mccc" | "game" | null;
  modFiles: number;
}

export interface SpaceEntry {
  name: string;
  rel: string;
  bytes: number;
  files: number;
  isDir: boolean;
}

export interface CacheItem {
  id: "thumbs" | "textures" | "avatar" | "cache" | "cachestr" | "onlinethumbs" | "errors" | "logs";
  bytes: number;
  files: number;
}

export interface ClearReport {
  freed: number;
  failed: number;
}

export interface Track {
  name: string;
  path: string;
  bytes: number;
}

export interface Genre {
  name: string;
  tracks: Track[];
}

export interface GameFolderCheck {
  state: "ok" | "invalid";
  root: string | null;
  name: string | null;
}

export interface ArchivedInstance extends Instance {
  compressed: boolean;
  archivedAt: number;
  archiveBytes: number;
}

export interface Diagnostics {
  gameRoot: string;
  gameDir: string;
  gameDirExists: boolean;
  oneDrive: boolean;
  cloudFiles: number;
  lockedFiles: number;
  lockedSample: string | null;
  writeError: string | null;
  gameRunning: boolean;
}
