import { useEffect, useState } from "react";
import EditInstanceForm from "./EditInstanceForm";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import type { Instance, Snapshot } from "../lib/types";
import InstanceForm from "./InstanceForm";
import DeleteFlow from "./DeleteFlow";

function Modal({ children, onClose, wide }: { children: React.ReactNode; onClose: () => void; wide?: boolean }) {
  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className={"dialog" + (wide ? " edit-dialog" : "")} role="dialog" aria-modal="true">{children}</div>
    </div>
  );
}

type Action = () => Promise<void>;

function Confirm({ title, children, actions, onClose }: {
  title: string;
  children: React.ReactNode;
  actions: { label: string; run: Action; kind?: "danger" | "primary" | "ghost" }[];
  onClose: () => void;
}) {
  const { msg, t } = useSettings();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const run = async (action: Action) => {
    setBusy(true);
    setError("");
    try {
      await action();
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
      setBusy(false);
    }
  };

  return (
    <Modal onClose={onClose}>
      <div className="dialog-body">
        <h2>{title}</h2>
        {children}
        {error && <p className="error" role="alert">{error}</p>}
        <div className="actions wrap">
          <button type="button" className="ghost" disabled={busy} onClick={onClose}>{t("common.cancel")}</button>
          {actions.map((a) => (
            <button key={a.label} type="button" className={a.kind === "danger" ? "danger" : a.kind === "ghost" ? "ghost" : "play"} disabled={busy} onClick={() => run(a.run)}>
              {busy ? t("common.working") : a.label}
            </button>
          ))}
        </div>
      </div>
    </Modal>
  );
}

export function ConvertDialog({ instance, onClose, onDone }: { instance: Instance; onClose: () => void; onDone: () => void }) {
  const { t } = useSettings();
  const toMp = instance.kind === "solo";
  const [files, setFiles] = useState<string[]>([]);

  useEffect(() => {
    if (!toMp) backend.s4mpLeftovers(instance.id).then(setFiles).catch(() => setFiles([]));
  }, [instance.id, toMp]);

  const convert = (remove: boolean) => async () => {
    await backend.convertInstance(instance.id, toMp ? "mp" : "solo", remove);
    onDone();
  };

  if (toMp) {
    return (
      <Confirm title={t("convert.toMpTitle", { name: instance.name })} onClose={onClose} actions={[{ label: t("convert.move"), run: convert(false) }]}>
        <p>{t("convert.toMpBody")}</p>
      </Confirm>
    );
  }

  return (
    <Confirm
      title={t("convert.toSoloTitle", { name: instance.name })}
      onClose={onClose}
      actions={
        files.length > 0
          ? [
              { label: t("convert.moveKeep"), run: convert(false), kind: "ghost" },
              { label: t("convert.moveDelete"), run: convert(true), kind: "danger" },
            ]
          : [{ label: t("convert.move"), run: convert(false) }]
      }
    >
      <p>{t("convert.toSoloBody")}</p>
      {files.length > 0 && (
        <>
          <p>{t("convert.willDelete")}</p>
          <ul className="file-box">{files.map((f) => <li key={f}>{f}</li>)}</ul>
          <small className="status">{t("convert.safe")}</small>
        </>
      )}
    </Confirm>
  );
}

export function RestoreDialog({ instance, snapshot, onClose, onDone }: { instance: Instance; snapshot: Snapshot; onClose: () => void; onDone: () => void }) {
  const { t, when } = useSettings();
  return (
    <Confirm
      title={t("restore.title")}
      onClose={onClose}
      actions={[{ label: t("restore.confirm"), run: async () => { await backend.restoreSnapshot(instance.id, snapshot.id); onDone(); } }]}
    >
      <p>{t("restore.body", { name: instance.name, when: when(snapshot.created) })}</p>
    </Confirm>
  );
}

function TrayEnableDialog({ instance, onClose, onDone }: { instance: Instance; onClose: () => void; onDone: () => void }) {
  const { t } = useSettings();
  return (
    <Confirm
      title={t("tray.enableTitle")}
      onClose={onClose}
      actions={[{ label: t("tray.enableConfirm"), run: async () => { await backend.enableTraySync(instance.id); onDone(); } }]}
    >
      <p>{t("tray.enableBody")}</p>
      <p><small className="status">{t("tray.note")}</small></p>
      <p><small className="status">{t("tray.enableInfo")}</small></p>
    </Confirm>
  );
}

function TrayDisableDialog({ instance, onClose, onDone }: { instance: Instance; onClose: () => void; onDone: () => void }) {
  const { t, msg } = useSettings();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const hasOld = instance.trayOld;

  const run = async (mode: "restore" | "merge" | "empty") => {
    setBusy(true);
    setError("");
    try {
      await backend.disableTraySync(instance.id, mode);
      onDone();
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
      setBusy(false);
    }
  };

  return (
    <Modal onClose={busy ? () => undefined : onClose}>
      <div className="dialog-body">
        <h2>{t("tray.disableTitle")}</h2>
        <p>{hasOld ? t("tray.disableBodyOld") : t("tray.disableBodyNoOld")}</p>
        <div className="tray-actions">
          <button type="button" className="ghost tip left" disabled={busy || !hasOld} data-tip={hasOld ? t("tray.restoreTip") : t("tray.restoreNone")} onClick={() => run("restore")}>{t("tray.restore")}</button>
          <button type="button" className="ghost tip right" disabled={busy} data-tip={t("tray.mergeTip")} onClick={() => run("merge")}>{t("tray.merge")}</button>
          {hasOld ? (
            <button type="button" className="ghost tip left" disabled={busy} data-tip={t("tray.emptyTip")} onClick={() => run("empty")}>{t("tray.empty")}</button>
          ) : (
            <button type="button" className="ghost tip left" disabled={busy} data-tip={t("tray.unlinkTip")} onClick={() => run("empty")}>{t("tray.unlink")}</button>
          )}
        </div>
        {error && <p className="error" role="alert">{error}</p>}
        <div className="actions">
          <button type="button" className="ghost" disabled={busy} onClick={onClose}>{t("common.cancel")}</button>
        </div>
      </div>
    </Modal>
  );
}

function ArchiveDialog({ instance, active, onClose, onDone }: { instance: Instance; active: boolean; onClose: () => void; onDone: () => void }) {
  const { t, msg } = useSettings();
  const [clean, setClean] = useState(false);
  const [compress, setCompress] = useState(false);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState({ done: 0, total: 1 });
  const [error, setError] = useState("");

  useEffect(() => {
    const off = backend.onArchiveProgress((p) => setProgress({ done: p.done, total: Math.max(1, p.total) }));
    return () => void off.then((f) => f());
  }, []);

  const run = async () => {
    setBusy(true);
    setError("");
    setProgress({ done: 0, total: 1 });
    try {
      await backend.archiveInstance(instance.id, clean, compress);
      onDone();
    } catch (e) {
      setError(msg(e instanceof Error ? e.message : String(e)));
      setBusy(false);
    }
  };

  return (
    <Modal onClose={busy ? () => undefined : onClose}>
      <div className="dialog-body">
        <h2>{t("archive.title", { name: instance.name })}</h2>
        <p>{t("archive.body")}</p>
        <div className="archive-opts">
          <label className="check">
            <input type="checkbox" checked={clean} disabled={busy} onChange={(e) => setClean(e.target.checked)} />
            <span>{t("archive.clean")}</span>
          </label>
          {clean && <p className="warn-text">{t("archive.cleanWarn")}</p>}
          <label className="check">
            <input type="checkbox" checked={compress} disabled={busy} onChange={(e) => setCompress(e.target.checked)} />
            <span>{t("archive.compress")}</span>
          </label>
          <small className="status hint">{t("archive.compressHint")}</small>
        </div>
        {active && <p className="error" role="alert">{t("archive.active")}</p>}
        {busy && <div className="progress"><i style={{ width: `${Math.round((progress.done / progress.total) * 100)}%` }} /></div>}
        {error && <p className="error" role="alert">{error}</p>}
        <div className="actions">
          <button type="button" className="ghost" disabled={busy} onClick={onClose}>{t("common.cancel")}</button>
          <button type="button" className="play" disabled={busy || active} onClick={run}>{busy ? t("archive.working") : t("archive.confirm")}</button>
        </div>
      </div>
    </Modal>
  );
}

type EditStep = "edit" | "duplicate" | "convert" | "delete" | "archive" | "trayOn" | "trayOff";

export function EditDialog({ instance, active, onClose, onDone }: { instance: Instance; active: boolean; onClose: () => void; onDone: () => void }) {
  const { t, settings } = useSettings();
  const [step, setStep] = useState<EditStep>("edit");

  if (step === "delete") {
    return <DeleteFlow instance={instance} onCancel={() => setStep("edit")} onDone={onDone} />;
  }

  if (step === "archive") {
    return <ArchiveDialog instance={instance} active={active} onClose={() => setStep("edit")} onDone={onDone} />;
  }

  if (step === "trayOn") {
    return <TrayEnableDialog instance={instance} onClose={() => setStep("edit")} onDone={onDone} />;
  }

  if (step === "trayOff") {
    return <TrayDisableDialog instance={instance} onClose={() => setStep("edit")} onDone={onDone} />;
  }

  if (step === "convert") {
    return <ConvertDialog instance={instance} onClose={() => setStep("edit")} onDone={onDone} />;
  }

  return (
    <Modal onClose={onClose} wide={step === "edit"}>
      {step === "edit" && <EditInstanceForm instance={instance} onClose={onClose} onDone={onDone} onStep={setStep} />}
      {step === "duplicate" && (
        <InstanceForm
          title={t("edit.dupTitle")}
          submitLabel={t("edit.dupBtn")}
          initial={{ name: t("edit.dupName", { name: instance.name }), description: instance.description }}
          extra={<small className="status">{t("edit.dupNote")}</small>}
          onSubmit={async (v) => { await backend.duplicateInstance(instance.id, v.name, v.description, v.code); onDone(); }}
          onSecondary={{ label: t("common.back"), run: () => setStep("edit") }}
        />
      )}
    </Modal>
  );
}
