import { useState } from "react";
import { createPortal } from "react-dom";
import { useSettings } from "../lib/settings";

export default function InfoButton({ title, text, label }: { title: string; text: string; label?: string }) {
  const { t } = useSettings();
  const [open, setOpen] = useState(false);
  return (
    <>
      <button type="button" className="info-dot info-btn" aria-label={label ?? title} onClick={(e) => { e.preventDefault(); e.stopPropagation(); setOpen(true); }}>i</button>
      {open && createPortal(
        <div className="overlay stacked" onMouseDown={(e) => e.target === e.currentTarget && setOpen(false)}>
          <div className="dialog" role="dialog" aria-modal="true">
            <div className="dialog-body">
              <h2>{title}</h2>
              <p>{text}</p>
              <div className="actions"><button type="button" className="play" onClick={() => setOpen(false)}>{t("common.close")}</button></div>
            </div>
          </div>
        </div>,
        document.body,
      )}
    </>
  );
}
