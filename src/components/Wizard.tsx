import { useEffect, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { LANGUAGES } from "../i18n";
import type { Environment, Settings } from "../lib/types";
import PathField from "./PathField";
import PlatformFields from "./PlatformFields";
import Backdrop from "./Backdrop";

type Step = "lang" | "welcome" | "platform" | "s4mm" | "mp" | "tray" | "summary";
const STEPS: Step[] = ["lang", "welcome", "platform", "s4mm", "mp", "tray", "summary"];

interface Props {
  onDone: () => void;
  onCancel?: () => void;
}

export default function Wizard({ onDone, onCancel }: Props) {
  const { settings, save, t } = useSettings();
  const [draft, setDraft] = useState<Settings>(settings);
  const [env, setEnv] = useState<Environment | null>(null);
  const [index, setIndex] = useState(0);
  const [error, setError] = useState("");
  const step = STEPS[index];

  useEffect(() => {
    backend.detectEnvironment().then(setEnv).catch(() => undefined);
  }, []);

  const pickLanguage = (code: string) => {
    setDraft((d) => ({ ...d, language: code }));
    void save({ ...settings, language: code });
  };

  const setS4mm = (enabled: boolean) =>
    setDraft((d) => ({ ...d, s4mm: { ...d.s4mm, enabled, path: enabled ? d.s4mm.path ?? env?.s4mm ?? null : d.s4mm.path } }));

  const ready = (() => {
    if (step === "platform") {
      if (draft.platform === "custom") return !!draft.installDir?.trim();
      if (draft.platform === "ea" || draft.platform === "epic") return !!draft.installDir?.trim();
    }
    if (step === "s4mm" && draft.s4mm.enabled) return !!draft.s4mm.path?.trim();
    if (step === "mp" && draft.multiplayer.enabled) return !!draft.multiplayer.launcher?.trim();
    return true;
  })();

  const back = () => (index === 0 ? onCancel?.() : setIndex(index - 1));
  const next = () => setIndex(index + 1);

  const finish = async () => {
    setError("");
    try {
      await save({ ...draft, setupDone: true });
      onDone();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const yesNo = (value: boolean, set: (v: boolean) => void) => (
    <div className="seg" role="group">
      <button type="button" aria-pressed={value} onClick={() => set(true)}>{t("common.yes")}</button>
      <button type="button" aria-pressed={!value} onClick={() => set(false)}>{t("common.no")}</button>
    </div>
  );

  const platformName = t(`wizard.${draft.platform}`);
  const onOff = (v: boolean) => (v ? t("wizard.enabled") : t("wizard.disabled"));

  return (
    <div className="overlay wizard">
      <Backdrop />
      <div className="dialog wide" role="dialog" aria-modal="true">
        <div className="dialog-body" key={step}>
          {step !== "lang" && (
            <div className="dots" aria-hidden="true">
              {STEPS.slice(1).map((s, i) => <i key={s} className={i + 1 <= index ? "on" : ""} />)}
            </div>
          )}

          {step === "lang" && (
            <>
              <h2>{t("wizard.langTitle")}</h2>
              <small className="status">{t("wizard.langHint")}</small>
              <div className="choices">
                {LANGUAGES.map((l) => (
                  <button key={l.code} type="button" className="choice" aria-pressed={draft.language === l.code} onClick={() => pickLanguage(l.code)}>
                    <b>{l.name}</b>
                  </button>
                ))}
              </div>
            </>
          )}

          {step === "welcome" && (
            <>
              <h2>{t("wizard.welcomeTitle")}</h2>
              <p>{t("wizard.welcomeBody")}</p>
            </>
          )}

          {step === "platform" && (
            <>
              <h2>{t("wizard.platformTitle")}</h2>
              <p>{t("wizard.platformBody")}</p>
              <PlatformFields value={draft} env={env} onChange={setDraft} />
            </>
          )}

          {step === "s4mm" && (
            <>
              <h2>{t("wizard.s4mmTitle")}</h2>
              <p>{t("wizard.s4mmBody")}</p>
              {yesNo(draft.s4mm.enabled, setS4mm)}
              {draft.s4mm.enabled && (
                <>
                  <PathField label={t("wizard.s4mmPath")} extensions={["exe"]} value={draft.s4mm.path ?? ""} onChange={(v) => setDraft({ ...draft, s4mm: { ...draft.s4mm, path: v || null } })} />
                  {!env?.s4mm && !draft.s4mm.path && <small className="status warn">{t("wizard.s4mmMissing")}</small>}
                </>
              )}
            </>
          )}

          {step === "mp" && (
            <>
              <h2>{t("wizard.mpTitle")}</h2>
              <p>{t("wizard.mpBody")}</p>
              {yesNo(draft.multiplayer.enabled, (enabled) => setDraft({ ...draft, multiplayer: { ...draft.multiplayer, enabled } }))}
              {draft.multiplayer.enabled && (
                <PathField label={t("wizard.mpPath")} hint={t("wizard.mpPathHint")} extensions={["exe"]} value={draft.multiplayer.launcher ?? ""} onChange={(v) => setDraft({ ...draft, multiplayer: { ...draft.multiplayer, launcher: v || null } })} />
              )}
            </>
          )}

          {step === "tray" && (
            <>
              <h2>{t("wizard.trayTitle")}</h2>
              <p>{t("wizard.trayBody")}</p>
              <p><small className="status">{t("tray.note")}</small></p>
              {yesNo(draft.traySyncDefault, (traySyncDefault) => setDraft({ ...draft, traySyncDefault }))}
            </>
          )}

          {step === "summary" && (
            <>
              <h2>{t("wizard.summaryTitle")}</h2>
              <p>{t("wizard.summaryBody")}</p>
              <dl className="summary">
                <dt>{t("wizard.summaryLanguage")}</dt><dd>{LANGUAGES.find((l) => l.code === draft.language)?.name}</dd>
                <dt>{t("wizard.summaryPlatform")}</dt><dd>{platformName}</dd>
                <dt>{t("wizard.summaryS4mm")}</dt><dd>{onOff(draft.s4mm.enabled)}</dd>
                <dt>{t("wizard.summaryMp")}</dt><dd>{onOff(draft.multiplayer.enabled)}</dd>
                <dt>{t("wizard.summaryTray")}</dt><dd>{onOff(draft.traySyncDefault)}</dd>
              </dl>
            </>
          )}

          {error && <p className="error" role="alert">{error}</p>}

          <div className="actions">
            {(index > 0 || onCancel) && (
              <button type="button" className="ghost" onClick={back}>{index === 0 ? t("common.cancel") : t("common.back")}</button>
            )}
            {step === "summary" ? (
              <button type="button" className="play" onClick={finish}>{t("wizard.finish")}</button>
            ) : (
              <button type="button" className="play" disabled={!ready} onClick={next}>{step === "welcome" ? t("wizard.start") : t("common.next")}</button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
