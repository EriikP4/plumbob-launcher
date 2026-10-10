import { useCallback, useEffect, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import type { ArchivedInstance } from "../lib/types";
import HeroArt from "./HeroArt";
import Icon from "./Icon";

interface Props {
  onBack: () => void;
  onRestored: (id: string) => void;
}

export default function ArchivedView({ onBack, onRestored }: Props) {
  const { t, msg, size, when, duration } = useSettings();
  const [items, setItems] = useState<ArchivedInstance[] | null>(null);
  const [curId, setCurId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState({ done: 0, total: 1 });
  const [error, setError] = useState("");

  const load = useCallback(async () => {
    const list = await backend.listArchived();
    setItems(list);
    setCurId((prev) => (list.some((i) => i.id === prev) ? prev : list[0]?.id ?? null));
  }, []);

  useEffect(() => {
    load().catch((e) => {
      setItems([]);
      setError(msg(e instanceof Error ? e.message : String(e)));
    });
  }, [load, msg]);

  useEffect(() => {
    const off = backend.onArchiveProgress((p) => setProgress({ done: p.done, total: Math.max(1, p.total) }));
    return () => void off.then((f) => f());
  }, []);

  const cur = items?.find((i) => i.id === curId) ?? null;

  const restore = async () => {
    if (!cur || busy) return;
    setBusy(true);
    setError("");
    setProgress({ done: 0, total: 1 });
    try {
      await backend.restoreArchived(cur.id);
      onRestored(cur.id);
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
      setBusy(false);
    }
  };

  const stats: [string, string | number, string][] = cur
    ? [
        [t("stat.mods"), cur.modCount, "mods"],
        [t("stat.saves"), cur.saveCount, "saves"],
        [t("stat.tray"), cur.trayCount, "tray"],
        [t("archived.original"), size(cur.sizeBytes), "size"],
        [t("stat.playtime"), duration(cur.playSeconds), "time"],
      ]
    : [];

  return (
    <>
      <aside className="rail" aria-label={t("archived.title")}>
        <div className="brand">
          <span className="plumbwrap"><i className="plumb" /></span>
          <div>Plumbob Launcher<small>{t("archived.title")}</small></div>
        </div>
        <div className="list" role="listbox">
          {(items ?? []).map((d) => (
            <button key={d.id} type="button" role="option" aria-selected={d.id === curId} className="item" disabled={busy && d.id !== curId} onClick={() => setCurId(d.id)}>
              <span className="thumb" style={{ background: `linear-gradient(135deg,${d.colors[0]},${d.colors[1]})` }}>{d.name[0]}</span>
              <span className="txt"><b>{d.name}</b><span className="sub">{d.modCount} mods · {d.compressed ? t("archived.compressed") : t("archived.plain")}</span></span>
            </button>
          ))}
        </div>
        <div className="rail-actions">
          <button className="ghost" type="button" disabled={busy} onClick={onBack}>{t("archived.back")}</button>
        </div>
      </aside>
      <div className="content">
        <main className="scroll" key={cur?.id ?? "none"}>
          {items && !cur && <p className="note">{t("archived.empty")}</p>}
          {cur && (
            <>
              <section className="hero">
                <HeroArt colors={cur.colors} />
                <div className="shade" />
                <div className="in">
                  <span className="tag">{cur.compressed ? t("archived.compressed") : t("archived.plain")}</span>
                  <h1>{cur.name}</h1>
                  <p>{cur.description}</p>
                </div>
              </section>
              <div className="stats">
                {stats.map(([k, v, icon]) => (
                  <div className="stat" key={k}>
                    <label><Icon name={icon} size={13} /> {k}</label>
                    <b>{v}</b>
                  </div>
                ))}
              </div>
              <section className="card">
                <dl className="summary">
                  <dt>{t("archived.when", { when: when(cur.archivedAt) })}</dt>
                  <dd>{t("archived.stored", { size: size(cur.archiveBytes) })}</dd>
                  {cur.gameVersion && <><dt>{t("archived.version")}</dt><dd>{cur.gameVersion}</dd></>}
                  {cur.topMods.length > 0 && <><dt>{t("archived.mods")}</dt><dd>{cur.topMods.join(", ")}</dd></>}
                </dl>
              </section>
            </>
          )}
          {error && <p className="error" role="alert">{error}</p>}
        </main>
        {cur && (
          <div className="bar legacy-bar">
            <div className="status">
              <b>{cur.name}</b>
              <span className="mono">{busy ? `${t("archived.restoring")} ${Math.round((progress.done / progress.total) * 100)}%` : t("archived.noPlay")}</span>
            </div>
            <button className="play" type="button" disabled={busy} onClick={restore}>{busy ? t("archived.restoring") : t("archived.restore")}</button>
          </div>
        )}
      </div>
    </>
  );
}
