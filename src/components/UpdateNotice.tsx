import { useState } from "react";
import { useSettings } from "../lib/settings";
import Icon from "./Icon";

export default function UpdateNotice({ onAccept }: { onAccept: (hide: boolean) => void }) {
  const { t } = useSettings();
  const [hide, setHide] = useState(false);
  const steps = [t("update.step1"), t("update.step2"), t("update.step3"), t("update.step4"), t("update.step5")];
  return (
    <div className="overlay">
      <div className="dialog wide" role="dialog" aria-modal="true">
        <div className="dialog-body">
          <h2 className="notice-title"><Icon name="warn" size={22} /> {t("update.noticeTitle")}</h2>
          <h3 className="notice-sub">{t("update.whatItMeans")}</h3>
          <p>{t("update.noticeBody")}</p>
          <ol className="notice-steps">
            {steps.map((s, i) => <li key={i}>{s}</li>)}
          </ol>
          <label className="check">
            <input type="checkbox" checked={hide} onChange={(e) => setHide(e.target.checked)} />
            <span>{t("update.dontShow")}</span>
          </label>
          <div className="actions">
            <button type="button" className="play" onClick={() => onAccept(hide)}>{t("update.ok")}</button>
          </div>
        </div>
      </div>
    </div>
  );
}
