import type { ReactNode } from "react";

interface Ctx {
  onToggle: (line: number) => void;
  onLink: (url: string) => void;
}

const INLINE = /`([^`]+)`|\*\*(.+?)\*\*|__(.+?)__|~~(.+?)~~|\*([^*\s][^*]*?)\*|\b_([^_\s][^_]*?)_\b|\[([^\]]+)\]\(([^)\s]+)\)|(https?:\/\/[^\s<)]+)/g;

function inline(text: string, ctx: Ctx, keyBase: string): ReactNode[] {
  const out: ReactNode[] = [];
  let last = 0;
  let n = 0;
  INLINE.lastIndex = 0;
  let m: RegExpExecArray | null;
  const link = (url: string, label: ReactNode, key: string) => (
    <a key={key} href={url} onClick={(e) => { e.preventDefault(); if (/^https?:\/\//i.test(url)) ctx.onLink(url); }}>{label}</a>
  );
  while ((m = INLINE.exec(text))) {
    if (m.index > last) out.push(text.slice(last, m.index));
    const key = `${keyBase}-${n++}`;
    if (m[1] !== undefined) out.push(<code key={key}>{m[1]}</code>);
    else if (m[2] !== undefined || m[3] !== undefined) out.push(<strong key={key}>{inline(m[2] ?? m[3], ctx, key)}</strong>);
    else if (m[4] !== undefined) out.push(<del key={key}>{inline(m[4], ctx, key)}</del>);
    else if (m[5] !== undefined || m[6] !== undefined) out.push(<em key={key}>{inline(m[5] ?? m[6], ctx, key)}</em>);
    else if (m[7] !== undefined) out.push(link(m[8], inline(m[7], ctx, key), key));
    else if (m[9] !== undefined) out.push(link(m[9], m[9], key));
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

const HEADING = /^(#{1,6})\s+(.*?)\s*#*\s*$/;
const RULE = /^\s{0,3}([-*_])(\s*\1){2,}\s*$/;
const LIST = /^(\s*)([-*+]|\d+[.)])\s+(.*)$/;
const TABLE_SEP = /^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$/;

function cells(line: string): string[] {
  let l = line.trim();
  if (l.startsWith("|")) l = l.slice(1);
  if (l.endsWith("|")) l = l.slice(0, -1);
  return l.split("|").map((c) => c.trim());
}

interface Item {
  line: number;
  text: string;
  ordered: boolean;
  indent: number;
  children: Item[];
}

function buildList(items: Item[]): Item[] {
  const root: Item[] = [];
  const stack: Item[] = [];
  for (const it of items) {
    while (stack.length && stack[stack.length - 1].indent >= it.indent) stack.pop();
    if (stack.length) stack[stack.length - 1].children.push(it);
    else root.push(it);
    stack.push(it);
  }
  return root;
}

function renderList(items: Item[], ctx: Ctx, key: string): ReactNode {
  const ordered = items[0].ordered;
  const Tag = ordered ? "ol" : "ul";
  return (
    <Tag key={key}>
      {items.map((it) => {
        const task = /^\[( |x|X)\]\s+(.*)$/.exec(it.text);
        const k = `${key}-${it.line}`;
        return (
          <li key={k} className={task ? "task" : undefined}>
            {task ? (
              <label>
                <input type="checkbox" checked={task[1] !== " "} onChange={() => ctx.onToggle(it.line)} />
                <span className={task[1] !== " " ? "done" : undefined}>{inline(task[2], ctx, k)}</span>
              </label>
            ) : inline(it.text, ctx, k)}
            {it.children.length > 0 && renderList(it.children, ctx, k + "c")}
          </li>
        );
      })}
    </Tag>
  );
}

function blocks(lines: string[], offset: number, ctx: Ctx, keyBase: string): ReactNode[] {
  const out: ReactNode[] = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    const key = `${keyBase}-${i}`;
    if (!line.trim()) { i += 1; continue; }

    const fence = /^\s*(```|~~~)\s*(\S*)\s*$/.exec(line);
    if (fence) {
      const body: string[] = [];
      i += 1;
      while (i < lines.length && !lines[i].trim().startsWith(fence[1])) { body.push(lines[i]); i += 1; }
      i += 1;
      out.push(<pre key={key}><code>{body.join("\n")}</code></pre>);
      continue;
    }

    const h = HEADING.exec(line);
    if (h) {
      const Tag = `h${h[1].length}` as "h1";
      out.push(<Tag key={key}>{inline(h[2], ctx, key)}</Tag>);
      i += 1;
      continue;
    }

    if (RULE.test(line)) { out.push(<hr key={key} />); i += 1; continue; }

    if (/^\s{0,3}>/.test(line)) {
      const body: string[] = [];
      const start = i;
      while (i < lines.length && /^\s{0,3}>/.test(lines[i])) { body.push(lines[i].replace(/^\s{0,3}>\s?/, "")); i += 1; }
      out.push(<blockquote key={key}>{blocks(body, offset + start, ctx, key)}</blockquote>);
      continue;
    }

    if (LIST.test(line)) {
      const items: Item[] = [];
      while (i < lines.length) {
        const m = LIST.exec(lines[i]);
        if (!m) break;
        items.push({ line: offset + i, text: m[3], ordered: /\d/.test(m[2]), indent: m[1].replace(/\t/g, "    ").length, children: [] });
        i += 1;
      }
      out.push(renderList(buildList(items), ctx, key));
      continue;
    }

    if (line.includes("|") && i + 1 < lines.length && TABLE_SEP.test(lines[i + 1]) && lines[i + 1].includes("-")) {
      const head = cells(line);
      const rows: string[][] = [];
      i += 2;
      while (i < lines.length && lines[i].includes("|") && lines[i].trim()) { rows.push(cells(lines[i])); i += 1; }
      out.push(
        <div className="md-table" key={key}>
          <table>
            <thead><tr>{head.map((c, j) => <th key={j}>{inline(c, ctx, `${key}h${j}`)}</th>)}</tr></thead>
            <tbody>{rows.map((r, ri) => <tr key={ri}>{head.map((_, j) => <td key={j}>{inline(r[j] ?? "", ctx, `${key}r${ri}c${j}`)}</td>)}</tr>)}</tbody>
          </table>
        </div>,
      );
      continue;
    }

    const para: string[] = [];
    while (
      i < lines.length &&
      lines[i].trim() &&
      !HEADING.test(lines[i]) &&
      !RULE.test(lines[i]) &&
      !LIST.test(lines[i]) &&
      !/^\s{0,3}>/.test(lines[i]) &&
      !/^\s*(```|~~~)/.test(lines[i])
    ) { para.push(lines[i]); i += 1; }
    out.push(
      <p key={key}>
        {para.map((l, j) => (
          <span key={j}>{j > 0 && <br />}{inline(l, ctx, `${key}-${j}`)}</span>
        ))}
      </p>,
    );
  }
  return out;
}

export function renderMarkdown(source: string, ctx: Ctx): ReactNode {
  return blocks(source.replace(/\r\n?/g, "\n").split("\n"), 0, ctx, "b");
}

export function toggleTask(source: string, line: number): string {
  const lines = source.split("\n");
  const l = lines[line];
  if (l === undefined) return source;
  lines[line] = l.replace(/\[( |x|X)\]/, (_, c: string) => (c === " " ? "[x]" : "[ ]"));
  return lines.join("\n");
}
