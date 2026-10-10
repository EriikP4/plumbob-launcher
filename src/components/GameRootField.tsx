import { useEffect, useRef, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import type { RootCheck } from "../lib/types";
import PathField from "./PathField";

interface Props {
  value: string | null;
  onCommit: (path: string | null) => void;
}

export default function GameRootField({ value, onCommit }: Props) {
  const { t } = useSettings();
  const [text, setText] = useState(value ?? "");
  const [check, setCheck] = useState<RootCheck | null>(null);
  const seq = useRef(0);

  useEffect(() => {
    const path = text.trim();
    const id = ++seq.current;
    if (!path) {
      setCheck(null);
      return;
    }
    const timer = window.setTimeout(() => {
      backend.checkGameRoot(path).then((c) => {
        if (id !== seq.current) return;
        setCheck(c);
        if (c.state === "ok") {
          if (path !== value) onCommit(path);
        }
      }).catch(() => undefined);
    }, 250);
    return () => window.clearTimeout(timer);
  }, [text]);

  return (
    <div className="game-root">
      <PathField label={t("root.label")} hint={t("root.hint")} folder value={text} onChange={setText} />
      {!text.trim() && <small className="status warn">{t("root.required")}</small>}
      {check?.state === "ok" && <small className="status ok-text">{check.version ? t("root.okVersion", { version: check.version }) : t("root.ok")}</small>}
      {check?.state === "inside" && check.root && (
        <small className="status warn">
          {t("root.inside", { root: check.root })}{" "}
          <button type="button" className="ghost small" onClick={() => setText(check.root!)}>{t("root.use")}</button>
        </small>
      )}
      {check?.state === "invalid" && <small className="status warn">{t("root.invalid")}</small>}
    </div>
  );
}
