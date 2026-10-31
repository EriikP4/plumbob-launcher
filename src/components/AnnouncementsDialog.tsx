import { useState } from "react";
import { createPortal } from "react-dom";
import { useSettings } from "../lib/settings";
import { openLink } from "../lib/native";
import { httpsUrl, renderRich } from "../lib/richtext";
import type { Announcement } from "../lib/types";
import Icon from "./Icon";

const ICONS = { info: "info", update: "download", event: "time", warning: "warn" } as const;
const FOLD_CHARS = 260;
const FOLD_LINES = 5;

interface Props {
  items: Announcement[];
  read: string[];
  onRead: (ids: string[]) => void;
  onClose: () => void;
}

function Item({ item, unread, onRead }: { item: Announcement; unread: boolean; onRead: (id: string) => void }) {
  const { t, settings } = useSettings();
  const [open, setOpen] = useState(false);
  const foldable = item.body.length > FOLD_CHARS || item.body.split("\n").length > FOLD_LINES;
  const url = item.link ? httpsUrl(item.link.url) : null;
  const toggle = () => {
    setOpen((v) => !v);
    if (unread) onRead(item.id);
  };
  const date = new Date(`${item.date}T12:00:00`);
  return (
    <article className={`news-item kind-${item.kind}` + (unread ? " unread" : "")}>
      <span className="news-icon" aria-hidden="true"><Icon name={ICONS[item.kind]} size={18} /></span>
      <div className="news-main">
        <div className="news-head">
          <b>{item.title}</b>
          {item.pinned && <span className="pill">{t("news.pinned")}</span>}
          {unread && <span className="pill ok">{t("news.new")}</span>}
        </div>
        <small className="status block">{Number.isNaN(date.getTime()) ? item.date : date.toLocaleDateString(settings.language, { day: "numeric", month: "long", year: "numeric" })} · {t(`news.kind.${item.kind}`)}</small>
        {item.body && <div className={"news-body" + (foldable && !open ? " folded" : "")}>{renderRich(item.body)}</div>}
        <div className="news-actions">
          {foldable && <button type="button" className="link-btn" onClick={toggle}>{open ? t("news.less") : t("news.more")}</button>}
          {url && item.link && <button type="button" className="ghost small" onClick={() => { if (unread) onRead(item.id); void openLink(url); }}>{item.link.label ?? t("news.open")}</button>}
        </div>
      </div>
    </article>
  );
}

export default function AnnouncementsDialog({ items, read, onRead, onClose }: Props) {
  const { t } = useSettings();
  const unread = items.filter((i) => !read.includes(i.id));
  return createPortal(
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="dialog wide news-dialog" role="dialog" aria-modal="true">
        <div className="dialog-body">
          <h2 className="notice-title"><Icon name="news" size={22} /> {t("news.title")}</h2>
          <div className="news-list">
            {items.length === 0 && <p>{t("news.empty")}</p>}
            {items.map((item) => (
              <Item key={item.id} item={item} unread={!read.includes(item.id)} onRead={(id) => onRead([id])} />
            ))}
          </div>
          <div className="actions">
            <button type="button" className="ghost" disabled={unread.length === 0} onClick={() => onRead(unread.map((i) => i.id))}>{t("news.markAll")}</button>
            <button type="button" className="play" onClick={onClose}>{t("common.close")}</button>
          </div>
        </div>
      </div>
    </div>,
    document.body,
  );
}
