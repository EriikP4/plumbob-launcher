import { useState } from "react";
import { useSettings } from "../lib/settings";
import type { GameFolderCheck } from "../lib/types";
import GameFolderPicker from "./GameFolderPicker";

export default function GameFolderDialog({ onDone }: { onDone: () => void }) {
  const { t, settings, save } = useSettings();
  const [picked, setPicked] = useState<GameFolderCheck | null>(null);
  const [busy, setBusy] = useState(false);

  const apply = async () => {
    if (!picked?.root || !picked.name) return;
    setBusy(true);
    try {
      await save({ ...settings, gameRoot: picked.root, gameFolder: picked.name });
      onDone();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="overlay">
      <div className="dialog wide" role="dialog" aria-modal="true">
        <div className="dialog-body">
          <h2>{t("gamefolder.title")}</h2>
          <p>{t("gamefolder.body")}</p>
          <GameFolderPicker label={t("settings.gameFolder")} onValid={setPicked} />
          <div className="actions">
            <button type="button" className="play" disabled={!picked || busy} onClick={apply}>{t("gamefolder.continue")}</button>
          </div>
        </div>
      </div>
    </div>
  );
}
