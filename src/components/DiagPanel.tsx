import { Fragment, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import type { Diagnostics } from "../lib/types";

export function DiagWarnings({ diag }: { diag: Diagnostics | null }) {
  const { t } = useSettings();
  if (!diag) return null;
  const warnings = [
    diag.oneDrive && t("diag.warnOneDrive"),
    diag.cloudFiles > 0 && t("diag.warnCloud", { n: diag.cloudFiles }),
    diag.lockedFiles > 0 && t("diag.warnLocked", { n: diag.lockedFiles }),
    diag.writeError && t("diag.warnWrite", { error: diag.writeError }),
  ].filter((w): w is string => !!w);
  if (warnings.length === 0) return null;
  return (
    <div className="diag-box">
      {warnings.map((w) => <div key={w} className="banner warn" role="alert"><span>{w}</span></div>)}
    </div>
  );
}

export default function DiagPanel() {
  const { t, msg } = useSettings();
  const [diag, setDiag] = useState<Diagnostics | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);

  const yes = (v: boolean) => (v ? t("common.yes") : t("common.no"));
  const lines = (d: Diagnostics): [string, string][] => [
    [t("diag.root"), d.gameRoot],
    [t("diag.gameDir"), d.gameDir],
    [t("diag.oneDrive"), yes(d.oneDrive)],
    [t("diag.cloud"), String(d.cloudFiles)],
    [t("diag.locked"), String(d.lockedFiles)],
    ...(d.lockedSample ? [[t("diag.firstLocked"), d.lockedSample] as [string, string]] : []),
    [t("diag.write"), d.writeError ?? t("diag.writeOk")],
    [t("diag.gameRunning"), yes(d.gameRunning)],
  ];

  const run = async () => {
    setBusy(true);
    setError("");
    setCopied(false);
    try {
      setDiag(await backend.diagnose());
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
    }
    setBusy(false);
  };

  const copy = async () => {
    if (!diag) return;
    try {
      await navigator.clipboard.writeText(lines(diag).map(([k, v]) => `${k}: ${v}`).join("\n"));
      setCopied(true);
    } catch {
      setCopied(false);
    }
  };

  return (
    <section className="card">
      <h2>{t("diag.title")}</h2>
      <small className="status">{t("diag.hint")}</small>
      <div className="actions">
        <button type="button" className="ghost" disabled={busy} onClick={run}>{busy ? t("diag.running") : t("diag.run")}</button>
        {diag && <button type="button" className="ghost" onClick={copy}>{copied ? t("diag.copied") : t("diag.copy")}</button>}
      </div>
      {diag && (
        <>
          <dl className="summary">
            {lines(diag).map(([k, v]) => <Fragment key={k}><dt>{k}</dt><dd>{v}</dd></Fragment>)}
          </dl>
          <DiagWarnings diag={diag} />
        </>
      )}
      {error && <p className="error" role="alert">{error}</p>}
    </section>
  );
}
