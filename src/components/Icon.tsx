import type { ReactNode } from "react";

const PATHS: Record<string, ReactNode> = {
  mods: <><path d="M21 8l-9-5-9 5v8l9 5 9-5z" /><path d="M3 8l9 5 9-5" /><path d="M12 13v8" /></>,
  saves: <><path d="M5 3h11l3 3v15H5z" /><path d="M8 3v6h8V3" /><path d="M8 21v-7h8v7" /></>,
  tray: <><path d="M22 12h-6l-2 3h-4l-2-3H2" /><path d="M5.5 5h13l3.5 7v6a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-6z" /></>,
  size: <><path d="M22 12H2" /><path d="M5.5 5h13l3.5 7v6a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-6z" /><path d="M6 16h.01" /><path d="M10 16h.01" /></>,
  time: <><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></>,
  code: <><path d="M4 9h16" /><path d="M4 15h16" /><path d="M10 3L8 21" /><path d="M16 3l-2 18" /></>,
  link: <><path d="M10 13a5 5 0 0 0 7.07 0l3-3a5 5 0 0 0-7.07-7.07l-1.5 1.5" /><path d="M14 11a5 5 0 0 0-7.07 0l-3 3a5 5 0 0 0 7.07 7.07l1.5-1.5" /></>,
  unlink: <><path d="M18.84 12.25l1.72-1.71a5 5 0 0 0-7.07-7.07l-1.72 1.71" /><path d="M5.17 11.75l-1.71 1.71a5 5 0 0 0 7.07 7.07l1.71-1.71" /><path d="M8 2v3" /><path d="M2 8h3" /><path d="M16 22v-3" /><path d="M22 16h-3" /></>,
  version: <><path d="M20.6 13.4l-7.2 7.2a2 2 0 0 1-2.8 0L3 13V3h10l7.6 7.6a2 2 0 0 1 0 2.8z" /><path d="M7.5 7.5h.01" /></>,
  played: <><circle cx="12" cy="12" r="9" /><path d="M10 8l6 4-6 4z" /></>,
  copies: <><path d="M3 4h18v4H3z" /><path d="M5 8v12h14V8" /><path d="M10 12h4" /></>,
  cache: <><path d="M3 5c0-1.7 4-3 9-3s9 1.3 9 3-4 3-9 3-9-1.3-9-3z" /><path d="M3 5v14c0 1.7 4 3 9 3s9-1.3 9-3V5" /><path d="M3 12c0 1.7 4 3 9 3s9-1.3 9-3" /></>,
  edit: <><path d="M12 20h9" /><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z" /></>,
  notes: <><path d="M15.5 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V8.5z" /><path d="M15 3v6h6" /></>,
  music: <><circle cx="12" cy="12" r="10" /><g transform="translate(6.5 6.5) scale(.46)"><path d="M9 18V6l10-2v12" /><circle cx="6" cy="18" r="3" /><circle cx="16" cy="16" r="3" /></g></>,
  note: <><path d="M9 18V6l10-2v12" /><circle cx="6" cy="18" r="3" /><circle cx="16" cy="16" r="3" /></>,
  warn: <><path d="M10.3 3.9L1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" /><path d="M12 9v4" /><path d="M12 17h.01" /></>,
  check: <path d="M20 6L9 17l-5-5" />,
  prev: <><path d="M19 20L9 12l10-8z" /><path d="M5 19V5" /></>,
  next: <><path d="M5 4l10 8-10 8z" /><path d="M19 5v14" /></>,
  volume: <><path d="M11 5L6 9H2v6h4l5 4z" /><path d="M15.5 8.5a5 5 0 0 1 0 7" /><path d="M18.5 5.5a9 9 0 0 1 0 13" /></>,
  shield: <><path d="M12 3l8 3v6c0 4.5-3.2 7.8-8 9-4.8-1.2-8-4.5-8-9V6z" /><path d="M9 12l2 2 4-4" /></>,
  spinner: <path d="M21 12a9 9 0 1 1-6.2-8.55" />,
  folder: <path d="M3 6a2 2 0 0 1 2-2h4l2 3h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />,
  trash: <><path d="M3 6h18" /><path d="M8 6V4h8v2" /><path d="M6 6l1 15h10l1-15" /><path d="M10 11v6" /><path d="M14 11v6" /></>,
  play: <path d="M7 4l13 8-13 8z" />,
  pause: <><path d="M7 4h3v16H7z" /><path d="M14 4h3v16h-3z" /></>,
  plus: <><path d="M12 5v14" /><path d="M5 12h14" /></>,
  close: <><path d="M6 6l12 12" /><path d="M18 6L6 18" /></>,
};

export default function Icon({ name, size = 16, className }: { name: string; size?: number; className?: string }) {
  return (
    <svg className={"icon" + (className ? " " + className : "")} viewBox="0 0 24 24" width={size} height={size} fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      {PATHS[name]}
    </svg>
  );
}
