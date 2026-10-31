import { useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import type { Instance, Role } from "../lib/types";
import Icon from "./Icon";
import AppearanceFields from "./AppearanceFields";
import AppearanceHeader from "./AppearanceHeader";
import NoIntroCard from "./NoIntroCard";
import { appearanceOf, applyAppearance, colorChanged, colorsFor, type Appearance } from "../lib/appearance";

type Step = "duplicate" | "convert" | "delete" | "archive" | "trayOn" | "trayOff";

export default function EditInstanceForm({ instance, onClose, onDone, onStep }: { instance: Instance; onClose: () => void; onDone: () => void; onStep: (step: Step) => void }) {
  const { t, msg, settings, save } = useSettings();
  const [tab, setTab] = useState<"general" | "appearance" | "options">("general");
  const [name, setName] = useState(instance.name);
  const [description, setDescription] = useState(instance.description);
  const [role, setRole] = useState<Role | null>(instance.role ?? null);
  const [busy, setBusy] = useState(false);
  const [lanIp, setLanIp] = useState(settings.lanHosts?.[instance.id] ?? "");
  const initial = appearanceOf(instance, settings);
  const [appearance, setAppearance] = useState<Appearance>(initial);
  const [error, setError] = useState("");
  const multiplayer = settings.multiplayer.enabled;
  const showRole = multiplayer && instance.kind === "mp";

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return setError(t("err.nameRequired"));
    setBusy(true);
    setError("");
    try {
      await backend.updateInstance(instance.id, name, description, showRole ? role : null);
      const styled = await applyAppearance(instance.id, appearance, settings, colorChanged(appearance, initial));
      const rest = { ...styled.lanHosts };
      if (showRole && role === "client" && lanIp.trim()) rest[instance.id] = lanIp.trim();
      else delete rest[instance.id];
      await save({ ...styled, lanHosts: rest });
      onDone();
    } catch (err) {
      setError(msg(err instanceof Error ? err.message : String(err)));
      setBusy(false);
    }
  };

  const storedBanner = settings.instanceBanners?.[instance.id];
  const existingBanner = storedBanner && storedBanner !== "none" ? storedBanner : undefined;

  const kindLabel = multiplayer ? (instance.kind === "solo" ? t("kind.solo") : t("kind.mp")) : "";
  const cards: { key: string; icon: string; title: string; hint?: string; run: () => void }[] = [
    { key: "dup", icon: "copy", title: t("edit.duplicate"), hint: t("edit.duplicateHint"), run: () => onStep("duplicate") },
    { key: "tray", icon: instance.traySynced ? "unlink" : "link", title: instance.traySynced ? t("tray.disableBtn") : t("tray.enableBtn"), hint: instance.traySynced ? undefined : t("tray.enableHint"), run: () => onStep(instance.traySynced ? "trayOff" : "trayOn") },
  ];
  if (multiplayer) {
    cards.push({ key: "convert", icon: "refresh", title: instance.kind === "solo" ? t("edit.convertMp") : t("edit.convertSolo"), run: () => onStep("convert") });
  }

  return (
    <form className="edit-form" onSubmit={submit}>
      <AppearanceHeader
        value={appearance}
        onChange={setAppearance}
        fallback={colorsFor(instance, settings)}
        letter={(name.trim() || instance.name)[0]}
        immediate={instance.id}
        editable={tab === "appearance"}
        existingIcon={settings.instanceIcons[instance.id]}
        existingBanner={existingBanner}
        onError={setError}
      >
        <div className="edit-head-info">
          <h2>{name.trim() || instance.name}</h2>
          <div className="edit-chips">
            {kindLabel && <span className="pill">{kindLabel}</span>}
            {showRole && role && <span className="pill">{t(`role.${role}`)}</span>}
            <span className="pill mono">{instance.id}</span>
          </div>
        </div>
      </AppearanceHeader>

      <div className="edit-tabs" role="tablist">
        <button type="button" role="tab" aria-selected={tab === "general"} className={tab === "general" ? "on" : ""} onClick={() => setTab("general")}><Icon name="info" size={14} /> {t("edit.tabGeneral")}</button>
        <button type="button" role="tab" aria-selected={tab === "appearance"} className={tab === "appearance" ? "on" : ""} onClick={() => setTab("appearance")}><Icon name="brush" size={14} /> {t("edit.tabAppearance")}</button>
        <button type="button" role="tab" aria-selected={tab === "options"} className={tab === "options" ? "on" : ""} onClick={() => setTab("options")}><Icon name="gear" size={14} /> {t("edit.tabOptions")}</button>
      </div>

      <div className="edit-pane" key={tab}>
        {tab === "general" && (
          <>
            <label className="field">
              <span>{t("form.name")}</span>
              <input value={name} onChange={(e) => setName(e.target.value)} maxLength={80} autoFocus />
            </label>
            <label className="field">
              <span>{t("form.desc")}</span>
              <input value={description} onChange={(e) => setDescription(e.target.value)} maxLength={200} />
            </label>
            <label className="field">
              <span>{t("form.code")}</span>
              <input value={instance.id} disabled readOnly />
            </label>
            {showRole && (
              <div className="field">
                <span>{t("role.label")}</span>
                <div className="seg" role="group" aria-label={t("role.label")}>
                  <button type="button" aria-pressed={role === "host"} onClick={() => setRole("host")}>{t("role.host")}</button>
                  <button type="button" aria-pressed={role === "client"} onClick={() => setRole("client")}>{t("role.client")}</button>
                </div>
              </div>
            )}
            {showRole && role === "client" && (
              <label className="field">
                <span>{t("lan.formIp")}</span>
                <small>{t("lan.formIpHint")}</small>
                <input value={lanIp} placeholder="192.168.1.130" spellCheck={false} onChange={(e) => setLanIp(e.target.value)} />
              </label>
            )}
          </>
        )}
        {tab === "appearance" && (
          <AppearanceFields value={appearance} onChange={setAppearance} />
        )}
        {tab === "options" && (
          <>
            <div className="act-grid">
              {cards.map((c) => (
                <button key={c.key} type="button" className="act-card" onClick={c.run}>
                  <span className="act-icon"><Icon name={c.icon} size={20} /></span>
                  <span className="act-text"><b>{c.title}</b>{c.hint && <small>{c.hint}</small>}</span>
                </button>
              ))}
            </div>
            <NoIntroCard code={instance.id} />
          </>
        )}
      </div>

      {error && <p className="error" role="alert">{error}</p>}

      <footer className="edit-foot">
        <div className="edit-danger">
          <button type="button" className="icon-danger tip left" data-tip={t("edit.archive")} aria-label={t("edit.archive")} onClick={() => onStep("archive")}><Icon name="archive" size={17} /></button>
          <button type="button" className="icon-danger red tip left" data-tip={t("edit.delete")} aria-label={t("edit.delete")} onClick={() => onStep("delete")}><Icon name="trash" size={17} /></button>
        </div>
        <div className="edit-main">
          <button type="button" className="ghost" disabled={busy} onClick={onClose}>{t("common.cancel")}</button>
          <button type="submit" className="play small-play" disabled={busy}>{busy ? t("common.working") : t("edit.save")}</button>
        </div>
      </footer>
    </form>
  );
}
