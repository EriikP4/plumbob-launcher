import type { ThemeSettings } from "./types";
import { backend } from "./api";

export type Vars = Record<string, string>;

const EFFECT_LEVEL = { mica: 0.55, acrylic: 0.5 } as const;

export const PRESETS: Record<string, { scheme: "dark" | "light"; vars: Vars; effect?: "mica" | "acrylic" }> = {
  plumbob: {
    scheme: "dark",
    vars: { "--bg": "#130f1c", "--panel": "#1b1528", "--panel2": "#241b36", "--line": "#34294d", "--fg": "#efeaf8", "--muted": "#a496c2", "--accent": "#a678ff", "--accent-fg": "#1a0836", "--warn": "#f0b64a", "--glow": "166, 120, 255" },
  },
  midnight: {
    scheme: "dark",
    vars: { "--bg": "#0d1117", "--panel": "#161b22", "--panel2": "#1f2630", "--line": "#30363d", "--fg": "#e6edf3", "--muted": "#8b949e", "--accent": "#58a6ff", "--accent-fg": "#04152b", "--warn": "#e3b341", "--glow": "88, 166, 255" },
  },
  forest: {
    scheme: "dark",
    vars: { "--bg": "#0e1612", "--panel": "#14201a", "--panel2": "#1b2c23", "--line": "#2a4234", "--fg": "#e8f3ec", "--muted": "#8fb09b", "--accent": "#4ade80", "--accent-fg": "#052e16", "--warn": "#f0b64a", "--glow": "74, 222, 128" },
  },
  mica: {
    scheme: "dark",
    effect: "mica",
    vars: { "--bg": "#1c1526", "--panel": "rgba(46, 35, 66, 0.9)", "--panel2": "rgba(66, 52, 94, 0.92)", "--line": "rgba(196, 161, 255, 0.24)", "--fg": "#f7f1ff", "--muted": "#c0b3da", "--accent": "#c4a1ff", "--accent-fg": "#1d0a3a", "--warn": "#f3c06a", "--glow": "196, 161, 255" },
  },
  acrylic: {
    scheme: "dark",
    effect: "acrylic",
    vars: { "--bg": "#0a1630", "--panel": "rgba(16, 36, 70, 0.88)", "--panel2": "rgba(28, 56, 102, 0.9)", "--line": "rgba(150, 205, 255, 0.3)", "--fg": "#eef6ff", "--muted": "#a8c2e0", "--accent": "#6cc4ff", "--accent-fg": "#03182e", "--warn": "#f0c25a", "--glow": "108, 196, 255" },
  },
  light: {
    scheme: "light",
    vars: { "--bg": "#f4f1fa", "--panel": "#ffffff", "--panel2": "#efe9f8", "--line": "#d9cfe9", "--fg": "#1e1530", "--muted": "#6b5f86", "--accent": "#7c4dff", "--accent-fg": "#ffffff", "--warn": "#b7791f", "--glow": "124, 77, 255" },
  },
};

const HEX = /^#([0-9a-f]{6})$/i;

export const isHex = (s: string) => HEX.test(s);

const rgb = (hex: string): [number, number, number] => {
  const n = parseInt(hex.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
};

const readable = (hex: string) => {
  const [r, g, b] = rgb(hex);
  return (0.299 * r + 0.587 * g + 0.114 * b) / 255 > 0.6 ? "#16091f" : "#ffffff";
};

export const SAFE_VAR = /^--[a-z0-9-]{1,32}$/;

export const buildTheme = (t: ThemeSettings) => {
  const preset = PRESETS[t.preset] ?? PRESETS.plumbob;
  const vars: Vars = { ...preset.vars };
  if (t.accent && isHex(t.accent)) {
    vars["--accent"] = t.accent;
    vars["--accent-fg"] = readable(t.accent);
    vars["--glow"] = rgb(t.accent).join(", ");
  }
  for (const [k, v] of Object.entries(t.custom)) {
    if (SAFE_VAR.test(k) && typeof v === "string" && v.length <= 64 && !/[;{}<>]/.test(v)) vars[k] = v;
  }
  return { vars, scheme: preset.scheme, effect: preset.effect };
};

export const applyTheme = (t: ThemeSettings) => {
  const { vars, scheme, effect } = buildTheme(t);
  const root = document.documentElement;
  for (const [k, v] of Object.entries(vars)) root.style.setProperty(k, v);
  root.style.colorScheme = scheme;
  root.style.background = vars["--bg"];
  root.classList.toggle("no-anim", !t.animations);
  if (effect) root.dataset.fx = effect;
  else delete root.dataset.fx;
  const level = effect ? EFFECT_LEVEL[effect] : 0;
  const bg = vars["--bg"];
  if (level > 0 && isHex(bg)) {
    root.style.setProperty("--bg-fx", `rgba(${rgb(bg).join(", ")}, ${(1 - level * 0.9).toFixed(2)})`);
    root.style.background = "transparent";
  } else {
    root.style.removeProperty("--bg-fx");
  }
  const accent = vars["--accent"];
  const tint = isHex(bg) && isHex(accent) ? ([...rgb(bg).map((c, i) => Math.round(c * 0.72 + rgb(accent)[i] * 0.28)), Math.round(60 + 50 * (1 - level))] as [number, number, number, number]) : null;
  void backend.setWindowEffect(effect ?? "none", tint).catch(() => undefined);
  try {
    localStorage.setItem("plumbob-bg", vars["--bg"]);
  } catch {
    return;
  }
};

export const parseThemeJson = (text: string): Partial<ThemeSettings> | null => {
  try {
    const raw = JSON.parse(text);
    if (typeof raw !== "object" || raw === null) return null;
    const out: Partial<ThemeSettings> = {};
    if (typeof raw.preset === "string" && raw.preset in PRESETS) out.preset = raw.preset;
    if (typeof raw.accent === "string" && isHex(raw.accent)) out.accent = raw.accent;
    if (typeof raw.animations === "boolean") out.animations = raw.animations;
    if (typeof raw.custom === "object" && raw.custom) {
      out.custom = Object.fromEntries(
        Object.entries(raw.custom as Record<string, unknown>).filter(
          ([k, v]) => SAFE_VAR.test(k) && typeof v === "string" && v.length <= 64 && !/[;{}<>]/.test(v),
        ),
      ) as Record<string, string>;
    }
    return Object.keys(out).length ? out : null;
  } catch {
    return null;
  }
};
