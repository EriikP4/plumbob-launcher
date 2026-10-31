import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { assetUrl } from "../lib/assets";
import type { Bust, Instance } from "../lib/types";

export default function BustPicker({ onClose, onPick, selectedId }: { onClose: () => void; onPick: (path: string, id: string) => void; selectedId?: string | null }) {
  const { t, msg } = useSettings();
  const [instances, setInstances] = useState<Instance[]>([]);
  const [code, setCode] = useState<string | null>(null);
  const [busts, setBusts] = useState<Bust[] | null>(null);
  const [error, setError] = useState("");
  const [progress, setProgress] = useState({ done: 0, total: 0 });

  useEffect(() => {
    backend.listInstances().then((l) => setInstances(l.filter((i) => !i.legacy))).catch(() => undefined);
  }, []);

  useEffect(() => {
    const off = backend.onBustsProgress((p) => setProgress(p));
    return () => void off.then((f) => f());
  }, []);

  useEffect(() => {
    let live = true;
    setBusts(null);
    setProgress({ done: 0, total: 0 });
    setError("");
    backend.listBusts(code).then((l) => live && setBusts(l)).catch((e) => {
      if (!live) return;
      setBusts([]);
      setError(msg(e instanceof Error ? e.message : String(e)));
    });
    return () => { live = false; };
  }, [code, msg]);

  return createPortal(
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="dialog wide" role="dialog" aria-modal="true">
        <div className="dialog-body">
          <h2>{t("bust.title")}</h2>
          {instances.length > 0 && (
            <label className="field">
              <span>{t("bust.from")}</span>
              <select value={code ?? ""} onChange={(e) => setCode(e.target.value || null)}>
                <option value="">{t("bust.active")}</option>
                {instances.map((i) => <option key={i.id} value={i.id}>{i.name}</option>)}
              </select>
            </label>
          )}
          {busts === null && (
            <div className="bust-loading">
              <div className="progress"><i style={{ width: `${progress.total > 0 ? Math.max(4, Math.round((progress.done / progress.total) * 100)) : 4}%` }} /></div>
              <small className="status">{progress.total > 0 ? t("bust.loading", { done: progress.done, total: progress.total }) : t("bust.reading")}</small>
            </div>
          )}
          {busts && busts.length === 0 && <p>{error || t("bust.empty")}</p>}
          {busts && busts.length > 0 && (
            <div className="bust-grid">
              {busts.map((b) => (
                <button key={b.id} type="button" className="bust" aria-pressed={b.id === selectedId} onClick={() => onPick(b.path, b.id)}>
                  <img src={assetUrl(b.path)} alt="" loading="lazy" decoding="async" />
                </button>
              ))}
            </div>
          )}
          <div className="actions">
            <button type="button" className="ghost" onClick={onClose}>{t("common.cancel")}</button>
          </div>
        </div>
      </div>
    </div>,
    document.body,
  );
}
