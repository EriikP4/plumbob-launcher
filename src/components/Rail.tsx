import { useState } from "react";
import { useSettings } from "../lib/settings";
import type { Instance } from "../lib/types";

interface Props {
  instances: Instance[];
  activeId: string | null;
  selectedId: string;
  locked: boolean;
  onSelect: (i: Instance) => void;
  onAdd: () => void;
  onReorder: (ids: string[]) => void;
  onSettings: () => void;
  onShots: () => void;
  onArchived: () => void;
}

interface GroupProps {
  activeId: string | null;
  title: string | null;
  items: Instance[];
  editing: boolean;
  selectedId: string;
  locked: boolean;
  onSelect: (i: Instance) => void;
  onMove: (from: string, to: string) => void;
  pending?: boolean;
}

function Group({ title, items, editing, activeId, selectedId, locked, onSelect, onMove, pending }: GroupProps) {
  const { t, when } = useSettings();
  const [dragId, setDragId] = useState<string | null>(null);
  const [overId, setOverId] = useState<string | null>(null);

  const step = (e: React.KeyboardEvent, i: number) => {
    if (!editing || !e.altKey || (e.key !== "ArrowUp" && e.key !== "ArrowDown")) return;
    const target = items[i + (e.key === "ArrowUp" ? -1 : 1)];
    if (!target) return;
    e.preventDefault();
    onMove(items[i].id, target.id);
  };

  return (
    <>
      {title && <div className="group">{title}</div>}
      <div className="list" role="listbox">
        {items.map((d, i) => (
          <button key={d.id} type="button" role="option" aria-selected={d.id === selectedId}
            className={"item" + (pending ? " pending" : "") + (d.id === dragId ? " dragging" : "") + (d.id === overId && dragId !== d.id ? " over" : "")}
            draggable={editing && !locked}
            disabled={locked && d.id !== selectedId}
            onClick={() => onSelect(d)}
            onKeyDown={(e) => step(e, i)}
            onDragStart={(e) => { setDragId(d.id); e.dataTransfer.effectAllowed = "move"; e.dataTransfer.setData("text/plain", d.id); }}
            onDragOver={(e) => { if (dragId) { e.preventDefault(); setOverId(d.id); } }}
            onDrop={(e) => { e.preventDefault(); if (dragId && dragId !== d.id) onMove(dragId, d.id); setDragId(null); setOverId(null); }}
            onDragEnd={() => { setDragId(null); setOverId(null); }}>
            {editing && <span className="grip" aria-hidden="true">⋮⋮</span>}
            <span className="thumb" style={{ background: `linear-gradient(135deg,${d.colors[0]},${d.colors[1]})` }}>{d.name[0]}</span>
            <span className="txt"><b>{d.name}</b><span className="sub">{d.modCount} mods · {when(d.lastPlayedAt)}</span></span>
            {d.id === activeId && <span className="active-dot" title={t("main.active")} aria-label={t("main.active")} />}
          </button>
        ))}
      </div>
    </>
  );
}

export default function Rail(props: Props) {
  const { t, settings } = useSettings();
  const [editing, setEditing] = useState(false);
  const multiplayer = settings.multiplayer.enabled;

  const added = props.instances.filter((i) => !i.legacy);
  const pending = props.instances.filter((i) => i.legacy);
  const groups: { title: string | null; items: Instance[] }[] = multiplayer
    ? [
        { title: t("rail.solo"), items: added.filter((i) => i.kind === "solo") },
        { title: t("rail.mp"), items: added.filter((i) => i.kind === "mp") },
      ]
    : [{ title: null, items: added }];

  const move = (index: number) => (from: string, to: string) => {
    const ids = groups[index].items.map((i) => i.id);
    const a = ids.indexOf(from);
    const b = ids.indexOf(to);
    const [moved] = ids.splice(a, 1);
    ids.splice(b, 0, moved);
    props.onReorder([...groups.flatMap((g, i) => (i === index ? ids : g.items.map((x) => x.id))), ...pending.map((x) => x.id)]);
  };

  return (
    <aside className="rail" aria-label={t("rail.aria")}>
      <div className="brand">
        <span className="plumbwrap"><i className="plumb" /></span>
        <div>Plumbob Launcher<small>{t("app.tagline")}</small></div>
      </div>
      {groups.map((g, i) => (
        <Group key={i} title={g.title} items={g.items} editing={editing} activeId={props.activeId} selectedId={props.selectedId} locked={props.locked} onSelect={props.onSelect} onMove={move(i)} />
      ))}
      {pending.length > 0 && (
        <Group title={t("rail.legacy")} items={pending} editing={false} activeId={props.activeId} selectedId={props.selectedId} locked={props.locked} onSelect={props.onSelect} onMove={() => undefined} pending />
      )}
      <div className="rail-actions">
        <button className="add" type="button" onClick={props.onAdd}>{t("rail.add")}</button>
        <button className={"ghost" + (editing ? " on" : "")} type="button" aria-pressed={editing} onClick={() => setEditing((v) => !v)}>{editing ? t("rail.done") : t("rail.edit")}</button>
        <button className="ghost" type="button" onClick={props.onShots}>{t("rail.shots")}</button>
        <button className="ghost" type="button" onClick={props.onArchived}>{t("rail.archived")}</button>
        <button className="ghost" type="button" onClick={props.onSettings}>{t("rail.settings")}</button>
      </div>
    </aside>
  );
}
