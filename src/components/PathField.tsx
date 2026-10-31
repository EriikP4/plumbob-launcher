import { useSettings } from "../lib/settings";
import { pickFile, pickFolder } from "../lib/native";

interface Props {
  label: string;
  hint?: string;
  value: string;
  placeholder?: string;
  folder?: boolean;
  extensions?: string[];
  pickLabel?: string;
  labelExtra?: React.ReactNode;
  onChange: (value: string) => void;
}

export default function PathField({ label, labelExtra, hint, value, placeholder, folder, extensions, pickLabel, onChange }: Props) {
  const { t } = useSettings();
  const pick = async () => {
    const chosen = folder ? await pickFolder() : await pickFile(extensions);
    if (chosen) onChange(chosen);
  };
  return (
    <div className="field">
      <span>{label}{labelExtra}</span>
      {hint && <small>{hint}</small>}
      <div className="path-row">
        <input value={value} placeholder={placeholder} onChange={(e) => onChange(e.target.value)} spellCheck={false} />
        <button type="button" className="ghost" onClick={pick}>{pickLabel ?? t("common.choose")}</button>
      </div>
    </div>
  );
}
