import { useEffect, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import type { InstanceInput, InstanceKind, Role } from "../lib/types";
import { applyAppearance, defaultAppearance, type Appearance } from "../lib/appearance";
import AppearanceFields from "./AppearanceFields";
import AppearanceHeader from "./AppearanceHeader";

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
  steps?: { offset: number; total: number };
}

const CODE = /^[A-Za-z0-9]{1,32}$/;

export default function InstanceForm({ title, submitLabel, initial, withKind, lockCode, withRole, extra, onSubmit, onSecondary, steps }: Props) {
  const { t, msg, settings, save } = useSettings();
  const [name, setName] = useState(initial?.name ?? "");
  const [description, setDescription] = useState(initial?.description ?? "");
  const [code, setCode] = useState(initial?.code ?? "");
  const [kind, setKind] = useState<InstanceKind>(initial?.kind ?? "solo");
  const [role, setRole] = useState<Role | null>(initial?.role ?? (withKind ? "host" : null));
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [page, setPage] = useState(0);
  const [lanIp, setLanIp] = useState("");
  const [ips, setIps] = useState("");
  const [appearance, setAppearance] = useState<Appearance>(defaultAppearance);
  const showKind = withKind && settings.multiplayer.enabled;
  const showRole = (showKind && kind === "mp") || (withRole && settings.multiplayer.enabled);

  const paged = !!extra || !!showKind || !!showRole;
  const asClient = !!showRole && role === "client";
  const asHost = !!showRole && role === "host";

  useEffect(() => {
    if (!asHost) return;
    backend.lanAddresses().then((l) => setIps(l.map((a) => a.ip).join(", "))).catch(() => undefined);
  }, [asHost]);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return setError(t("err.nameRequired"));
    if (!CODE.test(code)) return setError(t("err.codeInvalid"));
    if (paged && page === 0) {
      setError("");
      setPage(1);
      return;
    }
    setBusy(true);
    setError("");
    const ip = asClient ? lanIp.trim() : "";
    let changed = false;
    try {
      const styled = await applyAppearance(code, appearance, settings);
      await save(ip ? { ...styled, lanHosts: { ...styled.lanHosts, [code]: ip } } : styled);
      changed = true;
      await onSubmit({ name, description, code, kind, role: showRole ? role : null });
    } catch (err) {
      if (changed) void save(settings);
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

  const offset = steps?.offset ?? 0;
  const total = steps?.total ?? (paged ? 2 : 0);
  const first = !paged || page === 0;

  return (
    <form className="dialog-body" onSubmit={submit} key={page}>
      {paged && (
        <>
          <div className="dots" aria-hidden="true">
            {Array.from({ length: total }, (_, i) => <i key={i} className={i <= offset + page ? "on" : ""} />)}
          </div>
          <small className="status">{t("derive.step", { n: offset + page + 1, total })}{page === 0 ? ` · ${t("form.infoPage")}` : ""}</small>
        </>
      )}
      {first && (
        <>
          <AppearanceHeader value={appearance} onChange={setAppearance} fallback={["#6b5bd6", "#1b1745"]} letter={(name.trim() || "?")[0].toUpperCase()} immediate="draft" editable onError={setError}>
            <div className="edit-head-info">
              <h2>{title}</h2>
              <div className="edit-chips"><span className="pill">{name.trim() || t("form.name")}</span></div>
            </div>
          </AppearanceHeader>
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
          <AppearanceFields value={appearance} onChange={setAppearance} />
        </>
      )}
      {paged && page === 1 && <h2>{t("form.settingsTitle")}</h2>}
      {(!paged || page === 1) && (
        <>
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
          {asClient && (
            <label className="field">
              <span>{t("lan.formIp")}</span>
              <small>{t("lan.formIpHint")}</small>
              <input value={lanIp} placeholder="192.168.1.130" spellCheck={false} onChange={(e) => setLanIp(e.target.value)} />
            </label>
          )}
          {asHost && ips && <small className="status">{t("lan.formHostHint", { ips })}</small>}
        </>
      )}
      {error && <p className="error" role="alert">{error}</p>}
      <div className="actions">
        {paged && page === 1 ? (
          <button type="button" className="ghost" disabled={busy} onClick={() => { setError(""); setPage(0); }}>{t("common.back")}</button>
        ) : (
          onSecondary && <button type="button" className="ghost" disabled={busy} onClick={secondary}>{onSecondary.label}</button>
        )}
        <button type="submit" className="play" disabled={busy}>{busy ? t("common.working") : paged && page === 0 ? t("common.next") : submitLabel}</button>
      </div>
    </form>
  );
}
