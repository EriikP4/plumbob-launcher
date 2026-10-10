import { useEffect, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { pickSavePath, pickTextFiles } from "../lib/native";
import type { CodeCheck, Instance, ModDiff, ModFile, ModScan } from "../lib/types";
import Icon from "./Icon";

function parseManifest(text: string): ModFile[] | null {
  try {
    const data = JSON.parse(text) as { files?: unknown };
    if (!Array.isArray(data.files)) return null;
    const files = data.files.filter((f): f is ModFile => !!f && typeof (f as ModFile).name === "string" && typeof (f as ModFile).hash === "string");
    return files.map((f) => ({ name: f.name, path: String(f.path ?? f.name), size: Number(f.size) || 0, hash: f.hash }));
  } catch {
    return null;
  }
}

function FileList({ title, tone, files }: { title: string; tone: "warn" | "muted" | "bad"; files: { key: string; label: string; note?: string }[] }) {
  if (files.length === 0) return null;
  return (
    <div className="diff-group">
      <h4 className={"diff-" + tone}>{title} <span className="pill">{files.length}</span></h4>
      <div className="diff-items">
        {files.slice(0, 200).map((f) => (
          <div className="diff-item" key={f.key}>
            <span>{f.label}</span>
            {f.note && <small>{f.note}</small>}
          </div>
        ))}
        {files.length > 200 && <small className="status">+ {files.length - 200}</small>}
      </div>
    </div>
  );
}

export default function HashDialog({ instance, onClose }: { instance: Instance; onClose: () => void }) {
  const { t, msg, size } = useSettings();
  const [scan, setScan] = useState<ModScan | null>(null);
  const [scanning, setScanning] = useState(false);
  const [progress, setProgress] = useState({ done: 0, total: 1 });
  const [hostCode, setHostCode] = useState("");
  const [check, setCheck] = useState<CodeCheck | null>(null);
  const [diff, setDiff] = useState<ModDiff | null>(null);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    const off = backend.onHashProgress((p) => setProgress({ done: p.done, total: Math.max(1, p.total) }));
    return () => void off.then((f) => f());
  }, []);

  const run = async (): Promise<ModScan | null> => {
    setError("");
    setScanning(true);
    setProgress({ done: 0, total: 1 });
    try {
      const s = await backend.scanMods(instance.id);
      setScan(s);
      return s;
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
      return null;
    } finally {
      setScanning(false);
    }
  };

  const compare = async () => {
    setError("");
    setMessage("");
    const s = scan ?? (await run());
    if (!s) return;
    setCheck(await backend.checkCode(s.code, hostCode));
  };

  const copy = async () => {
    if (!scan) return;
    try {
      await navigator.clipboard.writeText(scan.code);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1800);
    } catch {
      setMessage(scan.code);
    }
  };

  const exportReport = async () => {
    if (!scan) return;
    const path = await pickSavePath(`mods-${instance.id}.json`, "json");
    if (!path) return;
    try {
      await backend.exportText(path, JSON.stringify({ app: "plumbob-launcher", kind: "mods-manifest", code: scan.code, files: scan.files }, null, 2));
      setMessage(t("hash.exported"));
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
    }
  };

  const importReport = async () => {
    setError("");
    setMessage("");
    const [file] = await pickTextFiles();
    if (!file) return;
    try {
      const host = parseManifest(await backend.readImport(file));
      if (!host) return setError(t("hash.badReport"));
      const s = scan ?? (await run());
      if (!s) return;
      setDiff(await backend.compareMods(host, s.files));
      setCheck(null);
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
    }
  };

  const percent = Math.min(100, Math.round((progress.done / progress.total) * 100));
  const bytes = scan ? scan.files.reduce((n, f) => n + f.size, 0) : 0;
  const isClient = instance.role === "client";

  const mine = (
    <section className="hash-block" key="mine">
      <h3>{t("hash.mineTitle")}</h3>
      <p className="status">{t("hash.mineHint")}</p>
      {!scan && !scanning && <button type="button" className="play" onClick={() => void run()}>{t("hash.calc")}</button>}
      {scanning && (
        <>
          <div className="progress" role="progressbar" aria-valuenow={percent}><i style={{ width: `${percent}%` }} /></div>
          <small className="status">{t("hash.scanning", { done: progress.done, total: progress.total })}</small>
        </>
      )}
      {scan && (
        <>
          <div className="hash-code">
            <code>{scan.code}</code>
            <button type="button" className="ghost small" onClick={() => void copy()}>{copied ? t("hash.copied") : t("hash.copy")}</button>
          </div>
          <small className="status">{t("hash.summary", { packages: scan.packages, scripts: scan.scripts, size: size(bytes) })}</small>
          <div className="actions left">
            <button type="button" className="ghost small" onClick={() => void run()} disabled={scanning}>{t("hash.recalc")}</button>
            <button type="button" className="ghost small" onClick={() => void exportReport()}>{t("hash.export")}</button>
          </div>
        </>
      )}
    </section>
  );

  const other = (
    <section className="hash-block" key="other">
      <h3>{t("hash.compareTitle")}</h3>
      <p className="status">{t("hash.compareHint")}</p>
      <div className="path-row">
        <input value={hostCode} placeholder="PB1-830-XXXXX-XXXXX-XXXXX-XXXXX" spellCheck={false} onChange={(e) => { setHostCode(e.target.value); setCheck(null); }} />
        <button type="button" className="play" disabled={!hostCode.trim() || scanning} onClick={() => void compare()}>{t("hash.compare")}</button>
      </div>
      {check && !check.valid && <p className="error" role="alert">{t("hash.invalidCode")}</p>}
      {check?.valid && check.equal && <p className="hash-ok"><Icon name="check" size={16} /> {t("hash.same")}</p>}
      {check?.valid && !check.equal && (
        <p className="hash-bad"><Icon name="warn" size={16} /> {t("hash.different", { host: check.hostCount ?? 0, mine: scan?.files.length ?? 0 })}</p>
      )}
      <div className="actions left">
        <button type="button" className="ghost small" disabled={scanning} onClick={() => void importReport()}>{t("hash.import")}</button>
      </div>
      {diff && (
        <div className="diff">
          {diff.missing.length + diff.extra.length + diff.different.length === 0 && <p className="hash-ok"><Icon name="check" size={16} /> {t("hash.same")}</p>}
          <FileList title={t("hash.missing")} tone="bad" files={diff.missing.map((f) => ({ key: f.path + f.hash, label: f.name, note: f.path }))} />
          <FileList title={t("hash.changed")} tone="warn" files={diff.different.map((c) => ({ key: c.host.path + c.host.hash, label: c.host.name, note: c.local.path }))} />
          <FileList title={t("hash.extra")} tone="muted" files={diff.extra.map((f) => ({ key: f.path + f.hash, label: f.name, note: f.path }))} />
        </div>
      )}
    </section>
  );

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !scanning && onClose()}>
      <div className="dialog wide hash-dialog" role="dialog" aria-modal="true">
        <div className="dialog-body">
          <h2 className="notice-title"><Icon name="shield" size={22} /> {t("hash.title", { name: instance.name })}</h2>
          {instance.role && <small className="status">{instance.role === "host" ? t("role.hostHint") : t("role.clientHint")}</small>}
          {isClient ? [other, mine] : [mine, other]}
          {message && <small className="status">{message}</small>}
          {error && <p className="error" role="alert">{error}</p>}
          <div className="actions"><button type="button" className="ghost" disabled={scanning} onClick={onClose}>{t("common.close")}</button></div>
        </div>
      </div>
    </div>
  );
}
