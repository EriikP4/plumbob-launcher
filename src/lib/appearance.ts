import { backend } from "./api";
import { pickImages } from "./native";
import { assetUrl } from "./assets";
import type { Instance, Settings } from "./types";

export const PRESET_COLORS = ["#b5cf75", "#7fa9ff", "#56d6c6", "#b9bcff", "#ff9de6", "#ffb978", "#ffa3b3", "#8cd98a"];

export const isHex = (v: string | undefined | null): v is string => !!v && /^#[0-9a-fA-F]{6}$/.test(v);

export const randomPreset = () => PRESET_COLORS[Math.floor(Math.random() * PRESET_COLORS.length)];

const mix = (hex: string, amount: number) => {
  const n = parseInt(hex.slice(1), 16);
  const ch = (shift: number) => Math.round(((n >> shift) & 255) * (1 - amount));
  return "#" + [16, 8, 0].map((s) => ch(s).toString(16).padStart(2, "0")).join("");
};

export const gradientOf = (hex: string): [string, string] => [hex, mix(hex, 0.72)];

export const colorsFor = (instance: Pick<Instance, "id" | "colors">, settings: Settings): [string, string] => {
  const chosen = settings.instanceColors?.[instance.id];
  return isHex(chosen) ? gradientOf(chosen) : instance.colors;
};

export const bannerFor = (settings: Settings, id: string): string | undefined => {
  const banner = settings.instanceBanners?.[id];
  return banner && banner !== "none" ? assetUrl(banner) : undefined;
};

export type ImageChoice = { mode: "keep" } | { mode: "none" } | { mode: "new"; path: string } | { mode: "stored"; path: string };
export type BannerChoice = { mode: "none" } | { mode: "keep" } | { mode: "new"; path: string } | { mode: "stored"; path: string };

export interface Appearance {
  color: string;
  icon: ImageChoice;
  banner: BannerChoice;
}

export const defaultAppearance = (): Appearance => ({ color: "random", icon: { mode: "none" }, banner: { mode: "none" } });

export const appearanceOf = (instance: Instance, settings: Settings): Appearance => {
  const stored = settings.instanceColors?.[instance.id];
  const banner = settings.instanceBanners?.[instance.id];
  return {
    color: isHex(stored) ? stored : instance.colors[0],
    icon: settings.instanceIcons?.[instance.id] ? { mode: "keep" } : { mode: "none" },
    banner: banner && banner !== "none" ? { mode: "keep" } : { mode: "none" },
  };
};

export const colorChanged = (a: Appearance, b: Appearance) => a.color !== b.color;

export async function applyAppearance(id: string, a: Appearance, settings: Settings, setColor = true): Promise<Settings> {
  const instanceColors = { ...settings.instanceColors };
  if (setColor) instanceColors[id] = a.color === "random" || !isHex(a.color) ? randomPreset() : a.color;

  const instanceIcons = { ...settings.instanceIcons };
  if (a.icon.mode === "none") delete instanceIcons[id];
  else if (a.icon.mode === "new") instanceIcons[id] = await backend.importIcon(id, a.icon.path);
  else if (a.icon.mode === "stored") instanceIcons[id] = a.icon.path;

  const instanceBanners = { ...settings.instanceBanners };
  if (a.banner.mode === "none") delete instanceBanners[id];
  else if (a.banner.mode === "new") instanceBanners[id] = await backend.importIcon(`banner-${id}`, a.banner.path);
  else if (a.banner.mode === "stored") instanceBanners[id] = a.banner.path;

  return { ...settings, instanceColors, instanceIcons, instanceBanners };
}

export async function pickIconChoice(immediate?: string): Promise<ImageChoice | null> {
  const [picked] = await pickImages();
  if (!picked) return null;
  return immediate ? { mode: "stored", path: await backend.importIcon(immediate, picked) } : { mode: "new", path: picked };
}

export async function pickBannerChoice(immediate?: string): Promise<BannerChoice | null> {
  const [picked] = await pickImages();
  if (!picked) return null;
  return immediate ? { mode: "stored", path: await backend.importIcon(`banner-${immediate}`, picked) } : { mode: "new", path: picked };
}
