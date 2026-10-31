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
  skipEaIntro: boolean;
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
  presence: { enabled: boolean; hideNumbers: boolean };
  traySyncDefault: boolean;
  autoSnapshots: boolean;
  hideUpdateNotice: boolean;
  gameFolder: string | null;
  profile: { source: "windows" | "custom" | "sim" | "none"; custom: string | null; sim: string | null; simId: string | null; name: string | null };
  instanceIcons: Record<string, string>;
  lanHosts: Record<string, string>;
  instanceColors: Record<string, string>;
  instanceBanners: Record<string, string>;
  ignoredUpdate: string | null;
  updateSound: boolean;
  uiScale: number;
  uiAuto: boolean;
  gameArgs: string[];
  tools: Tool[];
  musicWelcomeSeen: boolean;
  announcementsEnabled: boolean;
  readAnnouncements: string[];
  toastedAnnouncements: string[];
}

export interface PresenceState {
  section: "menu" | "instance" | "settings" | "screenshots" | "music";
  mods: number;
  bytes: number;
  shots: number;
  shotBytes: number;
  songs: number;
}

export interface Tool {
  id: string;
  name: string;
  path: string;
  icon: string | null;
  customIcon: boolean;
}

export type ModGroupId = "wickedwhims" | "basementaldrugs" | "mccc" | "translations";

export interface ModGroupInfo {
  id: ModGroupId;
  files: number;
  bytes: number;
}

export interface BaseScan {
  hasConfig: boolean;
  configFiles: string[];
  hasMods: boolean;
  modsFiles: number;
  modsBytes: number;
  groups: ModGroupInfo[];
}

export interface ModEntry {
  name: string;
  rel: string;
  isDir: boolean;
  files: number;
  bytes: number;
}

export interface DeriveSelection {
  config: boolean;
  allMods: boolean;
  groups: ModGroupId[];
  paths: string[];
}

export interface DeriveInput extends InstanceInput {
  base: string;
  selection: DeriveSelection;
}

export interface UpdateAsset {
  name: string;
  url: string;
  sigUrl: string;
  size: number;
}

export type AnnouncementKind = "info" | "update" | "event" | "warning";

export interface Announcement {
  id: string;
  date: string;
  kind: AnnouncementKind;
  pinned: boolean;
  title: string;
  body: string;
  link: { url: string; label: string | null } | null;
}

export interface UpdateInfo {
  version: string;
  tag: string;
  name: string;
  notes: string;
  page: string;
  published: string | null;
  installer: UpdateAsset | null;
}

export interface LanAddress {
  ip: string;
  name: string;
}

export interface LanHostInfo {
  addresses: LanAddress[];
  listen: string;
  port: number;
  name: string;
  files: number;
  bytes: number;
  gameVersion: string | null;
}

export interface LanPending {
  ip: string;
  name: string;
}

export interface LanHostStatus {
  clients: number;
  servedBytes: number;
  pending: LanPending[];
  trusted: string[];
}

export interface LanItem {
  path: string;
  name: string;
  size: number;
  hash: string;
  group: string | null;
}

export interface LanChange {
  host: LanItem;
  local: LanItem;
}

export interface LanDiff {
  hostName: string;
  hostVersion: string | null;
  localVersion: string | null;
  hostFiles: number;
  localFiles: number;
  missing: LanItem[];
  different: LanChange[];
  extra: LanItem[];
}

export interface LanDownload {
  hostPath: string;
  hash: string;
  size: number;
  dest: string;
}

export interface LanPlan {
  download: LanDownload[];
  remove: string[];
  backup: boolean;
}

export interface LanReport {
  added: number;
  replaced: number;
  removed: number;
  bytes: number;
  backupDir: string | null;
}

export interface LanProgress {
  phase: "download" | "apply";
  done: number;
  total: number;
}

export interface TrayOldInfo {
  path: string;
  files: number;
  bytes: number;
}

export interface Bust {
  id: string;
  path: string;
}

export interface EffectSupport {
  mica: boolean;
  acrylic: boolean;
}

export interface ProfileInfo {
  name: string;
  systemName: string;
  image: string | null;
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
