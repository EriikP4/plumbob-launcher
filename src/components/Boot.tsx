import { useEffect, useRef, useState } from "react";
import { useSettings } from "../lib/settings";
import { playChime } from "../lib/chime";
import HeroArt from "./HeroArt";
import { appVersion } from "../lib/native";

let shown = false;

export default function Boot() {
  const { settings, t } = useSettings();
  const enabled = !shown && settings.boot.animation && settings.theme.animations;
  const [phase, setPhase] = useState<"in" | "out" | "done">(enabled ? "in" : "done");
  const [version, setVersion] = useState("");
  const stop = useRef<() => void>(() => undefined);

  useEffect(() => {
    if (!enabled) return;
    shown = true;
    void appVersion().then(setVersion);
    if (settings.boot.sound) stop.current = playChime();
    const out = setTimeout(() => setPhase("out"), 4200);
    const done = setTimeout(() => setPhase("done"), 4900);
    return () => {
      clearTimeout(out);
      clearTimeout(done);
    };
  }, []);

  const skip = () => {
    stop.current();
    setPhase("done");
  };

  if (phase === "done") return null;
  return (
    <div className={"boot" + (phase === "out" ? " out" : "")} onClick={skip} role="presentation">
      <HeroArt dense colors={["color-mix(in srgb, var(--accent) 45%, #0b0714)", "color-mix(in srgb, var(--accent) 12%, #0b0714)"]} />
      <div className="boot-shade" />
      <div className="boot-in">
        <span className="tag">{t("app.tagline")}</span>
        <h1>Plumbob Launcher</h1>
      </div>
      <div className="boot-welcome"><span>{t("boot.welcome")}</span></div>
      {version && <small className="boot-version">v{version}</small>}
    </div>
  );
}
