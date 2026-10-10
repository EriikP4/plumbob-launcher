import { useSettings } from "../lib/settings";
import type { GameOptions } from "../lib/types";

export const RECOMMENDED: GameOptions = { mods: true, skipDlcIntro: true, noTelemetry: true, music: "half", windowed: true };
export const NONE: GameOptions = { mods: false, skipDlcIntro: false, noTelemetry: false, music: "keep", windowed: false };

const same = (a: GameOptions, b: GameOptions) =>
  a.mods === b.mods && a.skipDlcIntro === b.skipDlcIntro && a.noTelemetry === b.noTelemetry && a.music === b.music && a.windowed === b.windowed;

export default function GameOptionsFields({ value, onChange }: { value: GameOptions; onChange: (v: GameOptions) => void }) {
  const { t } = useSettings();
  const recommended = same(value, RECOMMENDED);
  const set = (patch: Partial<GameOptions>) => onChange({ ...value, ...patch });
  return (
    <fieldset className="game-options">
      <legend>{t("opts.title")}</legend>
      <label className="check recommended">
        <input type="checkbox" checked={recommended} onChange={(e) => onChange(e.target.checked ? RECOMMENDED : NONE)} />
        <span>{t("opts.recommended")}</span>
      </label>
      <div className="opts-list">
        <label className="check">
          <input type="checkbox" checked={value.mods} onChange={(e) => set({ mods: e.target.checked })} />
          <span>{t("opts.mods")}</span>
        </label>
        <label className="check">
          <input type="checkbox" checked={value.skipDlcIntro} onChange={(e) => set({ skipDlcIntro: e.target.checked })} />
          <span>{t("opts.skipDlc")}</span>
          <span className="tip left info-dot" tabIndex={0} data-tip={t("opts.skipDlcInfo")} aria-label={t("opts.skipDlcInfo")} onClick={(e) => e.preventDefault()}>i</span>
        </label>
        <label className="check">
          <input type="checkbox" checked={value.noTelemetry} onChange={(e) => set({ noTelemetry: e.target.checked })} />
          <span>{t("opts.telemetry")}</span>
        </label>
        <div className="opts-music">
          <label className="check">
            <input type="checkbox" checked={value.music === "half"} onChange={(e) => set({ music: e.target.checked ? "half" : "keep" })} />
            <span>{t("opts.musicHalf")}</span>
          </label>
          <label className="check">
            <input type="checkbox" checked={value.music === "off"} onChange={(e) => set({ music: e.target.checked ? "off" : "keep" })} />
            <span>{t("opts.musicOff")}</span>
          </label>
        </div>
        <label className="check">
          <input type="checkbox" checked={value.windowed} onChange={(e) => set({ windowed: e.target.checked })} />
          <span>{t("opts.windowed")}</span>
        </label>
      </div>
    </fieldset>
  );
}
