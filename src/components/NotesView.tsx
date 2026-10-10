import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { renderMarkdown, toggleTask } from "../lib/markdown";
import { openLink, pickSavePath, pickTextFiles } from "../lib/native";
import type { Instance } from "../lib/types";

interface Note {
  id: string;
  title: string;
  body: string;
  created: number;
  updated: number;
  pinned: boolean;
}

type Mode = "edit" | "split" | "preview";

const now = () => Math.floor(Date.now() / 1000);
const newId = () => Date.now().toString(36) + Math.random().toString(36).slice(2, 7);
const clean = (s: string) => s.replace(/\r\n?/g, "\n");

const makeNote = (title = "", body = ""): Note => ({ id: newId(), title, body: clean(body), created: now(), updated: now(), pinned: false });

const displayTitle = (n: Note, fallback: string) => {
  if (n.title.trim()) return n.title.trim();
  const first = n.body.split("\n").find((l) => l.trim());
  return first ? first.replace(/^[#>\-*\s[\]x]+/i, "").slice(0, 60) || fallback : fallback;
};

const snippet = (n: Note) =>
  n.body.replace(/```[\s\S]*?```/g, " ").replace(/[#>*_`~[\]-]/g, "").replace(/\s+/g, " ").trim().slice(0, 80);

const safeName = (s: string) => s.replace(/[\\/:*?"<>|]+/g, "_").trim().slice(0, 60) || "nota";

function parseStored(json: string, legacy: string, fallbackTitle: string): Note[] {
  try {
    const data = JSON.parse(json) as unknown;
    if (Array.isArray(data)) {
      return data
        .filter((n): n is Note => !!n && typeof n === "object" && typeof (n as Note).id === "string")
        .map((n) => ({ id: n.id, title: String(n.title ?? ""), body: clean(String(n.body ?? "")), created: Number(n.created) || now(), updated: Number(n.updated) || now(), pinned: !!n.pinned }));
    }
  } catch {}
  return legacy.trim() ? [makeNote(fallbackTitle, legacy)] : [];
}

function fromImport(name: string, text: string): Note[] {
  if (name.toLowerCase().endsWith(".json")) {
    try {
      const data = JSON.parse(text) as unknown;
      const list = Array.isArray(data) ? data : (data as { notes?: unknown })?.notes;
      if (Array.isArray(list)) {
        return list
          .filter((n) => n && typeof n === "object")
          .map((n: Partial<Note>) => ({ ...makeNote(String(n.title ?? ""), String(n.body ?? "")), created: Number(n.created) || now(), updated: Number(n.updated) || now(), pinned: !!n.pinned }));
      }
    } catch {
      return [];
    }
    return [];
  }
  const base = name.replace(/^.*[\\/]/, "").replace(/\.[^.]+$/, "");
  const heading = /^#\s+(.+)$/m.exec(text);
  return [makeNote(heading && !base ? heading[1] : base || heading?.[1] || "", text)];
}

export default function NotesDialog({ instance, onClose }: { instance: Instance; onClose: () => void }) {
  const { t, when } = useSettings();
  const tRef = useRef(t);
  tRef.current = t;
  const [notes, setNotes] = useState<Note[] | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [mode, setMode] = useState<Mode>("split");
  const [query, setQuery] = useState("");
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [saved, setSaved] = useState(true);
  const [message, setMessage] = useState("");
  const area = useRef<HTMLTextAreaElement>(null);
  const pendingSel = useRef<[number, number] | null>(null);
  const latest = useRef<Note[] | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const dirty = useRef(false);

  useEffect(() => {
    backend.getNotes(instance.id).then(({ json, legacy }) => {
      const list = parseStored(json, legacy, tRef.current("notes.defaultTitle"));
      setNotes(list);
      setSelected(list[0]?.id ?? null);
      if (!json && legacy.trim()) dirty.current = true;
    }).catch(() => setNotes([]));
  }, [instance.id]);

  const write = useCallback(async () => {
    window.clearTimeout(timer.current);
    timer.current = undefined;
    if (!dirty.current || !latest.current) return;
    dirty.current = false;
    await backend.saveNotes(instance.id, JSON.stringify(latest.current));
    setSaved(true);
  }, [instance.id]);

  useEffect(() => {
    latest.current = notes;
    if (notes === null || !dirty.current) return;
    setSaved(false);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => void write().catch(() => undefined), 500);
  }, [notes, write]);

  useEffect(() => () => void write().catch(() => undefined), [write]);

  useLayoutEffect(() => {
    if (pendingSel.current && area.current) {
      area.current.focus();
      area.current.setSelectionRange(pendingSel.current[0], pendingSel.current[1]);
      pendingSel.current = null;
    }
  });

  const update = (id: string, patch: Partial<Note>, touch = true) => {
    dirty.current = true;
    setNotes((list) => (list ? list.map((n) => (n.id === id ? { ...n, ...patch, updated: touch ? now() : n.updated } : n)) : list));
  };

  const ordered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (notes ?? [])
      .filter((n) => !q || n.title.toLowerCase().includes(q) || n.body.toLowerCase().includes(q))
      .sort((a, b) => Number(b.pinned) - Number(a.pinned) || b.updated - a.updated);
  }, [notes, query]);

  const current = notes?.find((n) => n.id === selected) ?? null;

  const addNote = (note = makeNote()) => {
    dirty.current = true;
    setNotes((list) => [note, ...(list ?? [])]);
    setSelected(note.id);
    setQuery("");
    setConfirmDelete(false);
  };

  const removeNote = () => {
    if (!current) return;
    if (!confirmDelete) return setConfirmDelete(true);
    dirty.current = true;
    const rest = (notes ?? []).filter((n) => n.id !== current.id);
    setNotes(rest);
    setSelected(rest[0]?.id ?? null);
    setConfirmDelete(false);
  };

  const flash = (text: string) => {
    setMessage(text);
    window.setTimeout(() => setMessage(""), 3500);
  };

  const exportOne = async () => {
    if (!current) return;
    const path = await pickSavePath(`${safeName(displayTitle(current, t("notes.untitled")))}.md`, "md");
    if (!path) return;
    try {
      await backend.exportText(path, current.body);
      flash(t("notes.exported"));
    } catch (e) {
      flash(String(e));
    }
  };

  const exportAll = async () => {
    if (!notes?.length) return;
    const path = await pickSavePath(`notas-${safeName(instance.name)}.json`, "json");
    if (!path) return;
    try {
      await backend.exportText(path, JSON.stringify({ app: "plumbob-launcher", notes }, null, 2));
      flash(t("notes.exported"));
    } catch (e) {
      flash(String(e));
    }
  };

  const importFiles = async () => {
    const paths = await pickTextFiles();
    if (!paths.length) return;
    const found: Note[] = [];
    for (const p of paths) {
      try {
        found.push(...fromImport(p, await backend.readImport(p)));
      } catch {}
    }
    if (!found.length) return flash(t("notes.importNone"));
    dirty.current = true;
    setNotes((list) => [...found, ...(list ?? [])]);
    setSelected(found[0].id);
    flash(t("notes.imported", { n: found.length }));
  };

  const change = (body: string, sel?: [number, number]) => {
    if (!current) return;
    if (sel) pendingSel.current = sel;
    update(current.id, { body });
  };

  const wrap = (before: string, after = before, placeholder = "") => {
    const el = area.current;
    if (!el || !current) return;
    const { selectionStart: s, selectionEnd: e, value } = el;
    const chosen = value.slice(s, e) || placeholder;
    const next = value.slice(0, s) + before + chosen + after + value.slice(e);
    change(next, [s + before.length, s + before.length + chosen.length]);
  };

  const prefixLines = (make: (i: number) => string, strip?: RegExp) => {
    const el = area.current;
    if (!el || !current) return;
    const { selectionStart: s, selectionEnd: e, value } = el;
    const start = value.lastIndexOf("\n", s - 1) + 1;
    let end = value.indexOf("\n", e);
    if (end === -1) end = value.length;
    const lines = value.slice(start, end).split("\n");
    const stripped = lines.map((l) => (strip ? l.replace(strip, "") : l));
    const same = stripped.every((l, i) => lines[i] !== l);
    const out = (same ? stripped : stripped.map((l, i) => make(i) + l)).join("\n");
    change(value.slice(0, start) + out + value.slice(end), [start, start + out.length]);
  };

  const heading = (level: number) => {
    const marker = "#".repeat(level) + " ";
    prefixLines(() => marker, new RegExp(`^${marker}`));
  };

  const block = () => {
    const el = area.current;
    if (!el || !current) return;
    const { selectionStart: s, selectionEnd: e, value } = el;
    const chosen = value.slice(s, e) || "";
    const text = "```\n" + chosen + "\n```";
    change(value.slice(0, s) + text + value.slice(e), [s + 4, s + 4 + chosen.length]);
  };

  const insertLink = () => {
    const el = area.current;
    if (!el || !current) return;
    const { selectionStart: s, selectionEnd: e, value } = el;
    const label = value.slice(s, e) || t("notes.linkText");
    const text = `[${label}](https://)`;
    const urlStart = s + label.length + 3;
    change(value.slice(0, s) + text + value.slice(e), [urlStart, urlStart + 8]);
  };

  const insertText = (text: string) => {
    const el = area.current;
    if (!el || !current) return;
    const { selectionStart: s, selectionEnd: e, value } = el;
    const lead = s > 0 && value[s - 1] !== "\n" ? "\n" : "";
    change(value.slice(0, s) + lead + text + value.slice(e), [s + lead.length + text.length, s + lead.length + text.length]);
  };

  const onKey = (ev: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if ((ev.ctrlKey || ev.metaKey) && !ev.shiftKey) {
      const k = ev.key.toLowerCase();
      if (k === "b") { ev.preventDefault(); return wrap("**", "**", t("notes.tb.bold")); }
      if (k === "i") { ev.preventDefault(); return wrap("*", "*", t("notes.tb.italic")); }
      if (k === "k") { ev.preventDefault(); return insertLink(); }
    }
    if (ev.key === "Tab") {
      ev.preventDefault();
      const el = ev.currentTarget;
      const { selectionStart: s, selectionEnd: e, value } = el;
      if (ev.shiftKey) {
        const start = value.lastIndexOf("\n", s - 1) + 1;
        if (value.startsWith("  ", start)) change(value.slice(0, start) + value.slice(start + 2), [Math.max(start, s - 2), Math.max(start, e - 2)]);
        return;
      }
      change(value.slice(0, s) + "  " + value.slice(e), [s + 2, s + 2]);
      return;
    }
    if (ev.key === "Enter" && !ev.shiftKey && !ev.ctrlKey) {
      const el = ev.currentTarget;
      const { selectionStart: s, selectionEnd: e, value } = el;
      if (s !== e) return;
      const start = value.lastIndexOf("\n", s - 1) + 1;
      const line = value.slice(start, s);
      const m = /^(\s*)([-*+]|\d+[.)])\s+(\[( |x|X)\]\s+)?(.*)$/.exec(line);
      if (!m) return;
      ev.preventDefault();
      if (!m[5]) {
        change(value.slice(0, start) + value.slice(s), [start, start]);
        return;
      }
      const marker = /\d/.test(m[2]) ? `${parseInt(m[2], 10) + 1}${m[2].slice(-1)}` : m[2];
      const text = `\n${m[1]}${marker} ${m[3] ? "[ ] " : ""}`;
      change(value.slice(0, s) + text + value.slice(e), [s + text.length, s + text.length]);
    }
  };

  const close = async () => {
    try {
      await write();
    } catch {}
    onClose();
  };

  const words = current ? (current.body.trim() ? current.body.trim().split(/\s+/).length : 0) : 0;
  const tools: { key: string; label: string; tip: string; run: () => void; cls?: string }[] = [
    { key: "b", label: "B", tip: t("notes.tb.bold"), run: () => wrap("**", "**", t("notes.tb.bold")), cls: "b" },
    { key: "i", label: "I", tip: t("notes.tb.italic"), run: () => wrap("*", "*", t("notes.tb.italic")), cls: "i" },
    { key: "s", label: "S", tip: t("notes.tb.strike"), run: () => wrap("~~", "~~", t("notes.tb.strike")), cls: "s" },
    { key: "h1", label: "H1", tip: t("notes.tb.h1"), run: () => heading(1) },
    { key: "h2", label: "H2", tip: t("notes.tb.h2"), run: () => heading(2) },
    { key: "h3", label: "H3", tip: t("notes.tb.h3"), run: () => heading(3) },
    { key: "ul", label: "•", tip: t("notes.tb.ul"), run: () => prefixLines(() => "- ", /^[-*+]\s+/) },
    { key: "ol", label: "1.", tip: t("notes.tb.ol"), run: () => prefixLines((i) => `${i + 1}. `, /^\d+[.)]\s+/) },
    { key: "task", label: "☑", tip: t("notes.tb.task"), run: () => prefixLines(() => "- [ ] ", /^[-*+]\s+\[( |x|X)\]\s+/) },
    { key: "quote", label: "❝", tip: t("notes.tb.quote"), run: () => prefixLines(() => "> ", /^>\s?/) },
    { key: "code", label: "</>", tip: t("notes.tb.code"), run: () => wrap("`", "`", "code") },
    { key: "block", label: "{ }", tip: t("notes.tb.block"), run: block },
    { key: "link", label: "🔗", tip: t("notes.tb.link"), run: insertLink },
    { key: "hr", label: "—", tip: t("notes.tb.hr"), run: () => insertText("\n---\n") },
    { key: "table", label: "▦", tip: t("notes.tb.table"), run: () => insertText("| A | B |\n| --- | --- |\n|   |   |\n") },
  ];

  const preview = current ? renderMarkdown(current.body, {
    onToggle: (line) => update(current.id, { body: toggleTask(current.body, line) }),
    onLink: (url) => void openLink(url),
  }) : null;

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && void close()}>
      <div className="dialog notes-dialog" role="dialog" aria-modal="true">
        <header className="notes-head">
          <h2>{t("notes.title", { name: instance.name })}</h2>
          <span className="status">{message || (saved ? t("notes.saved") : t("notes.saving"))}</span>
          <button type="button" className="ghost" onClick={() => void close()}>{t("common.close")}</button>
        </header>
        <div className="notes-body">
          <aside className="notes-side">
            <div className="notes-side-top">
              <input type="search" value={query} placeholder={t("notes.search")} onChange={(e) => setQuery(e.target.value)} />
              <button type="button" className="play" onClick={() => addNote()}>+</button>
            </div>
            <div className="notes-list">
              {notes === null && <div className="row"><span className="mono">{t("common.working")}</span></div>}
              {notes && ordered.length === 0 && <p className="status">{query ? t("notes.noMatch") : t("notes.empty")}</p>}
              {ordered.map((n) => (
                <button key={n.id} type="button" className={"note-item" + (n.id === selected ? " on" : "")} onClick={() => { setSelected(n.id); setConfirmDelete(false); }}>
                  <b>{n.pinned && <i className="pin" aria-hidden="true">★</i>}{displayTitle(n, t("notes.untitled"))}</b>
                  <small>{snippet(n) || "—"}</small>
                  <span className="mono">{when(n.updated)}</span>
                </button>
              ))}
            </div>
            <div className="notes-side-actions">
              <button type="button" className="ghost small" onClick={() => void importFiles()}>{t("notes.import")}</button>
              <button type="button" className="ghost small" disabled={!notes?.length} onClick={() => void exportAll()}>{t("notes.exportAll")}</button>
            </div>
          </aside>
          <section className="notes-main">
            {!current && (
              <div className="notes-blank">
                <p>{t("notes.pick")}</p>
                <button type="button" className="play" onClick={() => addNote()}>{t("notes.new")}</button>
              </div>
            )}
            {current && (
              <>
                <div className="notes-title-row">
                  <input className="notes-title" value={current.title} placeholder={t("notes.titlePlaceholder")} maxLength={120} onChange={(e) => update(current.id, { title: e.target.value })} />
                  <button type="button" className="ghost" aria-pressed={current.pinned} data-tip={t("notes.pin")} onClick={() => update(current.id, { pinned: !current.pinned }, false)}>{current.pinned ? "★" : "☆"}</button>
                  <div className="seg" role="group">
                    {(["edit", "split", "preview"] as Mode[]).map((m) => (
                      <button key={m} type="button" aria-pressed={mode === m} onClick={() => setMode(m)}>{t(`notes.mode.${m}`)}</button>
                    ))}
                  </div>
                </div>
                {mode !== "preview" && (
                  <div className="md-toolbar" role="toolbar">
                    {tools.map((b) => (
                      <button key={b.key} type="button" className={"tb " + (b.cls ?? "")} title={b.tip} aria-label={b.tip} onMouseDown={(e) => e.preventDefault()} onClick={b.run}>{b.label}</button>
                    ))}
                  </div>
                )}
                <div className={"notes-panes " + mode}>
                  {mode !== "preview" && (
                    <textarea ref={area} className="notes-editor" value={current.body} spellCheck onKeyDown={onKey} placeholder={t("notes.placeholder")} onChange={(e) => update(current.id, { body: clean(e.target.value) })} />
                  )}
                  {mode !== "edit" && <div className="md-preview">{current.body.trim() ? preview : <p className="status">{t("notes.previewEmpty")}</p>}</div>}
                </div>
                <footer className="notes-foot">
                  <span className="status">{t("notes.words", { n: words })} · {t("notes.chars", { n: current.body.length })} · {t("notes.updated", { when: when(current.updated) })}</span>
                  <span className="head-actions">
                    <button type="button" className="ghost small" onClick={() => void exportOne()}>{t("notes.exportOne")}</button>
                    <button type="button" className={"ghost small" + (confirmDelete ? " danger" : "")} onClick={removeNote}>{confirmDelete ? t("info.sure") : t("info.delete")}</button>
                  </span>
                </footer>
              </>
            )}
          </section>
        </div>
      </div>
    </div>
  );
}
