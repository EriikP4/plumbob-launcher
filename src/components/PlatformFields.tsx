import { useState } from "react";
import { useSettings } from "../lib/settings";
import type { Environment, Platform, Settings } from "../lib/types";
import PathField from "./PathField";
import GameRootField from "./GameRootField";
import InfoButton from "./InfoButton";
import ArgsDialog from "./ArgsDialog";

interface Props {
  value: Settings;
  env: Environment | null;
  onChange: (next: Settings) => void;
}

const OPTIONS: Platform[] = ["steam", "ea", "epic", "custom"];

export default function PlatformFields({ value, env, onChange }: Props) {
  const { t } = useSettings();
  const [args, setArgs] = useState(false);
  const commandSet = (value.customCommand ?? "").trim() !== "";
  const detected = value.platform === "ea" ? env?.ea : value.platform === "epic" ? env?.epic : null;

  const choose = (platform: Platform) => {
    const found = platform === "ea" ? env?.ea : platform === "epic" ? env?.epic : null;
    onChange({ ...value, platform, steamId: null, installDir: found ?? (platform === value.platform ? value.installDir : null) });
  };

  return (
    <div className="platform">
      <div className="choices">
        {OPTIONS.map((p) => (
          <button key={p} type="button" className="choice" aria-pressed={value.platform === p} onClick={() => choose(p)}>
            <b>{t(`wizard.${p}`)}</b>
            <small>{t(`wizard.${p}Hint`)}</small>
          </button>
        ))}
      </div>
      {(value.platform === "ea" || value.platform === "epic") && (
        <>
          <PathField label={t("wizard.installDir")} hint={t("wizard.installDirHint")} folder value={value.installDir ?? ""} onChange={(v) => onChange({ ...value, installDir: v || null })} />
          <small className="status">{detected ? t("common.detected", { path: detected }) : t("common.notFound")}</small>
        </>
      )}
      {args && <ArgsDialog mode={value.platform === "custom" ? "exe" : "copy"} locked={value.platform === "custom" && commandSet} value={value.gameArgs ?? []} onChange={(gameArgs) => onChange({ ...value, gameArgs })} onClose={() => setArgs(false)} />}
      {value.platform === "custom" && (
        <>
        <GameRootField value={value.installDir} onCommit={(installDir) => onChange({ ...value, installDir })} />
        <PathField label={t("wizard.customCommand")} labelExtra={<InfoButton title={t("custom.infoTitle")} text={t("custom.infoBody")} />} hint={t("wizard.customHint2")} placeholder={t("wizard.customDefault")} pickLabel={t("wizard.pickExe")} extensions={["exe"]} value={value.customCommand ?? ""} onChange={(v) => onChange({ ...value, customCommand: v || null })} />
        </>
      )}
      <div className="actions left">
        <button type="button" className="ghost" onClick={() => setArgs(true)}>{t("args.button")}</button>
      </div>
    </div>
  );
}
