import { Fragment, useEffect, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { LANGUAGES, type Key } from "../i18n";
import { PRESETS, isHex, parseThemeJson } from "../lib/themes";
import { openLink } from "../lib/native";
import type { Environment, Settings } from "../lib/types";
import Icon from "./Icon";
import PathField from "./PathField";
import PlatformFields from "./PlatformFields";
import GameFolderPicker from "./GameFolderPicker";
import DiagPanel from "./DiagPanel";

const S4MM_URL = "https://s4mm.gametimedev.de/";
const S4MP_URL = "https://www.curseforge.com/sims4/mods/sims-4-multiplayer-mod";

const CATEGORIES = ["general", "game", "appearance", "discord", "tools", "diag"] as const;

interface Section { id: string; cat: string; keys: Key[]; el: React.ReactNode }

const THEME_NAMES = ["plumbob", "midnight", "forest", "light", "mica", "acrylic"] as const;

export default function SettingsView({ onBack, onSetup, scale }: { onBack: () => void; onSetup: () => void; scale: number }) {
  const { settings, save, saveSoon, t } = useSettings();
  const [env, setEnv] = useState<Environment | null>(null);
  const [notice, setNotice] = useState(false);
  const [themeText, setThemeText] = useState("");
  const [themeMsg, setThemeMsg] = useState("");
  const [folder, setFolder] = useState("");
  const [query, setQuery] = useState("");
  const [support, setSupport] = useState({ mica: false, acrylic: false });
  const [cat, setCat] = useState<(typeof CATEGORIES)[number]>("general");

  useEffect(() => {
    backend.detectEnvironment().then(setEnv).catch(() => undefined);
    backend.effectSupport().then(setSupport).catch(() => undefined);
    backend.currentGameFolder().then(setFolder).catch(() => undefined);
  }, [settings.gameFolder, settings.gameRoot]);

  const set = (patch: Partial<Settings>) => void save({ ...settings, ...patch });
  const setTheme = (patch: Partial<Settings["theme"]>) => set({ theme: { ...settings.theme, ...patch } });
  const { s4mm, multiplayer, theme } = settings;

  const copyTheme = async () => {
    const text = JSON.stringify({ preset: theme.preset, accent: theme.accent, animations: theme.animations, custom: theme.custom }, null, 2);
    try {
      await navigator.clipboard.writeText(text);
      setThemeMsg(t("settings.copied"));
    } catch {
      setThemeText(text);
    }
  };

  const importTheme = () => {
    const parsed = parseThemeJson(themeText);
    if (!parsed) return setThemeMsg(t("settings.themeInvalid"));
    setTheme(parsed);
    setThemeMsg("");
    setThemeText("");
  };

  const [leaving, setLeaving] = useState(false);
  const back = () => {
    setLeaving(true);
    window.setTimeout(onBack, 190);
  };

  const q = query.trim().toLowerCase();
  const sections: Section[] = [
    { id: "general", cat: "general", keys: ["settings.general", "settings.language"] as Key[], el: (
          <section className="card">
            <h2>{t("settings.general")}</h2>
            <label className="field">
              <span>{t("settings.language")}</span>
              <select value={settings.language} onChange={(e) => set({ language: e.target.value })}>
                {LANGUAGES.map((l) => <option key={l.code} value={l.code}>{l.name}</option>)}
              </select>
            </label>
          </section>
    ) },
    { id: "boot", cat: "general", keys: ["settings.boot", "settings.bootAnimation", "settings.bootSound"] as Key[], el: (
          <section className="card">
            <h2>{t("settings.boot")}</h2>
            <label className="check">
              <input type="checkbox" checked={settings.boot.animation} onChange={(e) => set({ boot: { ...settings.boot, animation: e.target.checked } })} />
              <span>{t("settings.bootAnimation")}</span>
            </label>
            <label className="check">
              <input type="checkbox" checked={settings.boot.sound} disabled={!settings.boot.animation} onChange={(e) => set({ boot: { ...settings.boot, sound: e.target.checked } })} />
              <span>{t("settings.bootSound")}</span>
            </label>
          </section>
    ) },
    { id: "updates", cat: "general", keys: ["settings.updates", "settings.updateSound"] as Key[], el: (
          <section className="card">
            <h2>{t("settings.updates")}</h2>
            <label className="check">
              <input type="checkbox" checked={settings.updateSound} onChange={(e) => set({ updateSound: e.target.checked })} />
              <span>{t("settings.updateSound")}</span>
            </label>
          </section>
    ) },
    { id: "news", cat: "general", keys: ["settings.newsTitle", "settings.newsShow", "settings.newsHint"] as Key[], el: (
          <section className="card">
            <h2>{t("settings.newsTitle")}</h2>
            <label className="check">
              <input type="checkbox" checked={settings.announcementsEnabled} onChange={(e) => set({ announcementsEnabled: e.target.checked })} />
              <span>{t("settings.newsShow")}</span>
            </label>
            <small className="status">{t("settings.newsHint")}</small>
          </section>
    ) },
    { id: "setup", cat: "general", keys: ["settings.setup", "settings.setupBody", "settings.runSetup"] as Key[], el: (
          <section className="card">
            <h2>{t("settings.setup")}</h2>
            <p>{t("settings.setupBody")}</p>
            <div className="actions left"><button type="button" className="ghost" onClick={onSetup}>{t("settings.runSetup")}</button></div>
          </section>
    ) },
    { id: "platform", cat: "game", keys: ["settings.platform"] as Key[], el: (
          <section className="card">
            <h2>{t("settings.platform")}</h2>
            <PlatformFields value={settings} env={env} onChange={(next) => void save(next)} />
          </section>
    ) },
    { id: "paths", cat: "game", keys: ["settings.paths", "settings.gameFolder", "settings.snapshotsRoot"] as Key[], el: (
          <section className="card">
            <h2>{t("settings.paths")}</h2>
            <GameFolderPicker label={t("settings.gameFolder")} hint={t("settings.gameFolderHint")} placeholder={folder} onValid={(c) => c.root && c.name && void save({ ...settings, gameRoot: c.root, gameFolder: c.name })} />
            <PathField label={t("settings.snapshotsRoot")} hint={t("settings.snapshotsRootHint")} folder value={settings.snapshotsRoot ?? ""} placeholder={t("settings.default")} onChange={(v) => set({ snapshotsRoot: v || null })} />
          </section>
    ) },
    { id: "tray", cat: "game", keys: ["settings.tray", "settings.traySyncDefault"] as Key[], el: (
          <section className="card">
            <h2>{t("settings.tray")}</h2>
            <label className="check">
              <input type="checkbox" checked={settings.traySyncDefault} onChange={(e) => set({ traySyncDefault: e.target.checked })} />
              <span>{t("settings.traySyncDefault")}</span>
            </label>
            <small className="status">{t("tray.note")}</small>
          </section>
    ) },
    { id: "shots", cat: "game", keys: ["settings.shots", "settings.qualityBadges"] as Key[], el: (
          <section className="card">
            <h2>{t("settings.shots")}</h2>
            <label className="check">
              <input type="checkbox" checked={settings.shots.qualityBadges} onChange={(e) => set({ shots: { ...settings.shots, qualityBadges: e.target.checked } })} />
              <span>{t("settings.qualityBadges")}</span>
            </label>
          </section>
    ) },
    { id: "appearance", cat: "appearance", keys: ["settings.appearance", "settings.theme", "settings.accent", "settings.animations"] as Key[], el: (
          <section className="card">
            <h2>{t("settings.appearance")}</h2>
            <div className="field">
              <span>{t("settings.theme")}</span>
              <div className="themes">
                {THEME_NAMES.filter((name) => (name === "mica" ? support.mica : name === "acrylic" ? support.acrylic : true)).map((name) => (
                  <button key={name} type="button" className="theme" aria-pressed={theme.preset === name} onClick={() => setTheme({ preset: name })}>
                    <i style={{ background: `linear-gradient(135deg, ${PRESETS[name].vars["--bg"]} 50%, ${PRESETS[name].vars["--accent"]} 50%)` }} />
                    {t(`theme.${name}`)}
                  </button>
                ))}
              </div>
            </div>
            <div className="field">
              <span>{t("settings.accent")}</span>
              <div className="path-row">
                <input type="color" value={theme.accent && isHex(theme.accent) ? theme.accent : PRESETS[theme.preset]?.vars["--accent"] ?? "#a678ff"} onChange={(e) => saveSoon({ ...settings, theme: { ...theme, accent: e.target.value } })} />
                <button type="button" className="ghost" disabled={!theme.accent} onClick={() => setTheme({ accent: null })}>{t("settings.accentAuto")}</button>
              </div>
            </div>
            <label className="check">
              <input type="checkbox" checked={theme.animations} onChange={(e) => setTheme({ animations: e.target.checked })} />
              <span>{t("settings.animations")}</span>
            </label>
            <div className="field">
              <textarea value={themeText} placeholder={t("settings.themeJson")} rows={4} spellCheck={false} onChange={(e) => setThemeText(e.target.value)} />
              <div className="actions left">
                <button type="button" className="ghost" onClick={copyTheme}>{t("settings.exportTheme")}</button>
                <button type="button" className="ghost" disabled={!themeText.trim()} onClick={importTheme}>{t("settings.importTheme")}</button>
              </div>
              {themeMsg && <small className="status">{themeMsg}</small>}
            </div>
          </section>
    ) },
    { id: "uiscale", cat: "appearance", keys: ["uiscale.title", "uiscale.auto", "uiscale.reset"] as Key[], el: (
          <section className="card">
            <h2>{t("uiscale.title")}</h2>
            <label className="check">
              <input type="checkbox" checked={settings.uiAuto} onChange={(e) => set({ uiAuto: e.target.checked })} />
              <span>{t("uiscale.auto")}</span>
            </label>
            <div className="range-row">
              <button type="button" className="ghost small" aria-label="−" disabled={scale <= 70} onClick={() => set({ uiScale: Math.max(70, scale - 5), uiAuto: false })}>−</button>
              <input type="range" min={70} max={130} step={5} value={scale} aria-label={t("uiscale.title")} onChange={(e) => saveSoon({ ...settings, uiScale: Number(e.target.value), uiAuto: false })} />
              <button type="button" className="ghost small" aria-label="+" disabled={scale >= 130} onClick={() => set({ uiScale: Math.min(130, scale + 5), uiAuto: false })}>+</button>
              <b className="mono scale-value">{scale}%</b>
              <button type="button" className="ghost small" disabled={scale === 100 && !settings.uiAuto} onClick={() => set({ uiScale: 100, uiAuto: false })}>{t("uiscale.reset")}</button>
            </div>
          </section>
    ) },
    { id: "presence", cat: "discord", keys: ["settings.presence", "settings.presenceEnable", "settings.presenceHide"] as Key[], el: (
          <section className="card">
            <h2>{t("settings.presence")}</h2>
            <label className="check">
              <input type="checkbox" checked={settings.presence.enabled} onChange={(e) => set({ presence: { ...settings.presence, enabled: e.target.checked } })} />
              <span>{t("settings.presenceEnable")}</span>
            </label>
            <label className="check">
              <input type="checkbox" checked={settings.presence.hideNumbers} disabled={!settings.presence.enabled} onChange={(e) => set({ presence: { ...settings.presence, hideNumbers: e.target.checked } })} />
              <span>{t("settings.presenceHide")}</span>
            </label>
          </section>
    ) },
    { id: "s4mm", cat: "tools", keys: ["settings.s4mm", "settings.s4mmEnable", "settings.s4mmDownload"] as Key[], el: (
          <section className="card">
            <h2>{t("settings.s4mm")}</h2>
            <label className="check">
              <input type="checkbox" checked={s4mm.enabled} onChange={(e) => set({ s4mm: { ...s4mm, enabled: e.target.checked, path: s4mm.path ?? env?.s4mm ?? null } })} />
              <span>{t("settings.s4mmEnable")}</span>
            </label>
            {s4mm.enabled && (
              <>
                <PathField label={t("wizard.s4mmPath")} extensions={["exe"]} value={s4mm.path ?? ""} placeholder={env?.s4mm ?? ""} onChange={(v) => set({ s4mm: { ...s4mm, path: v || null } })} />
                <label className="check">
                  <input type="checkbox" checked={s4mm.launchWithGame} onChange={(e) => set({ s4mm: { ...s4mm, launchWithGame: e.target.checked } })} />
                  <span>{t("main.s4mmWith")}</span>
                </label>
              </>
            )}
            <div className="actions left"><button type="button" className="ghost" onClick={() => openLink(S4MM_URL)}>{t("settings.s4mmDownload")}</button></div>
          </section>
    ) },
    { id: "mp", cat: "tools", keys: ["settings.mp", "settings.mpEnable", "settings.mpDownload"] as Key[], el: (
          <section className="card">
            <h2>{t("settings.mp")}</h2>
            <label className="check">
              <input type="checkbox" checked={multiplayer.enabled} onChange={(e) => set({ multiplayer: { ...multiplayer, enabled: e.target.checked } })} />
              <span>{t("settings.mpEnable")}</span>
            </label>
            {multiplayer.enabled && (
              <PathField label={t("wizard.mpPath")} hint={t("wizard.mpPathHint")} extensions={["exe"]} value={multiplayer.launcher ?? ""} onChange={(v) => set({ multiplayer: { ...multiplayer, launcher: v || null } })} />
            )}
            <div className="actions left"><button type="button" className="ghost" onClick={() => setNotice(true)}>{t("settings.mpDownload")}</button></div>
          </section>
    ) },
    { id: "diag", cat: "diag", keys: ["diag.title"] as Key[], el: (
          <DiagPanel />
    ) },
  ];
  const visible = sections.filter((s) => (q ? s.keys.some((k) => t(k).toLowerCase().includes(q)) : s.cat === cat));

  return (
    <div className={"settings-page" + (leaving ? " leaving" : "")}>
      <header className="settings-head">
        <button type="button" className="ghost" onClick={back}>{t("settings.back")}</button>
        <h1>{t("settings.title")}</h1>
        <label className="search-box">
          <Icon name="search" size={15} />
          <input type="text" value={query} placeholder={t("settings.search")} spellCheck={false} onChange={(e) => setQuery(e.target.value)} />
          {query && <button type="button" className="search-clear" aria-label={t("settings.searchClear")} onClick={() => setQuery("")}><Icon name="close" size={13} /></button>}
        </label>
      </header>
      <div className="settings-layout">
        <nav className="settings-nav" aria-label={t("settings.title")}>
          {CATEGORIES.map((c) => (
            <button key={c} type="button" className={"settings-tab" + (!q && cat === c ? " on" : "")} aria-pressed={!q && cat === c} onClick={() => { setQuery(""); setCat(c); }}>{t(`settings.cat.${c}`)}</button>
          ))}
        </nav>
        <div className="settings" key={q ? "q" : cat}>
          {visible.map((s) => <Fragment key={s.id}>{s.el}</Fragment>)}
          {visible.length === 0 && <p className="note">{t("settings.noResults")}</p>}
        </div>
      </div>
      {notice && (
        <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && setNotice(false)}>
          <div className="dialog" role="dialog" aria-modal="true">
            <div className="dialog-body">
              <h2>{t("notice.title")}</h2>
              <p>{t("notice.body")}</p>
              <small className="status">{t("notice.source")}</small>
              <div className="actions">
                <button type="button" className="ghost" onClick={() => setNotice(false)}>{t("common.cancel")}</button>
                <button type="button" className="play" onClick={() => { void openLink(S4MP_URL); setNotice(false); }}>{t("notice.go")}</button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
