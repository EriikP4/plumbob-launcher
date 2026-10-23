import { useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { assetUrl } from "../lib/assets";
import { pickImages } from "../lib/native";

export default function IconField({ code }: { code: string }) {
  const { settings, save, t, msg } = useSettings();
  const [error, setError] = useState("");
  const icon = settings.instanceIcons[code];

  const choose = async () => {
    setError("");
    const [picked] = await pickImages();
    if (!picked) return;
    try {
      const stored = await backend.importIcon(code, picked);
      await save({ ...settings, instanceIcons: { ...settings.instanceIcons, [code]: stored } });
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
    }
  };

  const remove = () => {
    const rest = { ...settings.instanceIcons };
    delete rest[code];
    void save({ ...settings, instanceIcons: rest });
  };

  return (
    <div className="field icon-field">
      <span>{t("edit.icon")}</span>
      <div className="path-row">
        {icon && <img className="icon-preview" src={assetUrl(icon)} alt="" />}
        <button type="button" className="ghost" onClick={choose}>{t("edit.iconChoose")}</button>
        <button type="button" className="ghost" disabled={!icon} onClick={remove}>{t("edit.iconRemove")}</button>
      </div>
      <small className="status">{t("edit.iconHint")}</small>
      {error && <small className="error">{error}</small>}
    </div>
  );
}
