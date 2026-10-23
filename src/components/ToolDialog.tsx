import { useState } from "react";
import { createPortal } from "react-dom";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { assetUrl } from "../lib/assets";
import { pickFile, pickImages } from "../lib/native";
import type { Tool } from "../lib/types";
import Icon from "./Icon";

const fileName = (p: string) => p.replace(/^.*[\\/]/, "").replace(/\.[^.]+$/, "");
const newId = () => "t" + Math.random().toString(36).slice(2, 10);

export default function ToolDialog({ tool, onClose }: { tool: Tool | null; onClose: () => void }) {
  const { settings, save, t, msg } = useSettings();
  const [name, setName] = useState(tool?.name ?? "");
  const [path, setPath] = useState(tool?.path ?? "");
  const [icon, setIcon] = useState<string | null>(tool?.icon ?? null);
  const [custom, setCustom] = useState(tool?.customIcon ?? false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [sure, setSure] = useState(false);
  const id = tool?.id ?? newId();

  const detect = async (next: string) => {
    if (custom) return;
    const found = await backend.toolIcon(id, next).catch(() => null);
    setIcon(found);
  };

  const choosePath = async () => {
    const picked = await pickFile(["exe", "bat", "cmd", "lnk"]);
    if (!picked) return;
    setPath(picked);
    if (!name.trim()) setName(fileName(picked));
    await detect(picked);
  };

  const chooseIcon = async () => {
    const [picked] = await pickImages();
    if (!picked) return;
    const stored = await backend.importIcon(id, picked).catch(() => null);
    if (stored) {
      setIcon(stored);
      setCustom(true);
    }
  };

  const resetIcon = async () => {
    setCustom(false);
    setIcon(await backend.toolIcon(id, path).catch(() => null));
  };

  const valid = name.trim().length > 0 && path.trim().length > 0;

  const submit = async () => {
    if (!valid || busy) return;
    setBusy(true);
    setError("");
    try {
      const entry: Tool = { id, name: name.trim(), path: path.trim(), icon, customIcon: custom };
      const rest = settings.tools.filter((x) => x.id !== id);
      const next = tool ? settings.tools.map((x) => (x.id === id ? entry : x)) : [...rest, entry];
      await save({ ...settings, tools: next });
      onClose();
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
      setBusy(false);
    }
  };

  const remove = async () => {
    await save({ ...settings, tools: settings.tools.filter((x) => x.id !== id) });
    onClose();
  };

  return createPortal(
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="dialog" role="dialog" aria-modal="true">
        <form className="dialog-body" onSubmit={(e) => { e.preventDefault(); void submit(); }}>
          <h2>{tool ? t("tools.editTitle") : t("tools.addTitle")}</h2>
          <label className="field">
            <span>{t("tools.name")}</span>
            <input type="text" value={name} maxLength={40} autoFocus spellCheck={false} onChange={(e) => setName(e.target.value)} />
          </label>
          <div className="field">
            <span>{t("tools.path")}</span>
            <div className="path-row">
              <input type="text" value={path} spellCheck={false} placeholder="C:\..." onChange={(e) => setPath(e.target.value)} onBlur={() => void detect(path)} />
              <button type="button" className="ghost" onClick={() => void choosePath()}>{t("tools.choose")}</button>
            </div>
          </div>
          <div className="field">
            <span>{t("tools.icon")}</span>
            <div className="path-row">
              <span className="tool-icon big">{icon ? <img src={assetUrl(icon)} alt="" /> : <Icon name="wrench" size={22} />}</span>
              <button type="button" className="ghost" onClick={() => void chooseIcon()}>{t("tools.iconChange")}</button>
              <button type="button" className="ghost" disabled={!custom} onClick={() => void resetIcon()}>{t("tools.iconAuto")}</button>
            </div>
            <small className="status">{t("tools.iconHint")}</small>
          </div>
          {error && <small className="error">{error}</small>}
          <div className="edit-foot">
            <div className="edit-danger">
              {tool && (
                <button type="button" className={"icon-danger red tip left" + (sure ? " sure" : "")} data-tip={sure ? undefined : t("tools.remove")} aria-label={t("tools.remove")} onClick={() => { if (sure) void remove(); else { setSure(true); window.setTimeout(() => setSure(false), 3000); } }}>
                  <Icon name="trash" size={17} />
                  {sure && <span>{t("tools.removeSure")}</span>}
                </button>
              )}
            </div>
            <div className="edit-main">
              <button type="button" className="ghost" onClick={onClose}>{t("common.cancel")}</button>
              <button type="submit" className="play small-play" disabled={!valid || busy}>{t("tools.save")}</button>
            </div>
          </div>
        </form>
      </div>
    </div>,
    document.body,
  );
}
