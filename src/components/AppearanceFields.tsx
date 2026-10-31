import { useState } from "react";
import { useSettings } from "../lib/settings";
import Icon from "./Icon";
import { PRESET_COLORS, isHex, type Appearance } from "../lib/appearance";

interface Props {
  value: Appearance;
  onChange: (next: Appearance) => void;
}

export function ColorField({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const { t } = useSettings();
  const preset = PRESET_COLORS.indexOf(value.toLowerCase());
  const custom = value !== "random" && preset < 0;
  const [open, setOpen] = useState(custom);
  const [draft, setDraft] = useState(isHex(value) ? value : "#7fa9ff");
  const name = (i: number) => t(`appearance.c${i}` as never);
  const selected = value === "random" ? t("appearance.random") : preset >= 0 && !open ? name(preset) : `${t("appearance.custom")} ${value.toUpperCase()}`;

  const setCustom = (hex: string) => {
    setDraft(hex);
    if (isHex(hex)) onChange(hex.toLowerCase());
  };

  return (
    <div className="color-field">
      <div className="color-row" role="group" aria-label={t("appearance.color")}>
        {PRESET_COLORS.map((c, i) => (
          <button key={c} type="button" className="color-dot tip left" data-tip={name(i)} style={{ background: c }} aria-label={name(i)} aria-pressed={!open && value.toLowerCase() === c} onClick={() => { setOpen(false); onChange(c); }} />
        ))}
        <button type="button" className="color-dot color-random tip right" data-tip={t("appearance.random")} aria-label={t("appearance.random")} aria-pressed={value === "random"} onClick={() => { setOpen(false); onChange("random"); }}><Icon name="shuffle" size={15} /></button>
        <button type="button" className="color-dot color-custom tip right" data-tip={t("appearance.custom")} aria-label={t("appearance.custom")} aria-pressed={custom || open} onClick={() => { setOpen(true); setCustom(isHex(value) ? value : draft); }}>
          <i /><i /><i /><i />
        </button>
      </div>
      <small className="status">{t("appearance.selected", { name: selected })}</small>
      {(open || custom) && (
        <div className="color-custom-panel">
          <input type="color" value={isHex(draft) ? draft : "#7fa9ff"} aria-label={t("appearance.custom")} onChange={(e) => setCustom(e.target.value)} />
          <input type="text" value={draft} maxLength={7} spellCheck={false} aria-label={t("appearance.hex")} onChange={(e) => setCustom(e.target.value.startsWith("#") ? e.target.value : "#" + e.target.value)} />
        </div>
      )}
    </div>
  );
}

export default function AppearanceFields({ value, onChange }: Props) {
  const { t } = useSettings();

  return (
    <fieldset className="appearance">
      <legend>{t("appearance.title")}</legend>
      <div className="field">
        <span>{t("appearance.color")}</span>
        <ColorField value={value.color} onChange={(color) => onChange({ ...value, color })} />
      </div>
    </fieldset>
  );
}
