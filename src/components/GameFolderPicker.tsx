import { useEffect, useRef, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import type { GameFolderCheck } from "../lib/types";
import PathField from "./PathField";

interface Props {
  label: string;
  hint?: string;
  placeholder?: string;
  onValid: (check: GameFolderCheck) => void;
}

export default function GameFolderPicker({ label, hint, placeholder, onValid }: Props) {
  const { t } = useSettings();
  const [text, setText] = useState("");
  const [check, setCheck] = useState<GameFolderCheck | null>(null);
  const seq = useRef(0);

  useEffect(() => {
    const path = text.trim();
    const id = ++seq.current;
    if (!path) {
      setCheck(null);
      return;
    }
    const timer = window.setTimeout(() => {
      backend.checkGameFolder(path).then((c) => {
        if (id !== seq.current) return;
        setCheck(c);
        if (c.state === "ok") onValid(c);
      }).catch(() => undefined);
    }, 250);
    return () => window.clearTimeout(timer);
  }, [text]);

  return (
    <div className="game-root">
      <PathField label={label} hint={hint} placeholder={placeholder} folder value={text} onChange={setText} />
      {check?.state === "ok" && <small className="status ok-text">{t("gamefolder.ok", { name: check.name ?? "" })}</small>}
      {check?.state === "invalid" && <small className="status warn">{t("gamefolder.invalid")}</small>}
    </div>
  );
}
