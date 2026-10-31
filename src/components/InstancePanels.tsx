import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import type { CacheItem, Health, HealthWarning, Instance, SaveInfo, SpaceEntry, Snapshot } from "../lib/types";
import { compareVersions } from "../lib/format";
import Icon from "./Icon";
import CacheDialog from "./CacheDialog";

type Tab = "general" | "space" | "copies";

interface InfoProps {
  instance: Instance;
  snaps: Snapshot[];
  saves: SaveInfo[];
  locked: boolean;
  onSnapshot: () => void;
  onRestore: (s: Snapshot) => void;
  actions?: React.ReactNode;
  onChanged: () => void;
  latestVersion: string | null;
}

export function InfoCard({ instance, snaps, saves, locked, onSnapshot, onRestore, actions, onChanged, latestVersion }: InfoProps) {
  const { t, msg, when, size, settings, save } = useSettings();
  const [cache, setCache] = useState<CacheItem[] | null>(null);
  const [cleaning, setCleaning] = useState(false);
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState(false);
  const [confirm, setConfirm] = useState<string | null>(null);
  const [failed, setFailed] = useState("");
  const [tab, setTab] = useState<Tab>("general");
  const tabs: Tab[] = ["general", "space", "copies"];
  useEffect(() => {
    let live = true;
    setCache(null);
    backend.cacheItems(instance.id).then((c) => live && setCache(c)).catch(() => live && setCache([]));
    return () => { live = false; };
  }, [instance.id]);

  const cacheBytes = (cache ?? []).reduce((n, c) => n + c.bytes, 0);
  const mismatch = !!instance.gameVersion && !!latestVersion && compareVersions(instance.gameVersion, latestVersion) < 0;

  const make = async () => {
    setCreating(true);
    try {
      await onSnapshot();
    } finally {
      setCreating(false);
    }
  };

  const remove = async (id: string) => {
    if (confirm !== id) return setConfirm(id);
    setConfirm(null);
    setFailed("");
    try {
      await backend.deleteSnapshot(instance.id, id);
      onChanged();
    } catch (e) {
      setFailed(String(e));
    }
  };

  return (
    <section className="card tabbed">
      <div className="tabs" role="tablist">
        {tabs.map((k) => (
          <button key={k} type="button" role="tab" aria-selected={tab === k} onClick={() => setTab(k)}>{t(`info.tab.${k}`)}</button>
        ))}
      </div>
      <div className="panes">
      <div className={"pane" + (tab === "general" ? "" : " pane-hidden")} aria-hidden={tab !== "general"}>
      <div className="info-grid" key="general">
          <div>
            <label><Icon name="code" /> {t("info.code")}</label>
            <b>{instance.id}</b>
          </div>
          <div>
            <label><Icon name="tray" /> {t("info.tray")}</label>
            <b className={"tray-state" + (instance.traySynced ? " on" : "")}>
              <Icon name={instance.traySynced ? "link" : "unlink"} />
              {instance.traySynced ? t("info.traySynced") : t("info.trayAlone")}
            </b>
          </div>
          <div>
            <label><Icon name="version" /> {t("info.gameVersion")}</label>
            <b>
              {instance.gameVersion ?? "—"}
              {instance.gameVersion && (
                <span
                  className={"tip left ver-state " + (mismatch ? "warn" : "ok")}
                  tabIndex={0}
                  data-tip={mismatch ? t("info.versionMismatch", { version: latestVersion ?? "" }) : t("info.versionOk")}
                  aria-label={mismatch ? t("info.versionMismatch", { version: latestVersion ?? "" }) : t("info.versionOk")}
                >
                  <Icon name={mismatch ? "warn" : "check"} size={15} />
                </span>
              )}
            </b>
          </div>
          <div>
            <label><Icon name="played" /> {t("info.lastPlayed")}</label>
            <b>{instance.lastPlayedAt ? when(instance.lastPlayedAt) : "—"}</b>
          </div>
          <div>
            <label><Icon name="copies" /> {t("info.lastCopy")}</label>
            <b>{snaps[0] ? when(snaps[0].created) : "—"}</b>
          </div>
          <div>
            <label><Icon name="cache" /> {t("info.cacheSize")}</label>
            <b className="cache-line">
              {cache === null ? "…" : size(cacheBytes)}
              <button className="ghost small danger-outline" type="button" disabled={locked || instance.legacy || !cache || cacheBytes === 0} onClick={() => setCleaning(true)}>{t("info.cleanCache")}</button>
            </b>
          </div>
      </div>
      {actions && <div className="info-foot">{actions}</div>}
      </div>
      {tab === "space" && <SpaceTab code={instance.id} key="space" />}
      {tab === "copies" && (
        <div key="copies">
          <div className="copies-head">
            <label className="check">
              <input type="checkbox" checked={settings.autoSnapshots} onChange={(e) => void save({ ...settings, autoSnapshots: e.target.checked })} />
              <span>{t("info.autoCopies")}</span>
            </label>
            <span className="head-actions">
              <button className="ghost" type="button" disabled={snaps.length === 0 || creating} aria-pressed={editing} onClick={() => { setEditing(!editing); setConfirm(null); }}>{editing ? t("info.done") : t("info.edit")}</button>
              <button className="ghost" type="button" disabled={instance.legacy || locked || creating} onClick={() => void make()}>{creating ? <><Icon name="spinner" size={14} className="spin" /> {t("card.creating")}</> : t("card.createNow")}</button>
            </span>
          </div>
          {failed && <p className="error" role="alert">{failed}</p>}
          <div className="card-list">
            {snaps.length === 0 && <div className="row"><span className="mono">{t("info.noCopies")}</span></div>}
            {snaps.map((s) => (
              <div className="row" key={s.id}>
                <span>{msg(s.label)}</span>
                <span className="row-end">
                  <span className="mono">{when(s.created)}</span>
                  {editing ? (
                    <button className={"del-btn" + (confirm === s.id ? " sure" : "")} type="button" aria-label={t("info.delete")} onClick={() => remove(s.id)}>{confirm === s.id ? t("info.sure") : <Icon name="trash" size={13} />}</button>
                  ) : (
                    <button className="ghost small" type="button" disabled={locked || instance.legacy} onClick={() => onRestore(s)}>{t("card.restore")}</button>
                  )}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
      </div>
      {cleaning && cache && createPortal(
        <CacheDialog
          instance={instance}
          items={cache}
          onClose={() => setCleaning(false)}
          onDone={() => {
            setCleaning(false);
            backend.cacheItems(instance.id).then(setCache).catch(() => undefined);
          }}
        />,
        document.body,
      )}
    </section>
  );
}

function SpaceTab({ code }: { code: string }) {
  const { t, size } = useSettings();
  const [path, setPath] = useState("");
  const [files, setFiles] = useState(false);
  const [list, setList] = useState<SpaceEntry[] | null>(null);

  useEffect(() => {
    let live = true;
    setList(null);
    const load = files ? backend.spaceTopFiles(code, 40) : backend.spaceChildren(code, path);
    load.then((l) => live && setList(l)).catch(() => live && setList([]));
    return () => { live = false; };
  }, [code, path, files]);

  const total = list ? list.reduce((n, e) => n + e.bytes, 0) : 0;
  const max = list && list.length ? list[0].bytes : 0;
  const up = () => setPath(path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "");

  return (
    <div>
      <div className="space-head">
        <div className="seg" role="group">
          <button type="button" aria-pressed={!files} onClick={() => setFiles(false)}>{t("space.folders")}</button>
          <button type="button" aria-pressed={files} onClick={() => setFiles(true)}>{t("space.files")}</button>
        </div>
        {!files && path && (
          <button className="ghost small" type="button" onClick={up}>{t("space.up")} · {path}</button>
        )}
        {list && <span className="mono">{size(total)}</span>}
      </div>
      <div className="card-list space-list">
        {list === null && <div className="row"><span className="mono">{t("common.working")}</span></div>}
        {list && list.length === 0 && <div className="row"><span className="mono">{t("space.empty")}</span></div>}
        {list?.map((e) => (
          <button
            key={e.rel}
            type="button"
            className="space-row"
            disabled={files || !e.isDir}
            data-tip={files ? e.rel : undefined}
            onClick={() => e.isDir && setPath(e.rel)}
          >
            <i style={{ width: `${max ? Math.max(2, (e.bytes / max) * 100) : 0}%` }} />
            <span className="name">{e.isDir ? "▸ " : ""}{e.name}</span>
            <span className="mono">{size(e.bytes)}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

export function HealthCard({ instance, versionWarn, onShowIssues }: { instance: Instance; versionWarn: boolean; onShowIssues: (warnings: HealthWarning[]) => void }) {
  const { t, when } = useSettings();
  const [health, setHealth] = useState<Health | null>(null);

  useEffect(() => {
    let live = true;
    setHealth(null);
    backend.instanceHealth(instance.id).then((h) => live && setHealth(h)).catch(() => undefined);
    return () => { live = false; };
  }, [instance.id]);

  return (
    <section className="card health-card">
      <h2>
        {t("health.title")}
        {versionWarn && (
          <span className="tip right ver-state warn" tabIndex={0} data-tip={t("health.versionWarn")} aria-label={t("health.versionWarn")}>
            <Icon name="warn" size={17} />
          </span>
        )}
      </h2>
      {!health && <div className="row"><span className="mono">{t("common.working")}</span></div>}
      {health && (
        <div className="card-list health-list">
          {health.mods.map((m) => (
            <div className="row" key={m.id}>
              <span>{m.name}</span>
              <span className="row-end">
                {m.installed && m.version && <span className="mono">{m.version}</span>}
                <span className={"pill" + (m.installed ? " ok" : "")}>{m.installed ? t("health.installed") : t("health.missing")}</span>
              </span>
            </div>
          ))}
          {health.warnings.length === 0 && (
            <div className="row"><span>{t("health.clean")}</span><span className="pill ok">OK</span></div>
          )}
          {health.warnings.map((w) => (
            <div className="row" key={w.kind} title={w.items.join("\n")}>
              <span>{t(`health.${w.kind}`)}</span>
              <span className="row-end">
                <button type="button" className="ghost small" onClick={() => onShowIssues(health.warnings)}>{t("health.viewIssues")}</button>
                <span className="pill warn">{w.count}</span>
              </span>
            </div>
          ))}
          <div className="row">
            <span>
              {t("health.lastException")}
              {health.lastException && health.lastExceptionSource && <span className="pill src-pill">{t(`health.src.${health.lastExceptionSource}`)}</span>}
            </span>
            {health.lastException ? (
              <span className="row-end">
                <span className="mono">{when(health.lastException)}</span>
                {health.lastExceptionFile && <button className="ghost small" type="button" onClick={() => void backend.openInstanceFile(instance.id, health.lastExceptionFile!).catch(() => undefined)}>{t("health.open")}</button>}
              </span>
            ) : <span className="pill ok">{t("health.none")}</span>}
          </div>
        </div>
      )}
    </section>
  );
}
