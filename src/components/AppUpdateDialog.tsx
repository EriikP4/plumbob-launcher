import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { appVersion, openLink } from "../lib/native";
import { renderRich } from "../lib/richtext";
import type { UpdateInfo } from "../lib/types";
import Icon from "./Icon";

interface Props {
  info: UpdateInfo;
  onClose: () => void;
  onIgnore: () => void;
}

export default function AppUpdateDialog({ info, onClose, onIgnore }: Props) {
  const { t, msg, size } = useSettings();
  const [current, setCurrent] = useState("");
  const [method, setMethod] = useState<"installer" | "portable">("portable");
  const [gameOpen, setGameOpen] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState({ done: 0, total: 0 });
  const [error, setError] = useState("");

  useEffect(() => {
    void appVersion().then(setCurrent);
    backend.installMethod().then(setMethod).catch(() => undefined);
    backend.status().then((s) => setGameOpen(s.gameRunning)).catch(() => undefined);
    const off = backend.onUpdateProgress((p) => setProgress(p));
    return () => void off.then((f) => f());
  }, []);

  const direct = method === "installer" && !!info.installer;
  const percent = progress.total > 0 ? Math.min(100, Math.round((progress.done / progress.total) * 100)) : 0;

  const update = async () => {
    setError("");
    if (!direct || !info.installer) {
      void openLink(info.page);
      return onClose();
    }
    if (gameOpen && !confirm) return setConfirm(true);
    setBusy(true);
    setProgress({ done: 0, total: info.installer.size });
    try {
      await backend.installUpdate(info.installer, info.version);
      onClose();
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
      setBusy(false);
    }
  };

  const notes = info.notes.trim();

  return createPortal(
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !busy && onClose()}>
      <div className="dialog wide" role="dialog" aria-modal="true">
        <div className="dialog-body">
          <h2 className="notice-title"><Icon name="download" size={22} /> {t("appupd.title", { version: info.version })}</h2>
          {current && <small className="status">{t("appupd.current", { version: current })}</small>}
          <div className="update-notes">
            {notes ? renderRich(notes) : <p>{t("appupd.noNotes")}</p>}
          </div>
          <div className="actions left">
            <button type="button" className="link-btn" onClick={() => void openLink(info.page)}>{t("appupd.full")}</button>
          </div>
          {!direct && <small className="status">{t(info.installer ? "appupd.portableHint" : "appupd.pageHint")}</small>}
          {confirm && <p className="error" role="alert">{t("appupd.gameOpen")}</p>}
          {busy && (
            <div>
              <div className="progress"><i style={{ width: `${Math.max(3, percent)}%` }} /></div>
              <small className="status">{t("appupd.downloading", { done: size(progress.done), total: size(progress.total) })}</small>
            </div>
          )}
          {error && <p className="error" role="alert">{error}</p>}
          <div className="actions">
            <button type="button" className="ghost" disabled={busy} onClick={onClose}>{t("common.cancel")}</button>
            <button type="button" className="ghost" disabled={busy} onClick={onIgnore}>{t("appupd.ignore")}</button>
            <button type="button" className="play" disabled={busy} onClick={() => void update()}>{confirm ? t("appupd.updateAnyway") : t("appupd.update")}</button>
          </div>
        </div>
      </div>
    </div>,
    document.body,
  );
}
