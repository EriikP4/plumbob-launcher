import { useEffect, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { openLink } from "../lib/native";
import { NO_INTRO_URL } from "../lib/nointro";
import InfoTip from "./InfoTip";

export default function NoIntroCard({ code }: { code: string }) {
  const { t, msg } = useSettings();
  const [installed, setInstalled] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    let live = true;
    backend.noIntroStatus(code).then((v) => live && setInstalled(v)).catch(() => live && setInstalled(false));
    return () => {
      live = false;
    };
  }, [code]);

  const toggle = async () => {
    setError("");
    if (installed && !confirm) return setConfirm(true);
    setConfirm(false);
    setBusy(true);
    try {
      if (installed) {
        await backend.removeNoIntro(code);
        setInstalled(false);
      } else {
        await backend.installNoIntro(code);
        setInstalled(true);
      }
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="nointro-card">
      <label className="check">
        <input type="checkbox" checked={!!installed} disabled={busy || installed === null} onChange={() => void toggle()} />
        <span>{t("opts.skipEaIntro")}</span>
        <InfoTip text={t("opts.skipEaIntroInfo")} />
      </label>
      {busy && <small className="status">{t("common.working")}</small>}
      {confirm && (
        <p className="error" role="alert">
          {t("nointro.removeSure")}{" "}
          <button type="button" className="ghost small" onClick={() => void toggle()}>{t("nointro.remove")}</button>{" "}
          <button type="button" className="ghost small" onClick={() => setConfirm(false)}>{t("common.cancel")}</button>
        </p>
      )}
      {error && (
        <div>
          <p className="error" role="alert">{error}</p>
          <div className="actions left">
            <button type="button" className="ghost small" onClick={() => void openLink(NO_INTRO_URL)}>{t("nointro.openPage")}</button>
            <button type="button" className="ghost small" onClick={() => void backend.openFolder(code, "mods").catch(() => undefined)}>{t("nointro.openMods")}</button>
          </div>
        </div>
      )}
    </div>
  );
}
