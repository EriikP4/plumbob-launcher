import { useRef, useState } from "react";
import { useSettings } from "../lib/settings";
import Icon from "./Icon";
import ProfileButton from "./ProfileButton";
import ToolsPopover from "./ToolsPopover";

type Target = "home" | "shots" | "archived" | "settings";

export default function IconBar({ view, onNav, updateReady, checking, onUpdate, news }: { view: string; onNav: (target: Target) => void; updateReady: boolean; checking: boolean; onUpdate: () => void; news: { unread: number; onOpen: () => void } | null }) {
  const { t } = useSettings();
  const [tools, setTools] = useState(false);
  const toolsBtn = useRef<HTMLButtonElement>(null);

  const item = (target: Target, icon: string, label: string) => (
    <button key={target} type="button" className={"bar-btn tip side" + (view === target ? " on" : "")} data-tip={label} aria-label={label} aria-pressed={view === target} onClick={() => { setTools(false); onNav(target); }}>
      <Icon name={icon} size={20} />
    </button>
  );

  return (
    <aside className="iconbar" aria-label={t("rail.aria")}>
      <div className="bar-group">
        <ProfileButton />
        <span className="bar-sep" aria-hidden="true" />
        {item("home", "home", t("rail.home"))}
        {item("shots", "image", t("rail.shots"))}
        {item("archived", "archive", t("rail.archived"))}
        <button ref={toolsBtn} type="button" className={"bar-btn tip side" + (tools ? " on" : "")} data-tip={tools ? undefined : t("rail.tools")} aria-label={t("rail.tools")} aria-expanded={tools} onClick={() => setTools((v) => !v)}>
          <Icon name="wrench" size={20} />
        </button>
      </div>
      <div className="bar-group bottom">
        {news && (
          <button type="button" className="bar-btn tip side" data-tip={news.unread > 0 ? t("news.unread", { n: news.unread }) : t("news.title")} aria-label={t("news.aria")} onClick={() => { setTools(false); news.onOpen(); }}>
            <Icon name="news" size={20} />
            {news.unread > 0 && <i className={"news-dot" + (news.unread > 1 ? " count" : "")} aria-hidden="true">{news.unread > 1 ? Math.min(news.unread, 9) : ""}</i>}
          </button>
        )}
        <button type="button" className={"bar-btn tip side" + (checking ? " busy" : "")} data-tip={updateReady ? t("appupd.available") : t("appupd.check")} aria-label={updateReady ? t("appupd.available") : t("appupd.check")} disabled={checking} onClick={() => { setTools(false); onUpdate(); }}>
          <Icon name={checking ? "spinner" : "download"} size={20} className={checking ? "spin" : undefined} />
          {updateReady && <i className="update-dot" aria-hidden="true" />}
        </button>
        {item("settings", "gear", t("rail.settings"))}
      </div>
      {tools && <ToolsPopover anchor={toolsBtn.current} onClose={() => setTools(false)} />}
    </aside>
  );
}
