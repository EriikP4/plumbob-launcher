import { useEffect, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { assetUrl } from "../lib/assets";
import { pickImages } from "../lib/native";
import type { ProfileInfo } from "../lib/types";
import Icon from "./Icon";
import BustPicker from "./BustPicker";

export default function ProfilePicker() {
  const { settings, save, t } = useSettings();
  const [win, setWin] = useState<ProfileInfo | null>(null);
  const [busts, setBusts] = useState(false);
  const { source, custom, name } = settings.profile;

  useEffect(() => {
    backend.profileInfo("windows").then(setWin).catch(() => setWin(null));
  }, []);

  const letter = ((name?.trim() || win?.name || "?")[0] ?? "?").toUpperCase();

  const store = async (path: string) => {
    const stored = await backend.importIcon("profile", path).catch(() => null);
    if (stored) await save({ ...settings, profile: { ...settings.profile, source: "custom", custom: stored } });
  };

  const choose = async () => {
    const [picked] = await pickImages();
    if (picked) await store(picked);
  };

  const useBust = (path: string) => {
    setBusts(false);
    void store(path);
  };

  const pick = (next: "windows" | "none") => void save({ ...settings, profile: { ...settings.profile, source: next } });

  const useCustom = () => {
    if (!custom) return void choose();
    void save({ ...settings, profile: { ...settings.profile, source: "custom" } });
  };

  const options = [
    {
      key: "windows",
      label: t("settings.profileWindows"),
      on: source === "windows",
      click: () => pick("windows"),
      face: <span className="po-face">{win?.image ? <img src={assetUrl(win.image)} alt="" /> : letter}</span>,
    },
    {
      key: "custom",
      label: t("settings.profileCustom"),
      on: source === "custom",
      click: useCustom,
      face: <span className={"po-face" + (custom ? "" : " empty")}>{custom ? <img src={assetUrl(custom)} alt="" /> : <Icon name="upload" size={26} />}</span>,
      overlay: custom ? (
        <button type="button" className="po-change" aria-label={t("settings.profileEdit")} onClick={() => void choose()}>
          <Icon name="upload" size={20} />
          <small>{t("settings.profileEdit")}</small>
        </button>
      ) : null,
    },
    {
      key: "sim",
      label: t("settings.profileBust"),
      on: false,
      click: () => setBusts(true),
      face: <span className="po-face empty"><Icon name="simphoto" size={30} /></span>,
    },
    {
      key: "none",
      label: t("settings.profileNone"),
      on: source === "none",
      click: () => pick("none"),
      face: <span className="po-face">{letter}</span>,
    },
  ];

  return (
    <div className="profile-picker">
      {options.map((o) => (
        <div key={o.key} className="po-item">
          <div className="po-slot">
            <button type="button" className="profile-opt" aria-label={o.label} aria-pressed={o.on} onClick={o.click}>
              {o.face}
              {o.on && <span className="po-check" aria-hidden="true"><Icon name="check" size={12} /></span>}
            </button>
            {o.overlay}
          </div>
          <span className={"po-label" + (o.on ? " on" : "")}>{o.label}</span>
        </div>
      ))}
      {busts && <BustPicker onClose={() => setBusts(false)} onPick={useBust} />}
    </div>
  );
}
