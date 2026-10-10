import { useCallback, useEffect, useRef, useState } from "react";
import { assetUrl } from "../lib/assets";
import { backend } from "../lib/api";
import { pickImages } from "../lib/native";
import { qualityOf } from "../lib/quality";
import { useSettings } from "../lib/settings";
import type { Instance, Shot, ShotsProgress } from "../lib/types";

const IMPORTED = "~imported";
const url = assetUrl;

function Quality({ width, height }: { width: number; height: number }) {
  const q = qualityOf(width, height);
  if (!q) return null;
  return (
    <span className={"quality" + (q.glow ? " glow" : "")} style={{ ["--q" as string]: q.color }}>{q.label}</span>
  );
}

type Dialog = null | "move" | "add";

export default function ShotsView({ instances, onBack }: { instances: Instance[]; onBack: () => void }) {
  const { t, msg, when, settings } = useSettings();
  const [indexed, setIndexed] = useState<boolean | null>(null);
  const [building, setBuilding] = useState(false);
  const [progress, setProgress] = useState<ShotsProgress>({ done: 0, total: 0 });
  const [filter, setFilter] = useState("");
  const [shots, setShots] = useState<Shot[]>([]);
  const [open, setOpen] = useState<number | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [anchor, setAnchor] = useState<number | null>(null);
  const [dialog, setDialog] = useState<Dialog>(null);
  const [dest, setDest] = useState("");
  const [picked, setPicked] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [toDelete, setToDelete] = useState<Shot[] | null>(null);
  const [error, setError] = useState("");
  const alive = useRef(true);

  const fail = useCallback((e: unknown) => setError(msg(e instanceof Error ? e.message : String(e))), [msg]);

  const load = useCallback(async (instance: string) => {
    setShots(await backend.listShots(instance || null));
  }, []);

  const build = useCallback(async () => {
    setBuilding(true);
    setError("");
    try {
      await backend.shotsIndex();
      if (!alive.current) return;
      setIndexed(true);
      await load(filter);
    } catch (e) {
      fail(e);
    }
    if (alive.current) setBuilding(false);
  }, [filter, load, fail]);

  useEffect(() => {
    alive.current = true;
    const off = backend.onShotsProgress(setProgress);
    backend.shotsStatus().then((s) => {
      setIndexed(s.indexed);
      if (s.indexed) void load("");
    }).catch(fail);
    return () => {
      alive.current = false;
      void off.then((f) => f());
    };
  }, []);

  useEffect(() => {
    if (indexed) void load(filter);
    setSelected(new Set());
    setAnchor(null);
  }, [filter, indexed, load]);

  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => setNotice(""), 4000);
    return () => window.clearTimeout(timer);
  }, [notice]);

  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      const typing = e.target instanceof HTMLElement && ["INPUT", "SELECT", "TEXTAREA"].includes(e.target.tagName);
      if (open !== null) {
        if (e.key === "Escape") setOpen(null);
        if (e.key === "Delete" && shots[open]) setToDelete([shots[open]]);
        if (e.key === "ArrowRight") setOpen((i) => (i === null ? i : Math.min(shots.length - 1, i + 1)));
        if (e.key === "ArrowLeft") setOpen((i) => (i === null ? i : Math.max(0, i - 1)));
        return;
      }
      if (dialog || typing || toDelete) return;
      if (e.key === "Delete") {
        const chosen = shots.filter((s) => selected.has(s.id));
        if (chosen.length) setToDelete(chosen);
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
        e.preventDefault();
        setSelected(new Set(shots.map((s) => s.id)));
      } else if (e.key === "Escape") {
        setSelected(new Set());
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [open, shots, dialog, toDelete, selected]);

  const current = open === null ? null : shots[open];
  const nameOf = (code: string) => (code === IMPORTED ? t("shots.imported") : instances.find((i) => i.id.toLowerCase() === code.toLowerCase())?.name ?? code);
  const allSelected = shots.length > 0 && selected.size === shots.length;

  const toggle = (index: number, range: boolean) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (range && anchor !== null) {
        const [a, b] = [Math.min(anchor, index), Math.max(anchor, index)];
        for (let i = a; i <= b; i += 1) next.add(shots[i].id);
      } else if (next.has(shots[index].id)) {
        next.delete(shots[index].id);
      } else {
        next.add(shots[index].id);
      }
      return next;
    });
    setAnchor(index);
  };

  const selectAll = () => setSelected(allSelected ? new Set() : new Set(shots.map((s) => s.id)));

  const report = (kind: "moved" | "added" | "deleted", done: number, skipped: number) => {
    const text = done === 1 ? t(`shots.${kind}One` as "shots.movedOne") : t(`shots.${kind}` as "shots.moved", { n: done });
    setNotice(text + (skipped > 0 ? t("shots.skipped", { n: skipped }) : ""));
  };

  const refresh = async () => {
    await backend.shotsIndex();
    await load(filter);
  };

  const doDelete = async () => {
    if (!toDelete) return;
    setBusy(true);
    setError("");
    try {
      const result = await backend.deleteShots(toDelete.map((s) => s.path));
      setToDelete(null);
      setOpen(null);
      setSelected(new Set());
      await refresh();
      report("deleted", result.done, result.skipped);
    } catch (e) {
      fail(e);
    }
    setBusy(false);
  };

  const openMove = () => {
    setDest(instances[0]?.id ?? IMPORTED);
    setDialog("move");
  };

  const doMove = async () => {
    setBusy(true);
    setError("");
    try {
      const paths = shots.filter((s) => selected.has(s.id)).map((s) => s.path);
      const result = await backend.moveShots(paths, dest === IMPORTED ? null : dest);
      setDialog(null);
      setSelected(new Set());
      await refresh();
      report("moved", result.done, result.skipped);
    } catch (e) {
      fail(e);
    }
    setBusy(false);
  };

  const openAdd = () => {
    setDest(filter);
    setPicked([]);
    setDialog("add");
  };

  const choosePhotos = async () => {
    const files = await pickImages();
    if (files.length) setPicked(files);
  };

  const doAdd = async () => {
    setBusy(true);
    setError("");
    try {
      const result = await backend.importShots(picked, dest || null);
      setDialog(null);
      await refresh();
      report("added", result.done, result.skipped);
    } catch (e) {
      fail(e);
    }
    setBusy(false);
  };

  const options = (
    <>
      {instances.map((i) => <option key={i.id} value={i.id}>{i.name}</option>)}
    </>
  );

  return (
    <div className="settings shots">
      <header className="settings-head">
        <button type="button" className="ghost" onClick={onBack}>{t("settings.back")}</button>
        <h1>{t("shots.title")}</h1>
        {indexed && !building && (
          <div className="shots-tools">
            <label className="inline-field">
              <span>{t("shots.instance")}</span>
              <select value={filter} onChange={(e) => setFilter(e.target.value)}>
                <option value="">{t("shots.all")}</option>
                <option value={IMPORTED}>{t("shots.imported")}</option>
                {options}
              </select>
            </label>
            <span className="status">{t("shots.count", { n: shots.length })}</span>
            <button type="button" className="ghost" disabled={shots.length === 0} onClick={selectAll}>{allSelected ? t("shots.clearSel") : t("shots.selectAll")}</button>
            <button type="button" className="ghost" onClick={openAdd}>{t("shots.add")}</button>
            <button type="button" className="ghost" onClick={build}>{t("shots.refresh")}</button>
          </div>
        )}
      </header>

      {error && <p className="error" role="alert">{error}</p>}

      {selected.size > 0 && !building && (
        <div className="selection-bar" role="toolbar">
          <b>{selected.size === 1 ? t("shots.selectedOne") : t("shots.selected", { n: selected.size })}</b>
          <span className="selection-actions">
            <button type="button" className="play small" onClick={openMove}>{t("shots.move")}</button>
            <button type="button" className="danger small" onClick={() => setToDelete(shots.filter((s) => selected.has(s.id)))}>{t("shots.delete")}</button>
            <button type="button" className="ghost" onClick={() => setSelected(new Set())}>{t("common.cancel")}</button>
          </span>
        </div>
      )}

      {building && (
        <section className="card">
          <p>{t("shots.indexing", { done: progress.done, total: progress.total })}</p>
          <div className="bar-track"><i style={{ width: `${progress.total ? (progress.done / progress.total) * 100 : 5}%` }} /></div>
        </section>
      )}

      {indexed && !building && shots.length === 0 && <section className="card"><p>{t("shots.empty")}</p></section>}

      {indexed && !building && shots.length > 0 && (
        <div className="shot-grid">
          {shots.map((s, i) => {
            const isSelected = selected.has(s.id);
            return (
              <div
                key={s.id}
                role="button"
                tabIndex={0}
                aria-pressed={isSelected}
                className={"shot" + (isSelected ? " selected" : "") + (selected.size > 0 ? " selecting" : "")}
                onClick={(e) => (selected.size > 0 || e.ctrlKey || e.metaKey || e.shiftKey ? toggle(i, e.shiftKey) : setOpen(i))}
                onKeyDown={(e) => {
                  if (e.key === "Enter") setOpen(i);
                  if (e.key === " ") {
                    e.preventDefault();
                    toggle(i, e.shiftKey);
                  }
                }}
                title={`${nameOf(s.instance)} · ${when(s.modified)}`}
              >
                {s.thumb ? <img src={url(s.thumb)} alt={s.name} loading="lazy" decoding="async" /> : <span className="broken">{s.name}</span>}
                <button type="button" className="shot-check" aria-label={t("shots.select")} aria-pressed={isSelected} onClick={(e) => { e.stopPropagation(); toggle(i, e.shiftKey); }}>
                  <i />
                </button>
                {settings.shots.qualityBadges && <span className="shot-badge"><Quality width={s.width} height={s.height} /></span>}
                <span className="shot-meta"><b>{nameOf(s.instance)}</b><small>{when(s.modified)}</small></span>
              </div>
            );
          })}
        </div>
      )}

      {indexed === false && !building && (
        <div className="overlay">
          <div className="dialog" role="dialog" aria-modal="true">
            <div className="dialog-body">
              <h2>{t("shots.welcomeTitle")}</h2>
              <p>{t("shots.welcomeBody")}</p>
              <div className="actions">
                <button type="button" className="ghost" onClick={onBack}>{t("shots.later")}</button>
                <button type="button" className="play" onClick={build}>{t("shots.accept")}</button>
              </div>
            </div>
          </div>
        </div>
      )}

      {toDelete && (
        <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !busy && setToDelete(null)}>
          <div className="dialog" role="alertdialog" aria-modal="true">
            <div className="dialog-body">
              <h2>{toDelete.length === 1 ? t("shots.deleteTitleOne") : t("shots.deleteTitle", { n: toDelete.length })}</h2>
              <p>{toDelete.length === 1 ? t("shots.deleteBodyOne") : t("shots.deleteBody")}</p>
              <div className="actions">
                <button type="button" className="ghost" disabled={busy} onClick={() => setToDelete(null)}>{t("common.cancel")}</button>
                <button type="button" className="danger" disabled={busy} onClick={doDelete}>{busy ? t("common.working") : t("shots.delete")}</button>
              </div>
            </div>
          </div>
        </div>
      )}

      {dialog === "move" && (
        <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !busy && setDialog(null)}>
          <div className="dialog" role="dialog" aria-modal="true">
            <div className="dialog-body">
              <h2>{selected.size === 1 ? t("shots.moveTitleOne") : t("shots.moveTitle", { n: selected.size })}</h2>
              <p>{t("shots.moveBody")}</p>
              <label className="field">
                <span>{t("shots.dest")}</span>
                <select value={dest} onChange={(e) => setDest(e.target.value)}>
                  {options}
                  <option value={IMPORTED}>{t("shots.imported")}</option>
                </select>
              </label>
              <div className="actions">
                <button type="button" className="ghost" disabled={busy} onClick={() => setDialog(null)}>{t("common.cancel")}</button>
                <button type="button" className="play" disabled={busy} onClick={doMove}>{busy ? t("common.working") : t("shots.moveConfirm")}</button>
              </div>
            </div>
          </div>
        </div>
      )}

      {dialog === "add" && (
        <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !busy && setDialog(null)}>
          <div className="dialog" role="dialog" aria-modal="true">
            <div className="dialog-body">
              <h2>{t("shots.addTitle")}</h2>
              <p>{t("shots.addBody")}</p>
              <label className="field">
                <span>{t("shots.dest")}</span>
                <select value={dest} onChange={(e) => setDest(e.target.value)}>
                  <option value="">{t("shots.noDest")}</option>
                  {options}
                </select>
              </label>
              <div className="path-row">
                <button type="button" className="ghost" disabled={busy} onClick={choosePhotos}>{t("shots.pick")}</button>
                <span className="status">{picked.length === 1 ? t("shots.pickedOne") : picked.length ? t("shots.picked", { n: picked.length }) : ""}</span>
              </div>
              <div className="actions">
                <button type="button" className="ghost" disabled={busy} onClick={() => setDialog(null)}>{t("common.cancel")}</button>
                <button type="button" className="play" disabled={busy || picked.length === 0} onClick={doAdd}>{busy ? t("common.working") : t("shots.addConfirm")}</button>
              </div>
            </div>
          </div>
        </div>
      )}

      {current && (
        <div className="lightbox" onMouseDown={(e) => e.target === e.currentTarget && setOpen(null)} role="dialog" aria-modal="true">
          <img src={url(current.path)} alt={current.name} />
          <div className="lightbox-bar">
            <span className="lightbox-info"><b>{nameOf(current.instance)}</b> · {when(current.modified)} <Quality width={current.width} height={current.height} /></span>
            <span className="lightbox-actions">
              <button type="button" className="ghost" disabled={open === 0} onClick={() => setOpen((i) => Math.max(0, (i ?? 0) - 1))}>{t("shots.previous")}</button>
              <button type="button" className="ghost" disabled={open === shots.length - 1} onClick={() => setOpen((i) => Math.min(shots.length - 1, (i ?? 0) + 1))}>{t("shots.next")}</button>
              <button type="button" className="ghost" onClick={() => backend.revealShot(current.path).catch(() => undefined)}>{t("shots.show")}</button>
              <button type="button" className="danger small" onClick={() => setToDelete([current])}>{t("shots.delete")}</button>
              <button type="button" className="ghost" onClick={() => setOpen(null)}>{t("common.close")}</button>
            </span>
          </div>
        </div>
      )}

      {notice && <div className="toast" role="status" key={notice}>{notice}</div>}
    </div>
  );
}
