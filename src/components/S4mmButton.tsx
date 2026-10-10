import { useEffect, useRef, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { burstAt } from "../lib/checkfx";

export default function S4mmButton({ code, legacy, disabled, onError }: { code: string; legacy: boolean; disabled: boolean; onError: (e: unknown) => void }) {
  const { t } = useSettings();
  const [running, setRunning] = useState(false);
  const [opening, setOpening] = useState(false);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    let live = true;
    const poll = () => {
      if (document.hidden) return;
      backend.s4mmRunning().then((r) => {
        if (!live) return;
        setRunning(r);
        if (r) setOpening(false);
      }).catch(() => undefined);
    };
    poll();
    const id = window.setInterval(poll, 2000);
    return () => {
      live = false;
      window.clearInterval(id);
      window.clearTimeout(timer.current);
    };
  }, []);

  const open = async (e: React.MouseEvent<HTMLButtonElement>) => {
    burstAt(e.currentTarget);
    setOpening(true);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setOpening(false), 20000);
    try {
      await backend.openS4mm(legacy ? null : code);
    } catch (err) {
      setOpening(false);
      onError(err);
    }
  };

  const label = running ? t("main.s4mmRunning") : opening ? t("main.s4mmOpening") : t("main.s4mmOpen");
  return (
    <button className={"ghost" + (running || opening ? " busy" : "")} type="button" disabled={disabled || running || opening} onClick={open}>{label}</button>
  );
}
