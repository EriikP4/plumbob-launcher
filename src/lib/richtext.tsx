import type { ReactNode } from "react";

const bold = (text: string, key: string): ReactNode[] =>
  text.split(/\*\*(.+?)\*\*/g).map((part, i) => (i % 2 === 1 ? <strong key={`${key}-${i}`}>{part}</strong> : part));

export function renderRich(source: string): ReactNode[] {
  const out: ReactNode[] = [];
  let paragraph: string[] = [];
  let list: string[] = [];
  const flushParagraph = () => {
    if (!paragraph.length) return;
    const key = `p${out.length}`;
    out.push(
      <p key={key}>
        {paragraph.map((line, i) => (
          <span key={i}>{i > 0 && <br />}{bold(line, `${key}-${i}`)}</span>
        ))}
      </p>,
    );
    paragraph = [];
  };
  const flushList = () => {
    if (!list.length) return;
    const key = `l${out.length}`;
    out.push(<ul key={key}>{list.map((item, i) => <li key={i}>{bold(item, `${key}-${i}`)}</li>)}</ul>);
    list = [];
  };
  for (const raw of source.replace(/\r\n?/g, "\n").split("\n")) {
    const line = raw.trimEnd();
    const item = /^\s*[-*]\s+(.*)$/.exec(line);
    const heading = /^#{1,6}\s+(.*)$/.exec(line);
    if (heading) {
      flushParagraph();
      flushList();
      out.push(<h3 key={`h${out.length}`}>{bold(heading[1], `h${out.length}`)}</h3>);
    } else if (item) {
      flushParagraph();
      list.push(item[1]);
    } else if (!line.trim()) {
      flushParagraph();
      flushList();
    } else {
      flushList();
      paragraph.push(line);
    }
  }
  flushParagraph();
  flushList();
  return out;
}

export const httpsUrl = (url: string): string | null => {
  try {
    const parsed = new URL(url);
    return parsed.protocol === "https:" && !parsed.username && !parsed.password ? parsed.href : null;
  } catch {
    return null;
  }
};
