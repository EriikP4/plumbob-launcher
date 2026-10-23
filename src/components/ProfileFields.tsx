import { useEffect, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import ProfilePicker from "./ProfilePicker";

export default function ProfileFields() {
  const { settings, saveSoon, t } = useSettings();
  const [defaultName, setDefaultName] = useState("");

  useEffect(() => {
    backend.profileInfo().then((i) => setDefaultName(i.systemName)).catch(() => undefined);
  }, []);

  return (
    <>
      <label className="field">
        <span>{t("profile.name")}</span>
        <input type="text" value={settings.profile.name ?? ""} maxLength={32} placeholder={defaultName} spellCheck={false} onChange={(e) => saveSoon({ ...settings, profile: { ...settings.profile, name: e.target.value || null } })} />
        <small className="status">{t("profile.nameHint")}</small>
      </label>
      <div className="field">
        <span>{t("profile.photo")}</span>
        <ProfilePicker />
      </div>
    </>
  );
}
