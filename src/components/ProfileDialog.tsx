import { createPortal } from "react-dom";
import { useSettings } from "../lib/settings";
import ProfileFields from "./ProfileFields";

export default function ProfileDialog({ onClose }: { onClose: () => void }) {
  const { t } = useSettings();

  return createPortal(
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="dialog wide" role="dialog" aria-modal="true">
        <div className="dialog-body">
          <h2>{t("profile.edit")}</h2>
          <ProfileFields />
          <div className="actions">
            <button type="button" className="play small-play" onClick={onClose}>{t("profile.done")}</button>
          </div>
        </div>
      </div>
    </div>,
    document.body,
  );
}
