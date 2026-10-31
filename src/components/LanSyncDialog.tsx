import { useEffect, useMemo, useRef, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import type { Instance, LanDiff, LanHostInfo, LanHostStatus, LanItem, LanPlan, LanProgress, LanReport, Role } from "../lib/types";
import Icon from "./Icon";

type Page = "role" | "host" | "connect" | "review" | "confirm" | "done";
type Kind = "missing" | "different" | "extra";

interface Row {
  key: string;
  kind: Kind;
  item: LanItem;
  localPath?: string;
}

const GROUPS = ["wickedwhims", "basementaldrugs", "mccc", "translations"] as const;
const CLIENT_PAGES: Page[] = ["connect", "review", "confirm", "done"];

const rowsOf = (diff: LanDiff): Row[] => [
  ...diff.missing.map((item): Row => ({ key: "m|" + item.path + "|" + item.hash, kind: "missing", item })),
  ...diff.different.map((c): Row => ({ key: "d|" + c.host.path + "|" + c.host.hash, kind: "different", item: c.host, localPath: c.local.path })),
  ...diff.extra.map((item): Row => ({ key: "e|" + item.path + "|" + item.hash, kind: "extra", item })),
];

export default function LanSyncDialog({ instance, onClose, onChanged, bare }: { instance: Instance; onClose: () => void; onChanged?: () => void; bare?: boolean }) {
  const { t, msg, size, settings, save } = useSettings();
  const saved = settings.lanHosts?.[instance.id] ?? "";
  const [page, setPage] = useState<Page>(instance.role === "host" ? "host" : instance.role === "client" ? "connect" : "role");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [waiting, setWaiting] = useState(false);
  const [scan, setScan] = useState({ done: 0, total: 0 });
  const [hostInfo, setHostInfo] = useState<LanHostInfo | null>(null);
  const [status, setStatus] = useState<LanHostStatus | null>(null);
  const [remember, setRemember] = useState(true);
  const [copied, setCopied] = useState("");
  const [addr, setAddr] = useState(saved);
  const [editing, setEditing] = useState(!saved);
  const [saveIp, setSaveIp] = useState(true);
  const [diff, setDiff] = useState<LanDiff | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [filter, setFilter] = useState<string>("all");
  const [backup, setBackup] = useState(true);
  const [progress, setProgress] = useState<LanProgress | null>(null);
  const [report, setReport] = useState<LanReport | null>(null);
  const [target, setTarget] = useState(saved);
  const auto = useRef(false);

  useEffect(() => {
    const offHash = backend.onHashProgress((p) => setScan({ done: p.done, total: p.total }));
    const offLan = backend.onLanProgress((p) => setProgress(p));
    const offWait = backend.onLanWaiting(() => setWaiting(true));
    return () => {
      void offHash.then((f) => f());
      void offLan.then((f) => f());
      void offWait.then((f) => f());
    };
  }, []);

  useEffect(() => {
    if (page !== "host") return;
    let live = true;
    setError("");
    setBusy(true);
    setScan({ done: 0, total: 0 });
    backend
      .lanHostStart(instance.id)
      .then((info) => live && setHostInfo(info))
      .catch((e) => live && setError(msg(e instanceof Error ? e.message : String(e))))
      .finally(() => live && setBusy(false));
    return () => {
      live = false;
      setHostInfo(null);
      setStatus(null);
      void backend.lanHostStop().catch(() => undefined);
    };
  }, [page, instance.id, msg]);

  useEffect(() => {
    if (!hostInfo) return;
    let live = true;
    const poll = () => backend.lanHostStatus().then((s) => live && setStatus(s)).catch(() => undefined);
    void poll();
    const id = window.setInterval(poll, 1000);
    return () => {
      live = false;
      window.clearInterval(id);
    };
  }, [hostInfo]);

  const fail = (e: unknown) => setError(msg(e instanceof Error ? e.message : String(e)));

  const connect = async (address: string, persist: boolean) => {
    setError("");
    setBusy(true);
    setWaiting(false);
    setScan({ done: 0, total: 0 });
    try {
      const d = await backend.lanDiff(instance.id, address.trim());
      if (persist) await save({ ...settings, lanHosts: { ...settings.lanHosts, [instance.id]: address.trim() } });
      setTarget(address.trim());
      setTarget(address.trim());
      setDiff(d);
      const initial = new Set<string>();
      for (const r of rowsOf(d)) if (r.kind !== "extra") initial.add(r.key);
      setSelected(initial);
      setFilter("all");
      setPage("review");
    } catch (e) {
      fail(e);
      setEditing(true);
    } finally {
      setBusy(false);
      setWaiting(false);
    }
  };

  useEffect(() => {
    if (page !== "connect" || !saved || auto.current) return;
    auto.current = true;
    void connect(saved, false);
  }, [page, saved]);

  const switchRole = async (next: Role) => {
    setError("");
    try {
      await backend.updateInstance(instance.id, instance.name, instance.description, next);
      onChanged?.();
    } catch (e) {
      return fail(e);
    }
    auto.current = false;
    setEditing(!saved);
    setPage(next === "host" ? "host" : "connect");
  };

  const copy = async (text: string, id: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(id);
      window.setTimeout(() => setCopied(""), 1600);
    } catch {
      setError(text);
    }
  };

  const rows = useMemo(() => (diff ? rowsOf(diff) : []), [diff]);
  const groupOf = (r: Row) => r.item.group ?? "other";
  const visible = rows.filter((r) => filter === "all" || groupOf(r) === filter);
  const counts = (g: string) => rows.filter((r) => g === "all" || groupOf(r) === g).length;
  const chips = ["all", ...GROUPS, "other"].filter((g) => g === "all" || counts(g) > 0);
  const groupLabel = (g: string) => (g === "all" ? t("lan.groupAll") : g === "other" ? t("lan.groupOther") : t(`derive.group.${g}` as never));

  const chosen = rows.filter((r) => selected.has(r.key));
  const downloads = chosen.filter((r) => r.kind !== "extra");
  const removals = chosen.filter((r) => r.kind === "extra");
  const downloadBytes = downloads.reduce((n, r) => n + r.item.size, 0);
  const replacing = downloads.filter((r) => r.kind === "different").length;

  const toggle = (key: string) =>
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  const setMany = (list: Row[], on: boolean) =>
    setSelected((s) => {
      const next = new Set(s);
      for (const r of list) {
        if (on) next.add(r.key);
        else next.delete(r.key);
      }
      return next;
    });

  const sync = async () => {
    setError("");
    setBusy(true);
    setProgress(null);
    const plan: LanPlan = {
      backup,
      download: downloads.map((r) => ({ hostPath: r.item.path, hash: r.item.hash, size: r.item.size, dest: r.localPath ?? r.item.path })),
      remove: removals.map((r) => r.item.path),
    };
    try {
      setReport(await backend.lanSync(instance.id, target.trim(), plan));
      setPage("done");
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const answer = (ip: string, allow: boolean) => {
    void backend.lanHostAnswer(ip, allow, allow && remember).then(() => backend.lanHostStatus().then(setStatus)).catch(() => undefined);
  };

  const indicator = CLIENT_PAGES.includes(page) && (
    <>
      <div className="dots" aria-hidden="true">
        {CLIENT_PAGES.map((p, i) => <i key={p} className={i <= CLIENT_PAGES.indexOf(page) ? "on" : ""} />)}
      </div>
      <small className="status">{t("derive.step", { n: CLIENT_PAGES.indexOf(page) + 1, total: CLIENT_PAGES.length })}</small>
    </>
  );

  const section = (kind: Kind, title: string) => {
    const list = visible.filter((r) => r.kind === kind);
    if (list.length === 0) return null;
    const allOn = list.every((r) => selected.has(r.key));
    return (
      <div key={kind}>
        <div className="lan-head">
          <h4>{title} <span className="pill">{list.length}</span></h4>
          <button type="button" className="ghost small" onClick={() => setMany(list, !allOn)}>{allOn ? t("lan.clearVisible") : t("lan.selectVisible")}</button>
        </div>
        <div className="lan-list">
          {list.slice(0, 400).map((r) => (
            <div className="lan-row" key={r.key}>
              <label className="check">
                <input type="checkbox" checked={selected.has(r.key)} onChange={() => toggle(r.key)} />
                <span className="name" title={r.item.path}>{r.item.path}</span>
              </label>
              <span className="mono">{size(r.item.size)}</span>
            </div>
          ))}
          {list.length > 400 && <small className="status">+ {list.length - 400}</small>}
        </div>
      </div>
    );
  };

  const scanBar = busy && (
    <div>
      <div className="progress"><i style={{ width: `${scan.total > 0 ? Math.max(4, Math.round((scan.done / scan.total) * 100)) : 4}%` }} /></div>
      <small className="status">{waiting ? t("lan.waitingHost") : scan.total > 0 ? t("hash.scanning", { done: scan.done, total: scan.total }) : t("common.working")}</small>
    </div>
  );

  const percent = progress && progress.total > 0 ? Math.round((progress.done / progress.total) * 100) : 0;
  const versionDiffers = !!diff && !!diff.hostVersion && !!diff.localVersion && diff.hostVersion !== diff.localVersion;
  const nothingToDo = !!diff && diff.missing.length + diff.different.length + diff.extra.length === 0;

  let body: React.ReactNode = null;

  if (page === "role") {
    body = (
      <>
        <h2>{t("lan.roleTitle")}</h2>
        <div className="choices">
          <button type="button" className="choice" onClick={() => void switchRole("host")}>
            <b>{t("lan.roleHost")}</b>
            <small>{t("lan.roleHostHint")}</small>
          </button>
          <button type="button" className="choice" onClick={() => void switchRole("client")}>
            <b>{t("lan.roleClient")}</b>
            <small>{t("lan.roleClientHint")}</small>
          </button>
        </div>
        {error && <p className="error" role="alert">{error}</p>}
        <div className="actions"><button type="button" className="ghost" onClick={onClose}>{t("common.close")}</button></div>
      </>
    );
  } else if (page === "host") {
    body = (
      <>
        <h2>{t("lan.hostTitle")}</h2>
        {!hostInfo && <p>{t("lan.hostIntro", { name: instance.name })}</p>}
        {!hostInfo && busy && (
          <div>
            <div className="progress"><i style={{ width: `${scan.total > 0 ? Math.max(4, Math.round((scan.done / scan.total) * 100)) : 4}%` }} /></div>
            <small className="status">{scan.total > 0 ? t("lan.hostScanning", { done: scan.done, total: scan.total }) : t("common.working")}</small>
          </div>
        )}
        {hostInfo && (
          <>
            <p className="hash-ok"><Icon name="check" size={16} /> {t("lan.hostRunning")} · {t("lan.hostInfo", { files: hostInfo.files, size: size(hostInfo.bytes) })}</p>
            {status?.pending.map((p) => (
              <div className="derive-opt lan-ask" key={p.ip} role="alert">
                <b>{t("lan.askTitle", { name: p.name || t("lan.unknownPc") })}</b>
                <small className="status">{t("lan.askBody", { name: p.name || t("lan.unknownPc"), ip: p.ip })}</small>
                <label className="check">
                  <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
                  <span>{t("lan.remember")}</span>
                </label>
                <div className="actions left">
                  <button type="button" className="play small-play" onClick={() => answer(p.ip, true)}>{t("lan.allow")}</button>
                  <button type="button" className="ghost" onClick={() => answer(p.ip, false)}>{t("lan.deny")}</button>
                </div>
              </div>
            ))}
            <div className="field">
              <span>{t("lan.address")}</span>
              {hostInfo.addresses.map((a) => (
                <div className="lan-addr" key={a.ip}>
                  <code>{a.ip}</code>
                  {a.name && <small className="status">{a.name}</small>}
                  <button type="button" className="ghost small" onClick={() => void copy(a.ip, a.ip)}>{copied === a.ip ? t("hash.copied") : t("hash.copy")}</button>
                </div>
              ))}
            </div>
            <small className="status">{t("lan.hostListen", { listen: hostInfo.listen })}</small>
            <small className="status">{t("lan.hostClients", { n: status?.clients ?? 0 })} · {t("lan.hostServed", { size: size(status?.servedBytes ?? 0) })}</small>
            {status && status.trusted.length > 0 && (
              <div className="field">
                <span>{t("lan.trustedTitle")}</span>
                {status.trusted.map((ip) => (
                  <div className="lan-addr" key={ip}>
                    <code>{ip}</code>
                    <button type="button" className="ghost small" onClick={() => void backend.lanHostForget(ip).then(() => backend.lanHostStatus().then(setStatus))}>{t("lan.forget")}</button>
                  </div>
                ))}
              </div>
            )}
            <small className="status">{t("lan.hostFirewall")}</small>
            <small className="status">{t("lan.hostExit")}</small>
          </>
        )}
        {error && <p className="error" role="alert">{error}</p>}
        <div className="actions">
          <button type="button" className="link-btn" onClick={() => void switchRole("client")}>{t("lan.notHost")}</button>
          <button type="button" className="ghost" onClick={onClose}>{t("common.close")}</button>
        </div>
      </>
    );
  } else if (page === "connect") {
    const showForm = editing && !busy;
    body = (
      <>
        <h2>{t("lan.connectTitle")}</h2>
        {busy && <p>{t("lan.connecting", { ip: (addr || saved).trim() })}</p>}
        {scanBar}
        {showForm && (
          <>
            <p>{t("lan.connectHint")}</p>
            <label className="field">
              <span>{t("lan.addrLabel")}</span>
              <input value={addr} placeholder="192.168.1.130" spellCheck={false} autoFocus onChange={(e) => setAddr(e.target.value)} onKeyDown={(e) => e.key === "Enter" && addr.trim() && void connect(addr, saveIp && addr.trim() !== saved)} />
            </label>
            <label className="check">
              <input type="checkbox" checked={saveIp} onChange={(e) => setSaveIp(e.target.checked)} />
              <span>{t("lan.saveIp")}</span>
            </label>
          </>
        )}
        {!showForm && !busy && saved && (
          <small className="status">{t("lan.savedIp", { ip: saved })} · <button type="button" className="link-btn" onClick={() => setEditing(true)}>{t("lan.changeIp")}</button></small>
        )}
        {error && <p className="error" role="alert">{error}</p>}
        <div className="actions">
          <button type="button" className="link-btn" disabled={busy} onClick={() => void switchRole("host")}>{t("lan.notClient")}</button>
          <button type="button" className="ghost" disabled={busy} onClick={onClose}>{t("common.close")}</button>
          {showForm && <button type="button" className="play" disabled={!addr.trim()} onClick={() => void connect(addr, saveIp && addr.trim() !== saved)}>{t("lan.connect")}</button>}
          {!showForm && !busy && saved && <button type="button" className="play" onClick={() => void connect(saved, false)}>{t("lan.connect")}</button>}
        </div>
      </>
    );
  } else if (page === "review" && diff) {
    body = (
      <>
        <h2>{t("lan.reviewTitle", { name: diff.hostName })}</h2>
        {versionDiffers && <p className="status warn">{t("lan.versionDiff", { host: diff.hostVersion ?? "", mine: diff.localVersion ?? "" })}</p>}
        {nothingToDo ? (
          <p className="hash-ok"><Icon name="check" size={16} /> {t("lan.same")}</p>
        ) : (
          <>
            <div className="lan-chips">
              {chips.map((g) => (
                <button key={g} type="button" className="lan-chip" aria-pressed={filter === g} onClick={() => setFilter(g)}>{groupLabel(g)} · {counts(g)}</button>
              ))}
            </div>
            {section("missing", t("lan.missing"))}
            {section("different", t("lan.changed"))}
            {section("extra", t("lan.extra"))}
            <small className="status">{t("lan.totals", { n: downloads.length, size: size(downloadBytes), m: removals.length })}</small>
          </>
        )}
        <div className="actions">
          <button type="button" className="ghost" onClick={() => { setEditing(false); setPage("connect"); auto.current = true; }}>{t("common.back")}</button>
          {nothingToDo ? (
            <button type="button" className="play" onClick={onClose}>{t("common.close")}</button>
          ) : (
            <button type="button" className="play" disabled={chosen.length === 0} onClick={() => setPage("confirm")}>{t("common.next")}</button>
          )}
        </div>
      </>
    );
  } else if (page === "confirm") {
    body = (
      <>
        <h2>{t("lan.confirmTitle")}</h2>
        <p>{t("lan.confirmBody", { add: downloads.length - replacing, rep: replacing, del: removals.length, size: size(downloadBytes) })}</p>
        <label className="check derive-opt">
          <input type="checkbox" checked={backup} disabled={busy} onChange={(e) => setBackup(e.target.checked)} />
          <span>
            <b>{t("lan.backup")}</b>
            <small className="status block">{t("lan.backupHint")}</small>
          </span>
        </label>
        {!backup && <p className="error" role="alert">{t("lan.noBackupWarn")}</p>}
        {busy && (
          <div>
            <div className="progress"><i style={{ width: `${Math.max(3, percent)}%` }} /></div>
            <small className="status">
              {waiting
                ? t("lan.waitingHost")
                : progress?.phase === "apply"
                  ? t("lan.progressApply", { done: progress.done, total: progress.total })
                  : t("lan.progressDownload", { done: size(progress?.done ?? 0), total: size(progress?.total ?? downloadBytes) })}
            </small>
          </div>
        )}
        {error && <p className="error" role="alert">{error}</p>}
        <div className="actions">
          <button type="button" className="ghost" disabled={busy} onClick={() => setPage("review")}>{t("common.back")}</button>
          <button type="button" className="play" disabled={busy} onClick={() => void sync()}>{busy ? t("common.working") : t("lan.sync")}</button>
        </div>
      </>
    );
  } else if (page === "done" && report) {
    body = (
      <>
        <h2>{t("lan.doneTitle")}</h2>
        <p className="hash-ok"><Icon name="check" size={16} /> {t("lan.doneBody", { added: report.added, replaced: report.replaced, removed: report.removed, size: size(report.bytes) })}</p>
        {report.backupDir && <small className="status">{t("lan.backupAt", { path: report.backupDir })}</small>}
        <div className="actions">
          <button type="button" className="ghost" onClick={() => void backend.openFolder(instance.id, "root").catch(() => undefined)}>{t("lan.openFolder")}</button>
          <button type="button" className="play" onClick={onClose}>{t("common.close")}</button>
        </div>
      </>
    );
  }

  const dialog = (
    <div className="dialog wide derive" role="dialog" aria-modal="true">
      <div className="dialog-body" key={page}>
        <h2 className="notice-title"><Icon name="shield" size={20} /> {t("lan.title", { name: instance.name })}</h2>
        {indicator}
        {body}
      </div>
    </div>
  );

  if (bare) return dialog;

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !busy && page !== "confirm" && onClose()}>
      {dialog}
    </div>
  );
}
