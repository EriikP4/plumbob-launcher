import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { assetUrl } from "../lib/assets";
import { pickAudioFiles } from "../lib/native";
import type { Genre, Instance, Track } from "../lib/types";
import Icon from "./Icon";

const fileName = (p: string) => p.replace(/^.*[\\/]/, "");
const title = (n: string) => n.replace(/\.[^.]+$/, "");

const hue = (s: string) => {
  let h = 0;
  for (let i = 0; i < s.length; i += 1) h = (h * 31 + s.charCodeAt(i)) % 360;
  return h;
};

const clock = (s: number) => {
  if (!Number.isFinite(s) || s < 0) return "0:00";
  const m = Math.floor(s / 60);
  return `${m}:${String(Math.floor(s % 60)).padStart(2, "0")}`;
};

function readVolume() {
  try {
    const v = Number(localStorage.getItem("music.volume"));
    return Number.isFinite(v) && v >= 0 && v <= 1 && localStorage.getItem("music.volume") !== null ? v : 0.8;
  } catch {
    return 0.8;
  }
}

function AddDialog({ instance, genres, initial, onClose, onDone }: { instance: Instance; genres: string[]; initial: string; onClose: () => void; onDone: (genre: string) => void }) {
  const { t, msg } = useSettings();
  const [genre, setGenre] = useState(initial || genres[0] || "");
  const [files, setFiles] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState({ done: 0, total: 1 });
  const [error, setError] = useState("");

  useEffect(() => {
    const off = backend.onMusicProgress((p) => setProgress({ done: p.done, total: Math.max(1, p.total) }));
    return () => void off.then((f) => f());
  }, []);

  const choose = async () => {
    const picked = await pickAudioFiles();
    if (picked.length) setFiles((cur) => [...cur, ...picked.filter((p) => !cur.includes(p))]);
  };

  const add = async () => {
    setError("");
    setBusy(true);
    setProgress({ done: 0, total: 1 });
    try {
      await backend.addMusic(instance.id, genre, files);
      onDone(genre);
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
      setBusy(false);
    }
  };

  const percent = Math.min(100, Math.round((progress.done / progress.total) * 100));

  return (
    <div className="overlay stacked" onMouseDown={(e) => e.target === e.currentTarget && !busy && onClose()}>
      <div className="dialog wide music-add" role="dialog" aria-modal="true">
        <div className="dialog-body">
          <h2 className="notice-title"><Icon name="note" size={22} /> {t("music.addTitle")}</h2>
          <label className="field">
            <span>{t("music.genre")}</span>
            <select value={genre} disabled={busy} onChange={(e) => setGenre(e.target.value)}>
              {genres.map((g) => <option key={g} value={g}>{g}</option>)}
            </select>
          </label>
          <button type="button" className="drop-box" disabled={busy} onClick={choose}>
            <span className="drop-icon"><Icon name="plus" size={26} /></span>
            <b>{files.length ? t("music.addMore") : t("music.pickFiles")}</b>
            <small>mp3 · wav · flac · ogg · m4a</small>
          </button>
          {files.length > 0 && (
            <div className="picked-list">
              {files.map((f) => (
                <div className="picked" key={f}>
                  <Icon name="note" size={14} />
                  <span>{title(fileName(f))}</span>
                  <button type="button" className="ghost small" disabled={busy} aria-label={t("music.remove")} onClick={() => setFiles(files.filter((x) => x !== f))}><Icon name="close" size={12} /></button>
                </div>
              ))}
            </div>
          )}
          {busy && (
            <>
              <div className="progress" role="progressbar" aria-valuenow={percent}><i style={{ width: `${percent}%` }} /></div>
              <small className="status">{t("music.copying", { done: progress.done, total: progress.total })}</small>
            </>
          )}
          {error && <p className="error" role="alert">{error}</p>}
          <div className="actions">
            <button type="button" className="ghost" disabled={busy} onClick={onClose}>{t("common.cancel")}</button>
            <button type="button" className="play" disabled={busy || files.length === 0 || !genre} onClick={add}>{t("music.add")}{files.length ? ` (${files.length})` : ""}</button>
          </div>
        </div>
      </div>
    </div>
  );
}

function Player({ audio, track, genre, isPlaying, onToggle, onPrev, onNext }: {
  audio: React.RefObject<HTMLAudioElement | null>;
  track: Track | null;
  genre: string;
  isPlaying: boolean;
  onToggle: () => void;
  onPrev: () => void;
  onNext: () => void;
}) {
  const { t } = useSettings();
  const [time, setTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [volume, setVolume] = useState(readVolume);

  useEffect(() => {
    const el = audio.current;
    if (!el) return;
    const tick = () => setTime(el.currentTime);
    const meta = () => setDuration(el.duration);
    el.addEventListener("timeupdate", tick);
    el.addEventListener("loadedmetadata", meta);
    el.addEventListener("durationchange", meta);
    return () => {
      el.removeEventListener("timeupdate", tick);
      el.removeEventListener("loadedmetadata", meta);
      el.removeEventListener("durationchange", meta);
    };
  }, [audio]);

  useEffect(() => {
    if (audio.current) audio.current.volume = volume;
    try {
      localStorage.setItem("music.volume", String(volume));
    } catch {}
  }, [volume, audio]);

  const seek = (v: number) => {
    if (audio.current && Number.isFinite(audio.current.duration)) audio.current.currentTime = v;
    setTime(v);
  };

  const fill = (v: number, max: number) => ({ ["--fill" as string]: `${max ? (v / max) * 100 : 0}%` });

  return (
    <div className={"player" + (isPlaying ? " live" : "")}>
      <div className="player-info">
        <span className="player-art" style={{ ["--h" as string]: hue(genre || "x") }}><Icon name="note" size={20} /></span>
        <span className="player-text">
          <b>{track ? title(track.name) : t("music.nothing")}</b>
          <small>{track ? genre : t("music.pickSong")}</small>
        </span>
      </div>
      <div className="player-center">
        <div className="player-buttons">
          <button type="button" className="pbtn" aria-label={t("music.prev")} disabled={!track} onClick={onPrev}><Icon name="prev" size={18} /></button>
          <button type="button" className="pbtn main" aria-label={t("music.play")} disabled={!track} onClick={onToggle}><Icon name={isPlaying ? "pause" : "play"} size={20} /></button>
          <button type="button" className="pbtn" aria-label={t("music.next")} disabled={!track} onClick={onNext}><Icon name="next" size={18} /></button>
        </div>
        <div className="player-seek">
          <span className="mono">{clock(time)}</span>
          <input type="range" className="range" min={0} max={duration || 0} step={0.1} value={Math.min(time, duration || 0)} disabled={!track} style={fill(time, duration)} onChange={(e) => seek(Number(e.target.value))} />
          <span className="mono">{clock(duration)}</span>
        </div>
      </div>
      <div className="player-volume">
        <Icon name="volume" size={16} />
        <input type="range" className="range" min={0} max={1} step={0.01} value={volume} style={fill(volume, 1)} aria-label={t("music.volume")} onChange={(e) => setVolume(Number(e.target.value))} />
      </div>
    </div>
  );
}

export default function MusicDialog({ instance, onClose, onCount }: { instance: Instance; onClose: () => void; onCount?: (n: number) => void }) {
  const { t, msg, size, settings, save } = useSettings();
  const [welcome, setWelcome] = useState(!settings.musicWelcomeSeen);
  const [genres, setGenres] = useState<Genre[] | null>(null);
  const [all, setAll] = useState<string[]>([]);
  const [selected, setSelected] = useState("");
  const [adding, setAdding] = useState(false);
  const [playing, setPlaying] = useState<{ track: Track; genre: string } | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState("");
  const audio = useRef<HTMLAudioElement>(null);
  const queue = useRef<Track[]>([]);

  const load = useCallback(async (pick?: string) => {
    const list = await backend.listMusic(instance.id);
    setGenres(list);
    setSelected((cur) => pick && list.some((g) => g.name === pick) ? pick : list.some((g) => g.name === cur) ? cur : list[0]?.name ?? "");
  }, [instance.id]);

  useEffect(() => {
    backend.musicGenres().then(setAll).catch(() => undefined);
    load().catch(() => setGenres([]));
  }, [load]);

  const current = genres?.find((g) => g.name === selected) ?? null;

  useEffect(() => {
    if (genres) onCount?.(genres.reduce((n, g) => n + g.tracks.length, 0));
  }, [genres, onCount]);

  const start = useCallback((track: Track, genre: string, tracks: Track[]) => {
    queue.current = tracks;
    setPlaying({ track, genre });
  }, []);

  useEffect(() => {
    const el = audio.current;
    if (playing && el) {
      el.src = assetUrl(playing.track.path);
      void el.play().catch(() => undefined);
    }
  }, [playing]);

  const toggle = () => {
    const el = audio.current;
    if (!el || !playing) return;
    if (el.paused) void el.play();
    else el.pause();
  };

  const step = useCallback((dir: 1 | -1) => {
    if (!playing) return;
    const list = queue.current;
    const i = list.findIndex((x) => x.path === playing.track.path);
    const next = list[i + dir];
    if (next) setPlaying({ track: next, genre: playing.genre });
    else if (dir === 1) {
      setIsPlaying(false);
    }
  }, [playing]);

  const clickTrack = (track: Track) => {
    if (playing?.track.path === track.path) return toggle();
    if (current) start(track, current.name, current.tracks);
  };

  const remove = async (track: Track) => {
    if (confirm !== track.path) return setConfirm(track.path);
    setConfirm("");
    setError("");
    if (playing?.track.path === track.path) {
      audio.current?.pause();
      setPlaying(null);
    }
    try {
      await backend.deleteTrack(instance.id, track.path);
      await load();
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
    }
  };

  const finishWelcome = () => {
    setWelcome(false);
    void save({ ...settings, musicWelcomeSeen: true });
  };

  const list = all.length ? all : (genres ?? []).map((g) => g.name);

  if (welcome) {
    return (
      <div className="overlay">
        <div className="dialog wide music-welcome" role="dialog" aria-modal="true">
          <div className="dialog-body">
            <span className="welcome-art" aria-hidden="true"><Icon name="note" size={40} /></span>
            <h2>{t("music.welcomeTitle")}</h2>
            <p>{t("music.welcomeBody")}</p>
            <div className="actions"><button type="button" className="play" onClick={finishWelcome}>{t("music.thanks")}</button></div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !adding && onClose()}>
      <div className="dialog music-dialog" role="dialog" aria-modal="true">
        <header className="music-head">
          <span className="music-badge"><Icon name="note" size={20} /></span>
          <div className="music-head-text">
            <h2>{t("music.heading")}</h2>
            <small className="status">{instance.name}</small>
          </div>
          <button type="button" className="play small-play" onClick={() => setAdding(true)}><Icon name="plus" size={15} /> {t("music.addMusic")}</button>
          <button type="button" className="ghost" onClick={onClose}>{t("common.close")}</button>
        </header>
        {genres === null && <div className="music-empty"><p>{t("common.working")}</p></div>}
        {genres && genres.length === 0 && (
          <div className="music-empty">
            <span className="welcome-art" aria-hidden="true"><Icon name="note" size={40} /></span>
            <p>{t("music.empty")}</p>
            <button type="button" className="play" onClick={() => setAdding(true)}>{t("music.addMusic")}</button>
          </div>
        )}
        {genres && genres.length > 0 && (
          <div className="music-body">
            <aside className="music-genres">
              {genres.map((g) => (
                <button key={g.name} type="button" className={"genre" + (g.name === selected ? " on" : "")} onClick={() => { setSelected(g.name); setConfirm(""); }}>
                  <span className="genre-art" style={{ ["--h" as string]: hue(g.name) }}>{g.name.slice(0, 1)}</span>
                  <span className="genre-text">
                    <b>{g.name}</b>
                    <small>{t("music.count", { n: g.tracks.length })}</small>
                  </span>
                </button>
              ))}
            </aside>
            <section className="music-tracks">
              <div className="music-tracks-head">
                <h3>{current?.name}</h3>
                <small className="status">{t("music.count", { n: current?.tracks.length ?? 0 })}</small>
              </div>
              <div className="track-list">
                {current?.tracks.map((tr, i) => {
                  const active = playing?.track.path === tr.path;
                  return (
                    <div key={tr.path} className={"track" + (active ? " on" : "")} onDoubleClick={() => clickTrack(tr)}>
                      <button type="button" className="track-play" aria-label={t("music.play")} onClick={() => clickTrack(tr)}>
                        {active && isPlaying ? <span className="eq" aria-hidden="true"><i /><i /><i /></span> : <span className="num">{i + 1}</span>}
                        <span className="hover-icon"><Icon name={active && isPlaying ? "pause" : "play"} size={14} /></span>
                      </button>
                      <span className="track-name">{title(tr.name)}</span>
                      <span className="mono">{size(tr.bytes)}</span>
                      <button type="button" className={"track-del" + (confirm === tr.path ? " danger" : "")} aria-label={t("music.delete")} onClick={() => void remove(tr)}>
                        {confirm === tr.path ? t("info.sure") : <Icon name="trash" size={14} />}
                      </button>
                    </div>
                  );
                })}
              </div>
              {error && <p className="error" role="alert">{error}</p>}
              <small className="status">{t("music.deleteNote")}</small>
            </section>
          </div>
        )}
        <audio
          ref={audio}
          onPlay={() => setIsPlaying(true)}
          onPause={() => setIsPlaying(false)}
          onEnded={() => step(1)}
        />
        {genres && genres.length > 0 && (
          <Player audio={audio} track={playing?.track ?? null} genre={playing?.genre ?? ""} isPlaying={isPlaying} onToggle={toggle} onPrev={() => step(-1)} onNext={() => step(1)} />
        )}
      </div>
      {adding && createPortal(
        <AddDialog
          instance={instance}
          genres={list}
          initial={selected}
          onClose={() => setAdding(false)}
          onDone={(genre) => {
            setAdding(false);
            void load(genre);
          }}
        />,
        document.body,
      )}
    </div>
  );
}
