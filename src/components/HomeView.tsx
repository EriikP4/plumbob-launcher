import { useState } from "react";
import { useSettings } from "../lib/settings";
import { compareVersions } from "../lib/format";
import type { Instance } from "../lib/types";
import Icon from "./Icon";
import InstanceThumb from "./InstanceThumb";

const GREETINGS = 10;

export default function HomeView({ instances, latest, onOpen, onPlay }: { instances: Instance[]; latest: string | null; onOpen: (i: Instance) => void; onPlay: (i: Instance) => void }) {
  const { t, when, duration } = useSettings();
  const [greeting] = useState(() => Math.floor(Math.random() * GREETINGS));
  const ready = instances.filter((i) => !i.legacy);
  const played = ready.filter((i) => i.playSeconds > 0);
  const last = [...played].sort((a, b) => b.lastPlayedAt - a.lastPlayedAt)[0] ?? ready[0] ?? null;
  const total = ready.reduce((n, i) => n + i.playSeconds, 0);
  const outdated = latest ? ready.filter((i) => i.gameVersion && compareVersions(i.gameVersion, latest) < 0) : [];
  const pending = instances.filter((i) => i.legacy);
  const old = ready.filter((i) => i.trayOld);
  const notices = [
    ...outdated.map((i) => ({ key: "v" + i.id, text: t("home.noticeVersion", { name: i.name, old: i.gameVersion ?? "", new: latest ?? "" }), go: i })),
    ...old.map((i) => ({ key: "t" + i.id, text: t("home.noticeTray", { name: i.name }), go: i })),
    ...pending.map((i) => ({ key: "p" + i.id, text: t("home.noticePending", { name: i.name }), go: i })),
  ];

  return (
    <main className="scroll home">
      <section className="hero home-hero">
        <div className="in">
          <span className="tag">{t("home.tag")}</span>
          <h1>{t(`home.greet.${greeting}` as never)}</h1>
          <p>{t("home.sub")}</p>
        </div>
      </section>

      <div className="stats home-stats">
        <div className="stat"><label><Icon name="saves" size={13} /> {t("home.statInstances")}</label><b>{ready.length}</b></div>
        <div className="stat"><label><Icon name="time" size={13} /> {t("home.statPlaytime")}</label><b>{duration(total)}</b></div>
        <div className="stat"><label><Icon name="mods" size={13} /> {t("home.statMods")}</label><b>{ready.reduce((n, i) => n + i.modCount, 0)}</b></div>
      </div>

      <div className="cols">
        <section className="card">
          <h2>{t("home.continue")}</h2>
          {last ? (
            <div className="continue">
              <InstanceThumb instance={last} />
              <div className="continue-info">
                <b>{last.name}</b>
                <small className="status block">{played.length > 0 ? t("home.lastPlayed", { when: when(last.lastPlayedAt) }) : t("home.neverPlayed")} · {t("home.modsCount", { n: last.modCount })}</small>
              </div>
              <div className="continue-actions">
                <button type="button" className="ghost" onClick={() => onOpen(last)}>{t("home.open")}</button>
                <button type="button" className="play small-play" disabled={last.legacy} onClick={() => onPlay(last)}>{t("home.play")}</button>
              </div>
            </div>
          ) : (
            <p>{t("main.empty")}</p>
          )}
        </section>

        <section className="card">
          <h2>{t("home.notices")}</h2>
          <div className="card-list">
            {notices.length === 0 && <div className="row"><span>{t("home.allGood")}</span><span className="pill ok">OK</span></div>}
            {notices.map((n) => (
              <div className="row" key={n.key}>
                <span>{n.text}</span>
                <button type="button" className="ghost small" onClick={() => onOpen(n.go)}>{t("home.open")}</button>
              </div>
            ))}
          </div>
        </section>
      </div>
    </main>
  );
}
