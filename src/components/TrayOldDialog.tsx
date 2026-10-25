import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import type { Instance, TrayOldInfo } from "../lib/types";

interface Props {
  instance: Instance;
  onClose: () => void;
  onIgnore: () => void;
  onDeleted: () => void;
}

export default function TrayOldDialog({ instance, onClose, onIgnore, onDeleted }: Props) {
  const { t, msg, size } = useSettings();
  const [info, setInfo] = useState<TrayOldInfo | null | undefined>(undefined);
  const [sure, setSure] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    backend.trayOldInfo(instance.id).then(setInfo).catch(() => setInfo(null));
  }, [instance.id]);

  const remove = async () => {
    if (!sure) return setSure(true);
    setBusy(true);
    setError("");
    try {
      await backend.deleteTrayOld(instance.id);
      onDeleted();
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
      setBusy(false);
    }
  };

  return createPortal(
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !busy && onClose()}>
      <div className="dialog" role="dialog" aria-modal="true">
        <div className="dialog-body">
          <h2>{t("trayOld.title", { name: instance.name })}</h2>
          <p>{t("trayOld.body")}</p>
          {info === undefined && <p>{t("common.working")}</p>}
          {info === null && <p>{t("trayOld.gone")}</p>}
          {info && (
            <dl className="summary">
              <dt>{t("trayOld.folder")}</dt><dd className="mono">{info.path}</dd>
              <dt>{t("trayOld.content")}</dt><dd>{t("trayOld.stats", { files: info.files, size: size(info.bytes) })}</dd>
            </dl>
          )}
          <p><small className="status">{t("trayOld.hint")}</small></p>
          {sure && <p className="error" role="alert">{t("trayOld.sure")}</p>}
          {error && <p className="error" role="alert">{error}</p>}
          <div className="actions">
            <button type="button" className="ghost" disabled={busy} onClick={() => backend.openFolder(instance.id, "root").catch(() => undefined)}>{t("trayOld.open")}</button>
            <button type="button" className="ghost" disabled={busy} onClick={onIgnore}>{t("trayOld.ignore")}</button>
            <button type="button" className="ghost danger-outline" disabled={busy || !info} onClick={() => void remove()}>{sure ? t("trayOld.confirmDelete") : t("trayOld.delete")}</button>
            <button type="button" className="ghost" disabled={busy} onClick={onClose}>{t("common.cancel")}</button>
          </div>
        </div>
      </div>
    </div>,
    document.body,
  );
}
