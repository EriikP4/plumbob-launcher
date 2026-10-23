import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { assetUrl } from "../lib/assets";
import type { Tool } from "../lib/types";
import Icon from "./Icon";
import ToolDialog from "./ToolDialog";

export default function ToolsPopover({ anchor, onClose }: { anchor: HTMLElement | null; onClose: () => void }) {
  const { settings, t, msg } = useSettings();
  const [editing, setEditing] = useState<Tool | "new" | null>(null);
  const [error, setError] = useState("");
  const [top, setTop] = useState(80);
  const ref = useRef<HTMLDivElement>(null);
  const left = anchor ? anchor.getBoundingClientRect().right + 14 : 80;

  useLayoutEffect(() => {
    if (!anchor || !ref.current) return;
    const y = anchor.getBoundingClientRect().top - 10;
    setTop(Math.max(12, Math.min(y, window.innerHeight - ref.current.offsetHeight - 12)));
  }, [anchor, settings.tools.length]);

  useEffect(() => {
    if (editing) return;
    const down = (e: MouseEvent) => {
      const target = e.target as Node;
      if (ref.current?.contains(target) || anchor?.contains(target)) return;
      onClose();
    };
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    document.addEventListener("mousedown", down);
    document.addEventListener("keydown", key);
    return () => {
      document.removeEventListener("mousedown", down);
      document.removeEventListener("keydown", key);
    };
  }, [anchor, onClose, editing]);

  const launch = (tool: Tool) => {
    setError("");
    backend.launchTool(tool.path).catch((e) => setError(msg(e instanceof Error ? e.message : String(e))));
  };

  const tools = settings.tools;
  return createPortal(
    <>
      <div ref={ref} className="tools-pop" style={{ left, top, visibility: editing ? "hidden" : "visible" }} role="dialog" aria-label={t("tools.title")}>
        <header className="tools-head">
          <h2>{t("tools.title")}</h2>
          <button type="button" className="mini-btn tip below right" data-tip={t("tools.add")} aria-label={t("tools.add")} onClick={() => setEditing("new")}><Icon name="plus" size={15} /></button>
        </header>
        {tools.length === 0 ? (
          <div className="tools-empty">
            <Icon name="wrench" size={26} />
            <b>{t("tools.empty")}</b>
            <small>{t("tools.emptyHint")}</small>
            <button type="button" className="ghost" onClick={() => setEditing("new")}>{t("tools.add")}</button>
          </div>
        ) : (
          <div className="tools-grid">
            {tools.map((tool) => (
              <div key={tool.id} className="tool-wrap">
                <button type="button" className="tool-tile" onClick={() => launch(tool)} onContextMenu={() => setEditing(tool)}>
                  <span className="tool-icon">{tool.icon ? <img src={assetUrl(tool.icon)} alt="" /> : tool.name[0]?.toUpperCase()}</span>
                  <span className="tool-name">{tool.name}</span>
                </button>
                <button type="button" className="tool-edit mini-btn" aria-label={t("tools.edit")} onClick={() => setEditing(tool)}><Icon name="edit" size={12} /></button>
              </div>
            ))}
          </div>
        )}
        {error && <small className="error">{error}</small>}
      </div>
      {editing && <ToolDialog tool={editing === "new" ? null : editing} onClose={() => setEditing(null)} />}
    </>,
    document.body,
  );
}
