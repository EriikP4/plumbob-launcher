import { useCallback, useEffect, useRef, useState } from "react";
import { backend } from "./lib/api";
import { useSettings } from "./lib/settings";
import type { Instance, LaunchMode, SaveInfo, Snapshot, Status } from "./lib/types";
import Rail from "./components/Rail";
import HeroArt from "./components/HeroArt";
import PlayBar from "./components/PlayBar";
import SettingsView from "./components/SettingsView";
import ShotsView from "./components/ShotsView";
import { EditDialog, RestoreDialog } from "./components/InstanceDialogs";
import { compareVersions, newestVersion } from "./lib/format";
import { assetUrl } from "./lib/assets";
import { HealthCard, InfoCard } from "./components/InstancePanels";
import NotesDialog from "./components/NotesView";
import MusicDialog from "./components/MusicView";
import UpdateNotice from "./components/UpdateNotice";
import HashDialog from "./components/HashView";
import Icon from "./components/Icon";
import S4mmButton from "./components/S4mmButton";
import GameFolderDialog from "./components/GameFolderDialog";
import { burstAt } from "./lib/checkfx";
import Wizard from "./components/Wizard";
import ArchivedView from "./components/ArchivedView";
import { AddDialog, SetupDialog } from "./components/Dialogs";

const pill = (ok: boolean, text: string) => <span className={"pill " + (ok ? "ok" : "warn")}>{text}</span>;

export default function App() {
  const { settings, save, t, msg, size, when, duration } = useSettings();
  const [status, setStatus] = useState<Status | null>(null);
  const [instances, setInstances] = useState<Instance[]>([]);
  const [snaps, setSnaps] = useState<Snapshot[]>([]);
  const [saves, setSaves] = useState<SaveInfo[]>([]);
  const [curId, setCurId] = useState<string | null>(null);
  const [playing, setPlaying] = useState(false);
  const [started, setStarted] = useState(false);
  const cancelled = useRef(false);
  const [mode, setMode] = useState<LaunchMode>("offline");
  const [adding, setAdding] = useState(false);
  const [preset, setPreset] = useState<Instance | undefined>();
  const [view, setView] = useState<"main" | "settings" | "shots" | "archived">("main");
  const [editing, setEditing] = useState(false);
  const [notes, setNotes] = useState(false);
  const [music, setMusic] = useState(false);
  const [hashing, setHashing] = useState(false);
  const [updateNotice, setUpdateNotice] = useState(false);
  const [gameVer, setGameVer] = useState<string | null>(null);
  const [restoring, setRestoring] = useState<Snapshot | null>(null);
  const [notice, setNotice] = useState("");
  const [rerun, setRerun] = useState(false);
  const [error, setError] = useState("");
  const [tick, setTick] = useState(0);

  const fail = useCallback((e: unknown) => setError(msg(e instanceof Error ? e.message : String(e))), [msg]);

  const reload = useCallback(async (select?: string) => {
    const [s, l] = await Promise.all([backend.status(), backend.listInstances()]);
    setStatus(s);
    setInstances(l);
    setCurId((prev) => select ?? (l.some((i) => i.id === prev) ? prev : (l.find((i) => i.id === s.activeId) ?? l[0])?.id ?? null));
  }, []);

  const setupDone = settings.setupDone;
  const gameRoot = settings.gameRoot;
  const gameFolder = settings.gameFolder;
  const snapshotsRoot = settings.snapshotsRoot;

  useEffect(() => {
    if (!setupDone) return;
    reload()
      .then(() => backend.status())
      .then((s) => {
        if (s.gameRunning && s.activeId) {
          setPlaying(true);
          setStarted(true);
          return backend.watchGame();
        }
      })
      .catch(fail);
  }, [reload, fail, setupDone, gameRoot, gameFolder, snapshotsRoot]);

  useEffect(() => {
    backend.gameVersion().then(setGameVer).catch(() => setGameVer(null));
  }, [settings.installDir, settings.platform]);

  useEffect(() => {
    const off = backend.onSessionEnd((e) => {
      setPlaying(false);
      setStarted(false);
      if (cancelled.current) cancelled.current = false;
      else if (!e.started) setError(t("main.gameNotOpened"));
      else if (e.error) setError(msg(e.error));
      setTick((n) => n + 1);
      reload().catch(() => undefined);
    });
    return () => void off.then((f) => f());
  }, [reload, t, msg]);

  const cur = instances.find((i) => i.id === curId) ?? null;
  const curKind = cur?.kind;
  const multiplayerOn = settings.multiplayer.enabled;

  useEffect(() => {
    setMode(curKind === "mp" && multiplayerOn ? "s4mp" : "offline");
  }, [curId, curKind, multiplayerOn]);

  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => setNotice(""), 4000);
    return () => window.clearTimeout(timer);
  }, [notice]);

  useEffect(() => {
    if (!cur) return;
    backend.listSnapshots(cur.id).then(setSnaps).catch(() => undefined);
    backend.listSaves(cur.id).then(setSaves).catch(() => setSaves([]));
  }, [cur?.id, tick]);

  if (!setupDone || rerun) {
    return (
      <Wizard
        onDone={() => { setRerun(false); setView("main"); }}
        onCancel={setupDone ? () => setRerun(false) : undefined}
      />
    );
  }

  if (!status) return error ? <p className="note">{error}</p> : null;

  const reorder = (ids: string[]) => {
    setInstances((prev) => ids.map((id) => prev.find((i) => i.id === id)).filter((i): i is Instance => !!i));
    backend.saveOrder(ids).catch(fail);
  };

  const dialogs = (
    <>
      {status.gameFolderChoose && <GameFolderDialog onDone={() => reload()} />}
      {!status.gameFolderChoose && status.needsSetup && <SetupDialog onDone={() => reload()} />}
      {hashing && cur && <HashDialog instance={cur} onClose={() => setHashing(false)} />}
      {music && cur && <MusicDialog instance={cur} onClose={() => setMusic(false)} />}
      {updateNotice && cur && <UpdateNotice onAccept={(hide) => { setUpdateNotice(false); if (hide) void save({ ...settings, hideUpdateNotice: true }); void start(); }} />}
      {notes && cur && <NotesDialog instance={cur} onClose={() => setNotes(false)} />}
      {editing && cur && <EditDialog instance={cur} active={status.activeId === cur.id} onClose={() => setEditing(false)} onDone={() => { setEditing(false); reload(); }} />}
      {restoring && cur && <RestoreDialog instance={cur} snapshot={restoring} onClose={() => setRestoring(null)} onDone={() => { setRestoring(null); setTick((n) => n + 1); reload(); }} />}
      {adding && <AddDialog instances={instances} preset={preset} onClose={() => setAdding(false)} onDone={() => { setAdding(false); reload(); }} />}
    </>
  );

  const openAdd = () => { setPreset(undefined); setAdding(true); };

  if (view === "settings") {
    return <div className="app single"><SettingsView onBack={() => setView("main")} onSetup={() => setRerun(true)} /></div>;
  }

  if (view === "archived") {
    return <div className="app"><ArchivedView onBack={() => setView("main")} onRestored={(id) => { setView("main"); void reload(id); }} /></div>;
  }

  if (view === "shots") {
    return <div className="app single"><ShotsView instances={instances} onBack={() => setView("main")} /></div>;
  }

  if (!cur) {
    return (
      <div className="app">
        <Rail instances={[]} activeId={null} selectedId="" locked={false} onSelect={() => undefined} onReorder={() => undefined} onAdd={openAdd} onSettings={() => setView("settings")} onShots={() => setView("shots")} onArchived={() => setView("archived")} />
        <div className="content"><p className="note">{t("main.empty")}</p></div>
        {dialogs}
      </div>
    );
  }

  const select = (i: Instance) => setCurId(i.id);
  const start = async () => {
    if (playing) return;
    setError("");
    if (cur.legacy) { setPreset(cur); return setAdding(true); }
    try {
      cancelled.current = false;
      setStarted(false);
      await backend.play(cur.id, mode);
      await reload(cur.id);
      setPlaying(true);
    } catch (e) {
      fail(e);
    }
  };
  const toggle = () => {
    if (playing) return;
    if (mismatch && !settings.hideUpdateNotice && !cur.legacy) return setUpdateNotice(true);
    void start();
  };
  const stats: [string, string | number, boolean | undefined, string][] = [
    [t("stat.mods"), cur.modCount, undefined, "mods"], [t("stat.saves"), cur.saveCount, undefined, "saves"], [t("stat.tray"), cur.trayCount, cur.traySynced, "tray"],
    [t("stat.size"), size(cur.sizeBytes), undefined, "size"], [t("stat.playtime"), duration(cur.playSeconds), undefined, "time"],
  ];
  const newest = newestVersion(instances.map((i) => i.gameVersion));
  const latest = gameVer ?? newest;
  const mismatch = !!cur.gameVersion && !!latest && compareVersions(cur.gameVersion, latest) < 0;
  const outdated = mismatch;
  const snapshotNow = async () => {
    setError("");
    try {
      await backend.createSnapshot(cur.id);
      setNotice(t("update.done"));
      setTick((n) => n + 1);
    } catch (e) {
      fail(e);
    }
  };
  const multiplayer = settings.multiplayer.enabled;
  const tag = [multiplayer ? (cur.kind === "solo" ? t("kind.solo") : t("kind.mp")) : "", cur.kind === "mp" && cur.role ? t(`role.${cur.role}`) : "", status.activeId === cur.id ? t("main.active") : ""].filter(Boolean).join(" · ");

  return (
    <div className="app">
      <Rail instances={instances} activeId={status.activeId} selectedId={cur.id} locked={playing} onSelect={select} onAdd={openAdd} onReorder={reorder} onSettings={() => setView("settings")} onShots={() => setView("shots")} onArchived={() => setView("archived")} />
      <div className="content">
        <main className="scroll" key={cur.id}>
          <section className="hero">
            <HeroArt colors={cur.colors} />
            <div className="shade" />
            <div className="hero-actions">
              {multiplayerOn && cur.kind === "mp" && <button className="icon-btn tip below right" type="button" data-tip={t("main.hash")} aria-label={t("main.hash")} disabled={cur.legacy} onClick={() => setHashing(true)}><Icon name="shield" size={18} /></button>}
              <button className="icon-btn tip below right" type="button" data-tip={t("main.notes")} aria-label={t("main.notes")} disabled={cur.legacy} onClick={() => setNotes(true)}><Icon name="notes" size={18} /></button>
              <button className="icon-btn tip below right" type="button" data-tip={t("main.music")} aria-label={t("main.music")} disabled={cur.legacy} onClick={() => setMusic(true)}><Icon name="note" size={18} /></button>
              <button className="icon-btn tip below right" type="button" data-tip={t("main.edit")} aria-label={t("main.edit")} disabled={cur.legacy || playing} onClick={() => setEditing(true)}><Icon name="edit" size={18} /></button>
            </div>
            <div className="in">
              {tag && <span className="tag">{tag}</span>}
              <h1>{cur.name}</h1>
              <p>{cur.description || (cur.legacy ? t("main.legacyDescription") : "")}</p>
            </div>
          </section>

          {outdated && (
            <div className="banner warn" role="alert">
              <span>{t("update.warn", { old: cur.gameVersion ?? "", new: latest ?? "" })}</span>
              <button type="button" className="ghost" disabled={cur.legacy} onClick={snapshotNow}>{t("update.snapshot")}</button>
            </div>
          )}

          <div className="stats">
            {stats.map(([k, v, linked, icon]) => (
              <div className="stat" key={k}>
                <label>
                  <Icon name={icon} size={13} /> {k}
                  {linked && (
                    <span className="tip left link-icon" tabIndex={0} data-tip={t("tray.linkedTip")} aria-label={t("tray.linkedTip")}>
                      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M10 13a5 5 0 0 0 7.07 0l3-3a5 5 0 0 0-7.07-7.07l-1.5 1.5" /><path d="M14 11a5 5 0 0 0-7.07 0l-3 3a5 5 0 0 0 7.07 7.07l1.5-1.5" /></svg>
                    </span>
                  )}
                </label>
                <b>{v}</b>
              </div>
            ))}
          </div>

          <div className="cols">
            <InfoCard instance={cur} snaps={snaps} saves={saves} locked={playing} onSnapshot={snapshotNow} onRestore={setRestoring} onChanged={() => setTick((n) => n + 1)} latestVersion={latest} actions={
              <>
                {settings.s4mm.enabled && <S4mmButton code={cur.id} legacy={cur.legacy} disabled={playing} onError={fail} />}
                <button className="ghost" type="button" onClick={(e) => { burstAt(e.currentTarget); backend.openFolder(cur.id, "root").catch(fail); }}>{t("card.openFolder")}</button>
              </>
            } />
            <HealthCard instance={cur} versionWarn={mismatch} />
            <section className="card saves-card">
              <h2>{t("card.saves")}</h2>
              <div className="card-list">
                {saves.length === 0 && <div className="row"><span className="mono">{t("saves.empty")}</span></div>}
                {saves.map((s, i) => (
                  <div className="row" key={s.slot}>
                    <span className="save-info">
                      {s.thumb ? <img className="save-thumb" src={assetUrl(s.thumb)} alt="" loading="lazy" decoding="async" /> : <span className="save-thumb empty" />}
                      <span>
                        {t("saves.slot", { n: s.slot })}
                        {i === 0 && <> {pill(true, t("saves.latest"))}</>}
                        <small className="status block">{size(s.sizeBytes)} · {t("saves.versions", { n: s.versions })}</small>
                      </span>
                    </span>
                    <span className="mono">{when(s.modified)}</span>
                  </div>
                ))}
              </div>
            </section>
          </div>
          {error && <p className="error" role="alert">{error}</p>}
          <p className="note">{backend.isSample ? t("main.noteSample") : t("main.noteLive")}</p>
        </main>
        {cur.legacy ? (
          <div className="bar legacy-bar">
            <div className="status">
              <b>{cur.name}</b>
              <span className="mono">{t("legacy.bar")}</span>
            </div>
            <button className="play" type="button" onClick={() => { setPreset(cur); setAdding(true); }}>{t("legacy.barBtn")}</button>
          </div>
        ) : (
          <PlayBar instance={cur} playing={playing} waiting={playing && !started} onCancel={() => { cancelled.current = true; void backend.cancelWatch(); }} mode={mode} onMode={setMode} onToggle={toggle} warn={mismatch} />
        )}
      </div>
      {dialogs}
      {notice && <div className="toast" role="status" key={notice}>{notice}</div>}
    </div>
  );
}
