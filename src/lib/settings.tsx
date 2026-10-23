import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import { backend } from "./api";
import type { Settings } from "./types";
import { PRESETS, applyTheme } from "./themes";
import { translate, translateMessage, type Key } from "../i18n";

type Vars = Record<string, string | number>;

interface Tools {
  t: (key: Key, vars?: Vars) => string;
  msg: (text: string) => string;
  size: (bytes: number) => string;
  when: (secs: number) => string;
  duration: (secs: number) => string;
}

interface Ctx extends Tools {
  settings: Settings;
  save: (next: Settings) => Promise<void>;
  saveSoon: (next: Settings) => void;
}

const SettingsContext = createContext<Ctx | null>(null);

const SAVE_DELAY = 350;

const makeTools = (lang: string): Tools => {
  const t = (key: Key, vars?: Vars) => translate(lang, key, vars);
  return {
    t,
    msg: (text) => translateMessage(lang, text),
    size: (bytes) => {
      const gb = bytes / 1024 ** 3;
      if (gb >= 1) return t("size.gb", { n: gb.toFixed(1).replace(".", lang === "es" ? "," : ".") });
      const dec = lang === "es" ? "," : ".";
      const mb = bytes / 1024 ** 2;
      if (mb >= 1) return t("size.mb", { n: mb < 10 ? mb.toFixed(1).replace(".", dec) : Math.round(mb) });
      const kb = bytes / 1024;
      if (kb >= 1) return t("size.kb", { n: Math.round(kb) });
      return t("size.b", { n: Math.round(bytes) });
    },
    duration: (secs) => {
      const minutes = Math.floor(secs / 60);
      if (minutes <= 0) return t("time.none");
      return minutes < 60 ? t("time.minutes", { n: minutes }) : t("time.hoursMinutes", { h: Math.floor(minutes / 60), m: minutes % 60 });
    },
    when: (secs) => {
      const d = new Date(secs * 1000);
      const days = Math.floor((Date.now() - d.getTime()) / 86400000);
      const time = d.toLocaleTimeString(lang, { hour: "2-digit", minute: "2-digit" });
      if (days <= 0) return t("time.today", { t: time });
      if (days === 1) return t("time.yesterday", { t: time });
      if (days < 30) return t("time.daysAgo", { n: days });
      return d.toLocaleDateString(lang);
    },
  };
};

export function SettingsProvider({ children }: { children: React.ReactNode }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [failed, setFailed] = useState("");
  const timer = useRef<number | undefined>(undefined);
  const latest = useRef<Settings | null>(null);

  useEffect(() => {
    backend.getSettings().then((s) => setSettings(s.theme.preset in PRESETS ? s : { ...s, theme: { ...s.theme, preset: "plumbob" } })).catch((e) => setFailed(String(e)));
  }, []);

  const theme = settings?.theme;
  const language = settings?.language;

  useEffect(() => {
    if (theme) applyTheme(theme);
  }, [theme]);

  useEffect(() => {
    if (language) document.documentElement.lang = language;
  }, [language]);

  useEffect(() => {
    const root = document.documentElement;
    const update = () => root.classList.toggle("unfocused", document.hidden || !document.hasFocus());
    update();
    window.addEventListener("focus", update);
    window.addEventListener("blur", update);
    document.addEventListener("visibilitychange", update);
    return () => {
      window.removeEventListener("focus", update);
      window.removeEventListener("blur", update);
      document.removeEventListener("visibilitychange", update);
      root.classList.remove("unfocused");
    };
  }, []);

  const flush = useCallback(async () => {
    if (timer.current !== undefined) {
      window.clearTimeout(timer.current);
      timer.current = undefined;
    }
    const pending = latest.current;
    latest.current = null;
    if (pending) await backend.saveSettings(pending);
  }, []);

  const save = useCallback(async (next: Settings) => {
    setSettings(next);
    latest.current = next;
    await flush();
  }, [flush]);

  const saveSoon = useCallback((next: Settings) => {
    setSettings(next);
    latest.current = next;
    if (timer.current !== undefined) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => void flush().catch(() => undefined), SAVE_DELAY);
  }, [flush]);

  const tools = useMemo(() => makeTools(language ?? "es"), [language]);

  const value = useMemo<Ctx | null>(
    () => (settings ? { settings, save, saveSoon, ...tools } : null),
    [settings, save, saveSoon, tools],
  );

  if (!value) return failed ? <p className="note">{failed}</p> : null;
  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}

export function useSettings() {
  const ctx = useContext(SettingsContext);
  if (!ctx) throw new Error("SettingsProvider missing");
  return ctx;
}
