import type { Instance, SaveInfo, Snapshot } from "./types";

const portrait = (hue: number) =>
  `data:image/svg+xml;utf8,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="180" height="120"><g fill="hsl(${hue},55%,60%)"><circle cx="60" cy="40" r="18"/><rect x="42" y="60" width="36" height="50" rx="12"/><circle cx="110" cy="44" r="16"/><rect x="94" y="62" width="32" height="48" rx="12"/></g></svg>`)}`;

const GB = 1024 ** 3;
const now = Math.floor(Date.now() / 1000);
const ago = (hours: number) => now - hours * 3600;

export const INSTANCES: Instance[] = [
  { id: "solo", name: "Solo", kind: "solo", description: "Your main save.", colors: ["#2f8f5b", "#0f3b2a"], modCount: 214, sizeBytes: 6.8 * GB, lastPlayedAt: ago(1), saveCount: 9, trayCount: 37, traySynced: true, trayOld: true, conflicts: 0, topMods: ["MC Command Center", "UI Cheats Extension", "Slice of Life", "Wonderful Whims", "Better BuildBuy"], legacy: false, playSeconds: 3600 * 41 + 1500, gameVersion: "1.128.90.1030" },
  { id: "rl", name: "Real Life Project", kind: "solo", description: "Real life rules, no cheats.", colors: ["#c9822b", "#4a2a0c"], modCount: 132, sizeBytes: 4.1 * GB, lastPlayedAt: ago(30), saveCount: 4, trayCount: 18, traySynced: false, trayOld: false, conflicts: 0, topMods: ["MC Command Center", "Basemental", "Real Life Pack", "Lot51 Core"], legacy: false, playSeconds: 3600 * 41 + 1500, gameVersion: "1.128.90.1030" },
  { id: "vr", name: "VR Mode", kind: "solo", description: "Profile with the virtual reality mod.", colors: ["#6b5bd6", "#1b1745"], modCount: 38, sizeBytes: 1.3 * GB, lastPlayedAt: ago(24 * 12), saveCount: 2, trayCount: 4, traySynced: false, trayOld: true, conflicts: 1, topMods: ["Sims 4 VR", "Camera Fix", "UI Cheats Extension"], legacy: false, playSeconds: 3600 * 41 + 1500, gameVersion: "1.128.90.1030" },
  { id: "cus", name: "CAS / Build Mode", kind: "solo", description: "Only create sims and build. Lots of CC.", colors: ["#d65b8c", "#4a1530"], modCount: 1480, sizeBytes: 18.4 * GB, lastPlayedAt: ago(24 * 3), saveCount: 1, trayCount: 9, traySynced: false, trayOld: false, conflicts: 0, topMods: ["Pack de peinados", "Pack de ropa", "Muebles build/buy", "Slider de CAS"], legacy: false, playSeconds: 3600 * 41 + 1500, gameVersion: "1.128.90.1030" },
  { id: "mp1", name: "Cheesies", kind: "mp", description: "Shared save through S4MP.", colors: ["#e1b93a", "#4a3a08"], modCount: 61, sizeBytes: 2.2 * GB, lastPlayedAt: ago(24 * 5), saveCount: 6, trayCount: 22, traySynced: true, trayOld: false, conflicts: 0, topMods: ["S4MP Client", "Sync Fix", "MC Command Center"], legacy: false, playSeconds: 3600 * 41 + 1500, gameVersion: "1.128.90.1030" },
  { id: "mp2", name: "Mother and Son", kind: "mp", description: "Two player save.", colors: ["#3aa7d6", "#0c3347"], modCount: 57, sizeBytes: 2.0 * GB, lastPlayedAt: ago(24 * 14), saveCount: 3, trayCount: 12, traySynced: false, trayOld: false, conflicts: 0, topMods: ["S4MP Client", "Sync Fix", "Basemental"], legacy: false, playSeconds: 3600 * 41 + 1500, gameVersion: "1.128.90.1030" },
];

export const SNAPSHOTS: Snapshot[] = [
  { id: "s1", label: "snapshot.beforeActivate", created: ago(1), trigger: "auto" },
  { id: "s2", label: "snapshot.sessionClose", created: ago(24), trigger: "auto" },
  { id: "s3", label: "Before updating mods", created: ago(24 * 6), trigger: "manual" },
];

export const SAVES: SaveInfo[] = [
  { slot: 3, sizeBytes: 148 * 1024 ** 2, modified: ago(1), versions: 6, thumb: portrait(200) },
  { slot: 1, sizeBytes: 96 * 1024 ** 2, modified: ago(24 * 4), versions: 5, thumb: portrait(320) },
  { slot: 2, sizeBytes: 31 * 1024 ** 2, modified: ago(24 * 21), versions: 3, thumb: null },
];
