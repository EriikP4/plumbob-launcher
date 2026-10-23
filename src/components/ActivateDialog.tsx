import { useState } from "react";
import { createPortal } from "react-dom";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import type { Instance } from "../lib/types";
import InstanceThumb from "./InstanceThumb";

export default function ActivateDialog({ instance, onClose, onDone }: { instance: Instance; onClose: () => void; onDone: () => void }) {
  const { t, msg } = useSettings();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const confirm = async () => {
    setBusy(true);
    setError("");
    try {
      await backend.setActiveInstance(instance.id);
      onDone();
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
      setBusy(false);
    }
  };

  return createPortal(
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !busy && onClose()}>
      <div className="dialog" role="dialog" aria-modal="true">
        <div className="dialog-body">
          <div className="activate-head">
            <InstanceThumb instance={instance} />
            <h2>{t("activate.title", { name: instance.name })}</h2>
          </div>
          <p>{t("activate.body")}</p>
          {error && <p className="error" role="alert">{error}</p>}
          <div className="actions">
            <button type="button" className="ghost" disabled={busy} onClick={onClose}>{t("common.cancel")}</button>
            <button type="button" className="play small-play" disabled={busy} onClick={() => void confirm()} autoFocus>{busy ? t("common.working") : t("activate.confirm")}</button>
          </div>
        </div>
      </div>
    </div>,
    document.body,
  );
}
