import { useEffect, useState } from "react";
import { backend } from "../lib/api";
import { useSettings } from "../lib/settings";
import { pickFolder } from "../lib/native";
import type { Diagnostics, GameOptions, ImportMode, Instance, InstanceKind, Role } from "../lib/types";
import { DiagWarnings } from "./DiagPanel";
import InstanceForm from "./InstanceForm";
import GameOptionsFields, { RECOMMENDED } from "./GameOptionsFields";
import DeriveWizard from "./DeriveWizard";

function Modal({ children, onClose }: { children: React.ReactNode; onClose?: () => void }) {
  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose?.()}>
      <div className="dialog" role="dialog" aria-modal="true">{children}</div>
    </div>
  );
}

export function SetupDialog({ onDone }: { onDone: () => void }) {
  const { t, settings } = useSettings();
  const [diag, setDiag] = useState<Diagnostics | null>(null);
  useEffect(() => {
    backend.diagnose().then(setDiag).catch(() => undefined);
  }, []);
  const register = async (name: string, description: string, code: string, kind?: InstanceKind, role?: Role | null) => {
    await backend.registerMain({ name, description, code, kind, role });
    onDone();
  };
  return (
    <Modal>
      <InstanceForm
        title={t("setup.title")}
        submitLabel={t("setup.save")}
        withKind
        extra={
          <>
            <DiagWarnings diag={diag} />
            <p>{t("setup.explain")}</p>
            {settings.multiplayer.enabled && <p>{t("setup.kindAsk")}</p>}
          </>
        }
        onSubmit={(v) => register(v.name, v.description, v.code, v.kind, v.role)}
        onSecondary={{ label: t("setup.skip"), run: () => register(t("setup.mainName"), "", t("setup.mainCode")) }}
      />
    </Modal>
  );
}

type Step = "choose" | "create" | "pick" | "adopt" | "import" | "derive";

const suggestCode = (path: string) =>
  (path.split(/[\\/]/).filter(Boolean).pop() ?? "").replace(/[^A-Za-z0-9]/g, "").slice(0, 12) || "Sims";

export function AddDialog({ instances, preset, onClose, onDone }: { instances: Instance[]; preset?: Instance; onClose: () => void; onDone: () => void }) {
  const { t, size } = useSettings();
  const [step, setStep] = useState<Step>(preset ? "adopt" : "choose");
  const [picked, setPicked] = useState<Instance | null>(preset ?? null);
  const [folder, setFolder] = useState("");
  const [mode, setMode] = useState<ImportMode>("copy");
  const [gameOptions, setGameOptions] = useState<GameOptions>(RECOMMENDED);
  const unregistered = instances.filter((i) => i.legacy);

  const chooseOther = async () => {
    const path = await pickFolder();
    if (path) {
      setFolder(path);
      setStep("import");
    }
  };

  if (step === "derive") {
    return <DeriveWizard instances={instances} onClose={onClose} onBack={() => setStep("choose")} onDone={onDone} />;
  }

  return (
    <Modal onClose={onClose}>
      {step === "choose" && (
        <div className="dialog-body">
          <h2>{t("add.title")}</h2>
          <button type="button" className="choice" onClick={() => setStep("create")}>
            <b>{t("add.create")}</b>
            <small>{t("add.createHint")}</small>
          </button>
          <button type="button" className="choice" onClick={() => setStep("pick")}>
            <b>{t("add.existing")}</b>
            <small>{t("add.existingHint")}</small>
          </button>
          <button type="button" className="choice" onClick={() => setStep("derive")}>
            <b>{t("derive.button")}</b>
            <small>{t("derive.buttonHint")}</small>
          </button>
          <div className="actions"><button type="button" className="ghost" onClick={onClose}>{t("common.cancel")}</button></div>
        </div>
      )}
      {step === "create" && (
        <InstanceForm
          title={t("add.createTitle")}
          submitLabel={t("add.createBtn")}
          withKind
          extra={<GameOptionsFields value={gameOptions} onChange={setGameOptions} />}
          onSubmit={async (v) => { await backend.createInstance({ ...v, options: gameOptions }); onDone(); }}
          onSecondary={{ label: t("common.back"), run: () => setStep("choose") }}
        />
      )}
      {step === "pick" && (
        <div className="dialog-body">
          <h2>{t("add.pickTitle")}</h2>
          {unregistered.map((i) => (
            <button key={i.id} type="button" className="choice" onClick={() => { setPicked(i); setStep("adopt"); }}>
              <b>Los Sims 4_{i.id}</b>
              <small>{t("add.folderStats", { mods: i.modCount, saves: i.saveCount, size: size(i.sizeBytes) })}</small>
            </button>
          ))}
          {unregistered.length === 0 && <small className="status">{t("add.none")}</small>}
          <button type="button" className="choice" onClick={chooseOther}>
            <b>{t("add.otherFolder")}</b>
            <small>{t("add.otherFolderHint")}</small>
          </button>
          <div className="actions"><button type="button" className="ghost" onClick={() => setStep("choose")}>{t("common.back")}</button></div>
        </div>
      )}
      {step === "import" && (
        <InstanceForm
          title={t("add.importTitle")}
          submitLabel={t("add.adopt")}
          withKind
          initial={{ code: suggestCode(folder) }}
          extra={
            <>
              <small className="status">{t("add.importFolder", { path: folder })}</small>
              <div className="choices">
                <button type="button" className="choice" aria-pressed={mode === "copy"} onClick={() => setMode("copy")}>
                  <b>{t("add.copy")}</b>
                  <small>{t("add.copyHint")}</small>
                </button>
                <button type="button" className="choice" aria-pressed={mode === "move"} onClick={() => setMode("move")}>
                  <b>{t("add.move")}</b>
                  <small>{t("add.moveHint")}</small>
                </button>
              </div>
            </>
          }
          onSubmit={async (v) => { await backend.importInstance({ ...v, path: folder, mode }); onDone(); }}
          onSecondary={{ label: t("common.back"), run: () => setStep("pick") }}
        />
      )}
      {step === "adopt" && picked && (
        <InstanceForm
          title={t("add.title")}
          submitLabel={t("add.adopt")}
          withKind
          lockCode
          initial={{ code: picked.id, kind: picked.kind }}
          onSubmit={async (v) => { await backend.adoptInstance(v); onDone(); }}
          onSecondary={preset ? { label: t("common.cancel"), run: onClose } : { label: t("common.back"), run: () => setStep("pick") }}
        />
      )}
    </Modal>
  );
}
