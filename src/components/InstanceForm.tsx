import { useState } from "react";
import { useSettings } from "../lib/settings";
import type { InstanceInput, InstanceKind, Role } from "../lib/types";

interface Props {
  title: string;
  submitLabel: string;
  initial?: Partial<InstanceInput>;
  withKind?: boolean;
  lockCode?: boolean;
  withRole?: boolean;
  extra?: React.ReactNode;
  onSubmit: (v: InstanceInput) => Promise<void>;
  onSecondary?: { label: string; run: () => void | Promise<void> };
}

const CODE = /^[A-Za-z0-9]{1,32}$/;

export default function InstanceForm({ title, submitLabel, initial, withKind, lockCode, withRole, extra, onSubmit, onSecondary }: Props) {
  const { t, msg, settings } = useSettings();
  const [name, setName] = useState(initial?.name ?? "");
  const [description, setDescription] = useState(initial?.description ?? "");
  const [code, setCode] = useState(initial?.code ?? "");
  const [kind, setKind] = useState<InstanceKind>(initial?.kind ?? "solo");
  const [role, setRole] = useState<Role | null>(initial?.role ?? (withKind ? "host" : null));
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const showKind = withKind && settings.multiplayer.enabled;
  const showRole = (showKind && kind === "mp") || (withRole && settings.multiplayer.enabled);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return setError(t("err.nameRequired"));
    if (!CODE.test(code)) return setError(t("err.codeInvalid"));
    setBusy(true);
    setError("");
    try {
      await onSubmit({ name, description, code, kind, role: showRole ? role : null });
    } catch (err) {
      setError(msg(err instanceof Error ? err.message : String(err)));
      setBusy(false);
    }
  };

  const secondary = async () => {
    setBusy(true);
    setError("");
    try {
      await onSecondary?.run();
    } catch (err) {
      setError(msg(err instanceof Error ? err.message : String(err)));
    }
    setBusy(false);
  };

  return (
    <form className="dialog-body" onSubmit={submit}>
      <h2>{title}</h2>
      <label className="field">
        <span>{t("form.name")}</span>
        <small>{t("form.nameHint")}</small>
        <input value={name} onChange={(e) => setName(e.target.value)} maxLength={80} autoFocus />
      </label>
      <label className="field">
        <span>{t("form.desc")}</span>
        <small>{t("form.descHint")}</small>
        <input value={description} onChange={(e) => setDescription(e.target.value)} maxLength={200} />
      </label>
      <label className="field">
        <span>{t("form.code")}</span>
        <small>{t("form.codeHint")}</small>
        <input value={code} onChange={(e) => setCode(e.target.value)} maxLength={32} disabled={lockCode} />
      </label>
      {extra}
      {showKind && (
        <div className="seg" role="group" aria-label={t("form.kindAria")}>
          <button type="button" aria-pressed={kind === "solo"} onClick={() => setKind("solo")}>{t("kind.solo")}</button>
          <button type="button" aria-pressed={kind === "mp"} onClick={() => setKind("mp")}>{t("kind.mp")}</button>
        </div>
      )}
      {showRole && (
        <div className="field">
          <span>{t("role.label")}</span>
          <div className="seg" role="group" aria-label={t("role.label")}>
            <button type="button" aria-pressed={role === "host"} onClick={() => setRole("host")}>{t("role.host")}</button>
            <button type="button" aria-pressed={role === "client"} onClick={() => setRole("client")}>{t("role.client")}</button>
          </div>
        </div>
      )}
      {error && <p className="error" role="alert">{error}</p>}
      <div className="actions">
        {onSecondary && <button type="button" className="ghost" disabled={busy} onClick={secondary}>{onSecondary.label}</button>}
        <button type="submit" className="play" disabled={busy}>{busy ? t("common.working") : submitLabel}</button>
      </div>
    </form>
  );
}
