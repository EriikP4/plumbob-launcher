import { useEffect, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { assetUrl } from "../lib/assets";
import type { ProfileInfo } from "../lib/types";
import ProfileDialog from "./ProfileDialog";

export default function ProfileButton() {
  const { settings, t } = useSettings();
  const [info, setInfo] = useState<ProfileInfo | null>(null);
  const [editing, setEditing] = useState(false);
  const { source, custom, name } = settings.profile;

  useEffect(() => {
    backend.profileInfo().then(setInfo).catch(() => setInfo(null));
  }, [source, custom, name]);

  if (!info) return null;
  const shown = info.name || "?";
  return (
    <>
      <button type="button" className="bar-avatar tip side" data-tip={`${shown} · ${t("profile.edit")}`} aria-label={t("profile.edit")} onClick={() => setEditing(true)}>
        <span className="avatar">{info.image ? <img src={assetUrl(info.image)} alt="" decoding="async" /> : shown[0].toUpperCase()}</span>
        <span className="bar-name">{shown}</span>
      </button>
      {editing && <ProfileDialog onClose={() => setEditing(false)} />}
    </>
  );
}
