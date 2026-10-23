import { useRef, useState } from "react";
import { useSettings } from "../lib/settings";
import Icon from "./Icon";
import ProfileButton from "./ProfileButton";
import ToolsPopover from "./ToolsPopover";

type Target = "home" | "shots" | "archived" | "settings";

export default function IconBar({ view, onNav }: { view: string; onNav: (target: Target) => void }) {
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
        {item("settings", "gear", t("rail.settings"))}
      </div>
      {tools && <ToolsPopover anchor={toolsBtn.current} onClose={() => setTools(false)} />}
    </aside>
  );
}
