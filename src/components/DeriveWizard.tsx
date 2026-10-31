import { useEffect, useMemo, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import type { BaseScan, GameOptions, Instance, ModEntry, ModGroupId } from "../lib/types";
import InstanceForm from "./InstanceForm";
import GameOptionsFields, { RECOMMENDED } from "./GameOptionsFields";

type Page = "base" | "what" | "mods" | "tree" | "final";

interface Props {
  instances: Instance[];
  onClose: () => void;
  onBack: () => void;
  onDone: () => void;
}

function ModTree({ code, picked, sizes, onToggle }: { code: string; picked: Set<string>; sizes: Map<string, number>; onToggle: (e: ModEntry, ancestor: boolean) => void }) {
  const { t, size } = useSettings();
  const [children, setChildren] = useState<Record<string, ModEntry[]>>({});
  const [open, setOpen] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState<Set<string>>(new Set(["" ]));

  const load = (rel: string) => {
    setLoading((s) => new Set(s).add(rel));
    backend
      .listModDir(code, rel)
      .catch(() => [] as ModEntry[])
      .then((list) => {
        setChildren((c) => ({ ...c, [rel]: list }));
        setLoading((s) => {
          const next = new Set(s);
          next.delete(rel);
          return next;
        });
      });
  };

  useEffect(() => {
    load("");
  }, [code]);

  const toggleOpen = (rel: string) => {
    setOpen((s) => {
      const next = new Set(s);
      if (next.has(rel)) next.delete(rel);
      else {
        next.add(rel);
        if (!children[rel]) load(rel);
      }
      return next;
    });
  };

  const hasAncestor = (rel: string) => {
    const parts = rel.split("/");
    for (let i = 1; i < parts.length; i++) if (picked.has(parts.slice(0, i).join("/"))) return true;
    return false;
  };

  const rows = (rel: string, depth: number): React.ReactNode[] => {
    const list = children[rel];
    if (!list) return loading.has(rel) ? [<div key={rel + "…"} className="tree-row" style={{ paddingLeft: 8 + depth * 18 }}><span className="mono">{t("common.working")}</span></div>] : [];
    if (list.length === 0 && rel === "") return [<div key="empty" className="tree-row"><span className="mono">{t("derive.treeEmpty")}</span></div>];
    return list.flatMap((e) => {
      const ancestor = hasAncestor(e.rel);
      const checked = ancestor || picked.has(e.rel);
      const node = (
        <div key={e.rel} className="tree-row" style={{ paddingLeft: 8 + depth * 18 }}>
          {e.isDir ? (
            <button type="button" className="tree-toggle" aria-label={e.name} aria-expanded={open.has(e.rel)} onClick={() => toggleOpen(e.rel)}>{open.has(e.rel) ? "▾" : "▸"}</button>
          ) : (
            <span className="tree-toggle" />
          )}
          <label className="check">
            <input type="checkbox" checked={checked} disabled={ancestor} onChange={() => onToggle(e, ancestor)} />
            <span className="name">{e.isDir ? "📁 " : ""}{e.name}</span>
          </label>
          <span className="mono">{e.isDir ? t("derive.groupStats", { files: e.files, size: size(e.bytes) }) : size(e.bytes)}</span>
        </div>
      );
      return open.has(e.rel) && e.isDir ? [node, ...rows(e.rel, depth + 1)] : [node];
    });
  };

  const total = [...picked].reduce((n, rel) => n + (sizes.get(rel) ?? 0), 0);
  return (
    <>
      <div className="mod-tree">{rows("", 0)}</div>
      <small className="status">{t("derive.treeSelected", { n: picked.size, size: size(total) })}</small>
    </>
  );
}

export default function DeriveWizard({ instances, onClose, onBack, onDone }: Props) {
  const { t, size, settings } = useSettings();
  const bases = instances.filter((i) => !i.legacy);
  const [page, setPage] = useState<Page>("base");
  const [baseId, setBaseId] = useState<string | null>(null);
  const [scan, setScan] = useState<BaseScan | null>(null);
  const [scanning, setScanning] = useState(false);
  const [config, setConfig] = useState(false);
  const [mods, setMods] = useState(false);
  const [allMods, setAllMods] = useState(false);
  const [groups, setGroups] = useState<Set<ModGroupId>>(new Set());
  const [manual, setManual] = useState(false);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [sizes] = useState<Map<string, number>>(() => new Map());
  const [gameOptions, setGameOptions] = useState<GameOptions>(RECOMMENDED);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);

  const base = bases.find((i) => i.id === baseId) ?? null;

  useEffect(() => {
    if (!baseId) return;
    let live = true;
    setScan(null);
    setScanning(true);
    setConfig(false);
    setMods(false);
    setAllMods(false);
    setGroups(new Set());
    setManual(false);
    setPicked(new Set());
    sizes.clear();
    backend
      .scanBase(baseId)
      .then((s) => live && setScan(s))
      .catch(() => live && setScan({ hasConfig: false, configFiles: [], hasMods: false, modsFiles: 0, modsBytes: 0, groups: [] }))
      .finally(() => live && setScanning(false));
    return () => {
      live = false;
    };
  }, [baseId, sizes]);

  useEffect(() => {
    const off = backend.onDeriveProgress((p) => setProgress(p));
    return () => void off.then((f) => f());
  }, []);

  const pages: Page[] = useMemo(() => ["base", "what", ...(mods ? (["mods"] as Page[]) : []), ...(mods && manual ? (["tree"] as Page[]) : []), "final"], [mods, manual]);
  const index = pages.indexOf(page);

  const modsReady = allMods || groups.size > 0 || (manual && picked.size > 0);
  const canNext = (() => {
    if (page === "base") return !!baseId && !!scan && !scanning;
    if (page === "what") return config || mods;
    if (page === "mods") return allMods || groups.size > 0 || manual;
    if (page === "tree") return picked.size > 0;
    return true;
  })();

  const go = (delta: number) => {
    const next = pages[index + delta];
    if (next) setPage(next);
    else if (delta < 0) onBack();
  };

  const toggleGroup = (id: ModGroupId) => {
    setManual(false);
    setAllMods(false);
    setGroups((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleAll = () => {
    const on = !allMods;
    setAllMods(on);
    if (on) {
      setGroups(new Set());
      setManual(false);
    }
  };

  const toggleManual = () => {
    const on = !manual;
    setManual(on);
    if (on) {
      setAllMods(false);
      setGroups(new Set());
    }
  };

  const togglePath = (e: ModEntry) => {
    sizes.set(e.rel, e.bytes);
    setPicked((s) => {
      const next = new Set(s);
      if (next.has(e.rel)) next.delete(e.rel);
      else {
        for (const p of next) if (p.startsWith(e.rel + "/")) next.delete(p);
        next.add(e.rel);
      }
      return next;
    });
  };

  const groupLabel = (id: ModGroupId) => t(`derive.group.${id}`);
  const modsSummary = !mods
    ? t("derive.sumNone")
    : allMods
      ? t("derive.sumAllMods")
      : manual
        ? t("derive.sumManual", { n: picked.size })
        : [...groups].map(groupLabel).join(", ");

  const indicator = (
    <>
      <div className="dots" aria-hidden="true">
        {[...pages, "final2"].map((p, i) => <i key={p} className={i <= index ? "on" : ""} />)}
      </div>
      <small className="status">{t("derive.step", { n: index + 1, total: pages.length + 1 })}</small>
    </>
  );

  const shell = (title: string, hint: string | null, body: React.ReactNode, nextLabel = t("common.next")) => (
    <div className="dialog-body" key={page}>
      {indicator}
      <h2>{title}</h2>
      {hint && <p>{hint}</p>}
      {body}
      <div className="actions">
        <button type="button" className="ghost" onClick={() => go(-1)}>{t("common.back")}</button>
        <button type="button" className="play" disabled={!canNext} onClick={() => go(1)}>{nextLabel}</button>
      </div>
    </div>
  );

  let content: React.ReactNode = null;

  if (page === "base") {
    content = shell(
      t("derive.baseTitle"),
      t("derive.baseHint"),
      <>
        {bases.length === 0 && <small className="status">{t("derive.baseEmpty")}</small>}
        <div className="derive-list">
          {bases.map((i) => (
            <button key={i.id} type="button" className="choice" aria-pressed={baseId === i.id} onClick={() => setBaseId(i.id)}>
              <b>{i.name}</b>
              <small>{t("derive.baseStats", { mods: i.modCount, size: size(i.sizeBytes) })}</small>
            </button>
          ))}
        </div>
        {scanning && <small className="status">{t("derive.scanning")}</small>}
      </>,
    );
  } else if (page === "what") {
    content = shell(
      t("derive.whatTitle"),
      t("derive.whatHint"),
      <div className="derive-opts">
        <label className="check derive-opt">
          <input type="checkbox" checked={config} disabled={!scan?.hasConfig} onChange={(e) => setConfig(e.target.checked)} />
          <span>
            <b>{t("derive.config")}</b>
            <small className="status block">{scan?.hasConfig ? t("derive.configHint", { files: scan.configFiles.join(", ") }) : t("derive.configNone")}</small>
          </span>
        </label>
        <label className="check derive-opt">
          <input type="checkbox" checked={mods} disabled={!scan?.hasMods} onChange={(e) => setMods(e.target.checked)} />
          <span>
            <b>{t("derive.mods")}</b>
            <small className="status block">{scan?.hasMods ? t("derive.modsHint") : t("derive.modsNone")}</small>
          </span>
        </label>
      </div>,
    );
  } else if (page === "mods" && scan) {
    content = shell(
      t("derive.modsTitle"),
      t("derive.modsSub"),
      <div className="derive-opts">
        <label className="check derive-opt">
          <input type="checkbox" checked={allMods} onChange={toggleAll} />
          <span>
            <b>{t("derive.allMods")}</b>
            <small className="status block">{t("derive.allModsHint", { files: scan.modsFiles, size: size(scan.modsBytes) })}</small>
          </span>
        </label>
        {scan.groups.map((g) => (
          <label key={g.id} className="check derive-opt">
            <input type="checkbox" checked={groups.has(g.id)} onChange={() => toggleGroup(g.id)} />
            <span>
              <b>{groupLabel(g.id)}</b>
              <small className="status block">
                {t("derive.groupStats", { files: g.files, size: size(g.bytes) })}
                {g.id === "translations" ? " · " + t("derive.translationsHint") : ""}
              </small>
            </span>
          </label>
        ))}
        <label className="check derive-opt">
          <input type="checkbox" checked={manual} onChange={toggleManual} />
          <span>
            <b>{t("derive.manual")}</b>
            <small className="status block">{t("derive.manualHint")}</small>
          </span>
        </label>
      </div>,
    );
  } else if (page === "tree" && baseId) {
    content = shell(
      t("derive.treeTitle"),
      t("derive.treeHint"),
      <ModTree code={baseId} picked={picked} sizes={sizes} onToggle={togglePath} />,
    );
  } else if (page === "final" && baseId) {
    const manualBytes = [...picked].reduce((n, rel) => n + (sizes.get(rel) ?? 0), 0);
    const modsBytes = allMods ? scan?.modsBytes ?? 0 : manual ? manualBytes : (scan?.groups ?? []).filter((g) => groups.has(g.id)).reduce((n, g) => n + g.bytes, 0);
    content = (
      <>
        <InstanceForm
          steps={{ offset: pages.length - 1, total: pages.length + 1 }}
          title={t("derive.finalTitle")}
          submitLabel={t("derive.create")}
          withKind
          extra={
            <>
              <dl className="summary">
                <dt>{t("derive.sumBase")}</dt><dd>{base?.name}</dd>
                <dt>{t("derive.config")}</dt><dd>{config ? t("common.yes") : t("derive.sumNone")}</dd>
                <dt>{t("derive.mods")}</dt><dd>{modsSummary}{mods && modsReady ? ` · ${size(modsBytes)}` : ""}</dd>
              </dl>
              {config ? <p className="status warn">{t("derive.configNotice")}</p> : <GameOptionsFields value={gameOptions} onChange={setGameOptions} />}
              {progress && progress.total > 0 && progress.done < progress.total && (
                <div>
                  <div className="progress"><i style={{ width: `${Math.round((progress.done / progress.total) * 100)}%` }} /></div>
                  <small className="status">{t("derive.copying", { done: progress.done, total: progress.total })}</small>
                </div>
              )}
            </>
          }
          onSubmit={async (v) => {
            setProgress(null);
            await backend.deriveInstance({
              base: baseId,
              name: v.name,
              description: v.description,
              code: v.code,
              kind: v.kind,
              role: v.role,
              options: config ? undefined : gameOptions,
              selection: { config, allMods: mods && allMods, groups: mods && !allMods && !manual ? [...groups] : [], paths: mods && manual ? [...picked] : [] },
            });
            onDone();
          }}
          onSecondary={{ label: t("common.back"), run: () => go(-1) }}
        />
      </>
    );
  }

  return (
    <div className="overlay">
      <div className="dialog wide derive" role="dialog" aria-modal="true">
        {content}
        {page !== "final" && (
          <div className="derive-cancel">
            <button type="button" className="ghost small" onClick={onClose}>{t("common.cancel")}</button>
          </div>
        )}
      </div>
    </div>
  );
}
