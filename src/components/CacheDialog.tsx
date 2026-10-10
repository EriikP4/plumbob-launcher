import { useEffect, useMemo, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import type { CacheItem, Instance } from "../lib/types";
import Icon from "./Icon";

const FILES: Record<CacheItem["id"], string> = {
  thumbs: "localthumbcache.package",
  textures: "localsimtexturecache.package",
  avatar: "avatarcache.package",
  cache: "cache/",
  cachestr: "cachestr/",
  onlinethumbs: "onlinethumbnailcache/",
  errors: "BE-ExceptionReport.html, mc_lastexception.html, lastException_*.txt…",
  logs: "*.log",
};

const RECOMMENDED: CacheItem["id"][] = ["onlinethumbs", "thumbs", "errors"];

interface Props {
  instance: Instance;
  items: CacheItem[];
  onClose: () => void;
  onDone: () => void;
}

export default function CacheDialog({ instance, items, onClose, onDone }: Props) {
  const { t, msg, size } = useSettings();
  const list = useMemo(() => [...items].sort((a, b) => b.bytes - a.bytes), [items]);
  const ids = list.map((i) => i.id);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [step, setStep] = useState<"pick" | "confirm" | "working" | "done">("pick");
  const [progress, setProgress] = useState({ done: 0, total: 1 });
  const [error, setError] = useState("");
  const [freed, setFreed] = useState(0);

  useEffect(() => {
    const off = backend.onCacheProgress((p) => setProgress({ done: p.done, total: Math.max(1, p.total) }));
    return () => void off.then((f) => f());
  }, []);

  const recommended = RECOMMENDED.filter((id) => ids.includes(id));
  const isRecommended = selected.size === recommended.length && recommended.every((id) => selected.has(id));
  const allSelected = ids.length > 0 && ids.every((id) => selected.has(id));
  const total = list.filter((i) => selected.has(i.id)).reduce((n, i) => n + i.bytes, 0);

  const toggle = (id: string) =>
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const run = async () => {
    setError("");
    setStep("working");
    setProgress({ done: 0, total: 1 });
    try {
      const report = await backend.clearCache(instance.id, [...selected]);
      setFreed(report.freed);
      setStep("done");
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
      setStep("pick");
    }
  };

  const percent = Math.min(100, Math.round((progress.done / progress.total) * 100));

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && step !== "working" && onClose()}>
      <div className="dialog wide cache-dialog" role="dialog" aria-modal="true">
        {step === "pick" && (
          <div className="dialog-body">
            <h2>{t("cache.title")}</h2>
            <div className="cache-tools">
              <button type="button" className="ghost small" disabled={isRecommended} onClick={() => setSelected(new Set(recommended))}>{t("cache.recommended")}</button>
              <button type="button" className="ghost small" disabled={allSelected} onClick={() => setSelected(new Set(ids))}>{t("cache.all")}</button>
              <button type="button" className="ghost small" disabled={selected.size === 0} onClick={() => setSelected(new Set())}>{t("cache.none")}</button>
            </div>
            <div className="cache-list">
              {list.map((item) => (
                <label key={item.id} className={"cache-item" + (selected.has(item.id) ? " on" : "")}>
                  <input type="checkbox" checked={selected.has(item.id)} onChange={() => toggle(item.id)} />
                  <span className="cache-text">
                    <b>{t(`cache.item.${item.id}`)}</b>
                    <small>{FILES[item.id]}</small>
                    {(item.id === "avatar" || item.id === "logs") && (
                      <span className={"cache-warn-wrap" + (selected.has(item.id) ? " open" : "")} aria-hidden={!selected.has(item.id)}>
                        <span>
                          <em className="cache-warn"><Icon name="warn" size={14} /> {t(`cache.warn.${item.id}`)}</em>
                        </span>
                      </span>
                    )}
                  </span>
                  <span className="mono">{size(item.bytes)}</span>
                </label>
              ))}
            </div>
            {error && <p className="error" role="alert">{error}</p>}
            <div className="actions">
              <button type="button" className="ghost" onClick={onClose}>{t("common.cancel")}</button>
              <button type="button" className="play" disabled={selected.size === 0} onClick={() => setStep("confirm")}>{t("cache.delete")}{selected.size > 0 ? ` · ${size(total)}` : ""}</button>
            </div>
          </div>
        )}
        {step === "confirm" && (
          <div className="dialog-body">
            <h2>{t("cache.confirmTitle")}</h2>
            <p>{t("cache.confirmBody", { size: size(total) })}</p>
            <small className="status">{t("cache.permanent")}</small>
            <div className="actions">
              <button type="button" className="ghost" onClick={() => setStep("pick")}>{t("common.back")}</button>
              <button type="button" className="play" onClick={run}>{t("cache.continue")}</button>
            </div>
          </div>
        )}
        {step === "working" && (
          <div className="dialog-body">
            <h2>{t("cache.working")}</h2>
            <div className="progress" role="progressbar" aria-valuenow={percent} aria-valuemin={0} aria-valuemax={100}><i style={{ width: `${percent}%` }} /></div>
            <small className="status">{percent}%</small>
          </div>
        )}
        {step === "done" && (
          <div className="dialog-body">
            <h2>{t("cache.doneTitle")}</h2>
            <p>{t("cache.doneBody", { size: size(freed) })}</p>
            <div className="actions">
              <button type="button" className="play" onClick={onDone}>{t("common.close")}</button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
