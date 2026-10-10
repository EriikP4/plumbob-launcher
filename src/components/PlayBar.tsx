import { useState } from "react";
import { useSettings } from "../lib/settings";
import type { Instance, LaunchMode } from "../lib/types";
import Icon from "./Icon";

interface Props {
  instance: Instance;
  playing: boolean;
  mode: LaunchMode;
  onMode: (m: LaunchMode) => void;
  onToggle: () => void;
  onCancel?: () => void;
  waiting?: boolean;
  warn?: boolean;
}

export default function PlayBar({ instance, playing, mode, onMode, onToggle, onCancel, waiting, warn }: Props) {
  const { t, settings, save } = useSettings();
  const [ripple, setRipple] = useState<{ x: number; y: number; id: number } | null>(null);
  const [pressed, setPressed] = useState(false);
  const press = (e: React.MouseEvent<HTMLButtonElement>) => {
    if (playing) return;
    const r = e.currentTarget.getBoundingClientRect();
    setRipple({ x: e.clientX - r.left, y: e.clientY - r.top, id: Date.now() });
    setPressed(true);
    window.setTimeout(() => setPressed(false), 520);
    onToggle();
  };
  const s4mp = settings.multiplayer.enabled && instance.kind === "mp";
  const sub = playing
    ? t("play.live")
    : mode === "s4mp" && s4mp
      ? t("play.readyS4mp")
      : t("play.ready", { platform: t(`platform.${settings.platform}`) });
  return (
    <div className={"bar" + (playing ? " live" : "")}>
      <div className="status">
        <b>{playing ? t("play.nowPlaying", { name: instance.name }) : instance.name}</b>
        {!playing && <span className="mono">{sub}</span>}
      </div>
      {settings.s4mm.enabled && (
        <div className="s4mm-controls">
          <label className="check">
            <input type="checkbox" checked={settings.s4mm.launchWithGame} onChange={(e) => void save({ ...settings, s4mm: { ...settings.s4mm, launchWithGame: e.target.checked } })} />
            <span>{t("main.s4mmWith")}</span>
          </label>
        </div>
      )}
      {s4mp && (
        <div className="seg" role="group" aria-label={t("mode.aria")}>
          <button type="button" aria-pressed={mode === "offline"} disabled={playing} onClick={() => onMode("offline")}>{t("mode.offline")}</button>
          <button type="button" aria-pressed={mode === "s4mp"} disabled={playing} onClick={() => onMode("s4mp")}>{t("mode.s4mp")}</button>
        </div>
      )}
      {waiting && onCancel && <button className="ghost" type="button" onClick={onCancel}>{t("play.cancel")}</button>}
      <button className={"play" + (pressed ? " pressed" : "")} type="button" disabled={playing} data-tip={playing ? t("play.live") : undefined} aria-label={playing ? `${t("play.playing")} ${t("play.live")}` : undefined} onClick={press}>
        {ripple && !playing && <span key={ripple.id} className="ripple" style={{ left: ripple.x, top: ripple.y }} onAnimationEnd={() => setRipple(null)} />}
        {playing ? <>{waiting && <i className="spin" aria-hidden="true" />}{t("play.playing")}</> : <>{warn && <Icon name="warn" size={17} className="play-warn" />}{t("play.play")}</>}
      </button>
    </div>
  );
}
