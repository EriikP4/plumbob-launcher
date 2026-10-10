import { useEffect, useRef, useState } from "react";
import { backend } from "../lib/api";
import { playAlarm } from "../lib/chime";
import { useSettings } from "../lib/settings";
import type { DeleteStats, Instance, ShotsProgress } from "../lib/types";

type Step = "ask" | "final" | "progress" | "done";

function AlarmLights({ level }: { level: 1 | 2 }) {
  return (
    <div className={"alarm-lights l" + level} aria-hidden="true">
      <div className="al-vig" />
      <i className="al-beacon b1" />
      <i className="al-beacon b2" />
      <i className="al-beacon b3" />
      <i className="al-beacon b4" />
      {[1, 2, 3, 4, 5, 6].map((n) => <i key={n} className={"al-light i" + n} />)}
    </div>
  );
}

export default function DeleteFlow({ instance, onCancel, onDone }: { instance: Instance; onCancel: () => void; onDone: () => void }) {
  const { t, msg, size, duration } = useSettings();
  const [step, setStep] = useState<Step>("ask");
  const [progress, setProgress] = useState<ShotsProgress>({ done: 0, total: 1 });
  const [stats, setStats] = useState<DeleteStats | null>(null);
  const [error, setError] = useState("");
  const stopSound = useRef<() => void>(() => undefined);

  const alarm = step === "ask" ? 1 : step === "final" || step === "progress" ? 2 : 0;

  useEffect(() => {
    const root = document.documentElement;
    if (alarm) root.dataset.alarm = String(alarm);
    else delete root.dataset.alarm;
    return () => {
      delete root.dataset.alarm;
    };
  }, [alarm]);

  useEffect(() => {
    stopSound.current();
    if (step === "ask") stopSound.current = playAlarm(1);
    else if (step === "final") stopSound.current = playAlarm(2);
    return () => stopSound.current();
  }, [step]);

  const run = async () => {
    setError("");
    setProgress({ done: 0, total: 1 });
    setStep("progress");
    const off = await backend.onDeleteProgress(setProgress);
    try {
      setStats(await backend.deleteInstance(instance.id));
      setStep("done");
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
      setStep("final");
    } finally {
      off();
    }
  };

  const pct = Math.min(100, Math.round((progress.done / Math.max(1, progress.total)) * 100));

  return (
    <>
      {alarm > 0 && <AlarmLights level={alarm as 1 | 2} />}
      <div className={"overlay" + (alarm ? " alarm-overlay" : "")}>
        {step === "ask" && (
          <div className="dialog alarm l1" role="alertdialog" aria-modal="true">
            <div className="dialog-body">
              <h2>{t("del.title", { name: instance.name })}</h2>
              <p>{t("del.body")}</p>
              <p><b>{t("del.ask")}</b></p>
              <div className="actions">
                <button type="button" className="ghost" onClick={onCancel}>{t("del.no")}</button>
                <button type="button" className="danger" onClick={() => setStep("final")}>{t("del.yes")}</button>
              </div>
            </div>
          </div>
        )}

        {step === "final" && (
          <div className="dialog alarm l2" role="alertdialog" aria-modal="true">
            <div className="dialog-body">
              <h2>{t("del.finalTitle")}</h2>
              <p><b>{t("del.finalBody")}</b></p>
              <small className="status">{t("del.disclaimer")}</small>
              {error && <p className="error" role="alert">{error}</p>}
              <div className="actions wrap">
                <button type="button" className="play" onClick={onCancel}>{t("del.stop")}</button>
                <button type="button" className="danger" onClick={run}>{t("del.confirm")}</button>
              </div>
            </div>
          </div>
        )}

        {step === "progress" && (
          <div className="dialog alarm l2" role="alertdialog" aria-modal="true">
            <div className="dialog-body">
              <h2>{t("del.progress", { name: instance.name, pct })}</h2>
              <div className="bar-track red-bar"><i style={{ width: `${pct}%` }} /></div>
            </div>
          </div>
        )}

        {step === "done" && stats && (
          <div className="dialog" role="dialog" aria-modal="true">
            <div className="dialog-body">
              <h2>{t("del.doneTitle", { name: stats.name })}</h2>
              <ul className="del-stats">
                <li>{t("del.freed", { size: size(stats.bytes) })}</li>
                <li>{t("del.hours", { time: duration(stats.playSeconds) })}</li>
                <li>{t("del.mods", { n: stats.mods })}</li>
                <li>{t("del.saves", { n: stats.saves })}</li>
              </ul>
              {stats.keptSnapshots > 0 && <small className="status">{t("del.kept", { n: stats.keptSnapshots })}</small>}
              <div className="actions">
                <button type="button" className="play" onClick={onDone}>{t("del.continue")}</button>
              </div>
            </div>
          </div>
        )}
      </div>
    </>
  );
}
