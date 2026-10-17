import { useSettings } from "../lib/settings";
import type { HealthWarning } from "../lib/types";
import Icon from "./Icon";

const SHOWN = 5;

export default function ModWarnDialog({ warnings, canFix, onFix, onIgnore, onCancel }: { warnings: HealthWarning[]; canFix: boolean; onFix: () => void; onIgnore: () => void; onCancel: () => void }) {
  const { t } = useSettings();
  return (
    <div className="overlay">
      <div className="dialog wide" role="dialog" aria-modal="true">
        <div className="dialog-body">
          <h2 className="notice-title"><Icon name="warn" size={22} /> {t("modwarn.title")}</h2>
          <p>{t("modwarn.body")}</p>
          <div className="modwarn-list">
            {warnings.map((w) => (
              <section key={w.kind} className="modwarn-item">
                <h3>{t(`health.${w.kind}`)} <span className="pill warn">{w.count}</span></h3>
                <p>{t(`modwarn.why.${w.kind}`)}</p>
                <p className="modwarn-fix">{t(`modwarn.how.${w.kind}`)}</p>
                <ul className="modwarn-files">
                  {w.items.slice(0, SHOWN).map((f) => <li key={f}>{f}</li>)}
                  {w.count > Math.min(w.items.length, SHOWN) && <li className="more">{t("modwarn.more", { n: w.count - Math.min(w.items.length, SHOWN) })}</li>}
                </ul>
              </section>
            ))}
          </div>
          <div className="actions wrap">
            <button type="button" className="ghost" onClick={onCancel}>{t("modwarn.cancel")}</button>
            <button type="button" className="ghost ignore" onClick={onIgnore}>{t("modwarn.ignore")}</button>
            {canFix && <button type="button" className="fix" onClick={onFix}>{t("modwarn.fix")}</button>}
          </div>
        </div>
      </div>
    </div>
  );
}
