import { useState } from "react";
import { createPortal } from "react-dom";
import { useSettings } from "../lib/settings";
import Icon from "./Icon";

const ARGS = [
  { arg: "-w", key: "args.windowed" },
  { arg: "-f", key: "args.fullscreen" },
  { arg: "-nointro", key: "args.nointro" },
  { arg: "-nohovereffects", key: "args.nohover" },
] as const;

interface Props {
  mode: "exe" | "copy";
  locked: boolean;
  value: string[];
  onChange: (next: string[]) => void;
  onClose: () => void;
}

export default function ArgsDialog({ mode, locked, value, onChange, onClose }: Props) {
  const { t } = useSettings();
  const [copied, setCopied] = useState("");

  const toggle = (arg: string, on: boolean) => {
    let next = value.filter((a) => a !== arg);
    if (on) {
      if (arg === "-w") next = next.filter((a) => a !== "-f");
      if (arg === "-f") next = next.filter((a) => a !== "-w");
      next = [...next, arg];
    }
    onChange(next);
  };

  const copy = async (arg: string) => {
    try {
      await navigator.clipboard.writeText(arg);
      setCopied(arg);
      window.setTimeout(() => setCopied((c) => (c === arg ? "" : c)), 1600);
    } catch {
      setCopied("");
    }
  };

  return createPortal(
    <div className="overlay stacked" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="dialog args-dialog" role="dialog" aria-modal="true">
        <div className="dialog-body">
          <div className="args-head">
            <span className="args-icon"><Icon name="wrench" size={20} /></span>
            <div>
              <h2>{t("args.title")}</h2>
              <small className="status">{mode === "copy" ? t("args.copyHint") : t("args.subtitle")}</small>
            </div>
          </div>
          <div className="args-list">
            {ARGS.map(({ arg, key }) => {
              const label = t(key);
              const off = mode === "exe" && locked;
              return (
                <div className={"args-row" + (off ? " locked tip below left" : "")} key={arg} data-tip={off ? t("args.lockedTip") : undefined}>
                  {mode === "exe" ? (
                    <label className="check">
                      <input type="checkbox" disabled={locked} checked={!locked && value.includes(arg)} onChange={(e) => toggle(arg, e.target.checked)} />
                      <span className="args-text" title={label}>{label}</span>
                    </label>
                  ) : (
                    <span className="args-text" title={label}>{label}</span>
                  )}
                  <code className="args-chip">{arg}</code>
                  {mode === "copy" && <button type="button" className="ghost small" onClick={() => void copy(arg)}>{copied === arg ? t("args.copied") : t("hash.copy")}</button>}
                </div>
              );
            })}
          </div>
          <div className="actions"><button type="button" className="play" onClick={onClose}>{t("common.close")}</button></div>
        </div>
      </div>
    </div>,
    document.body,
  );
}
