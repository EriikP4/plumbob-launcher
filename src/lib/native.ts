import { open, save } from "@tauri-apps/plugin-dialog";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";

const desktop = "__TAURI_INTERNALS__" in window;

export async function pickFile(extensions: string[] = ["exe"]): Promise<string | null> {
  if (!desktop) return null;
  const r = await open({ multiple: false, directory: false, filters: [{ name: extensions.join(", "), extensions }] });
  return typeof r === "string" ? r : null;
}

export async function pickImages(): Promise<string[]> {
  if (!desktop) return [];
  const r = await open({ multiple: true, directory: false, filters: [{ name: "png, jpg, jpeg", extensions: ["png", "jpg", "jpeg"] }] });
  return Array.isArray(r) ? r : typeof r === "string" ? [r] : [];
}

export async function pickFolder(): Promise<string | null> {
  if (!desktop) return null;
  const r = await open({ multiple: false, directory: true });
  return typeof r === "string" ? r : null;
}

export async function openLink(url: string): Promise<void> {
  if (desktop) await openUrl(url);
  else window.open(url, "_blank", "noopener");
}

export async function pickTextFiles(): Promise<string[]> {
  if (!desktop) return [];
  const r = await open({ multiple: true, directory: false, filters: [{ name: "md, txt, json", extensions: ["md", "txt", "json"] }] });
  return Array.isArray(r) ? r : typeof r === "string" ? [r] : [];
}

export async function pickSavePath(defaultPath: string, extension: string): Promise<string | null> {
  if (!desktop) return null;
  return save({ defaultPath, filters: [{ name: extension, extensions: [extension] }] });
}

export async function appVersion(): Promise<string> {
  if (!desktop) return "dev";
  try {
    return await getVersion();
  } catch {
    return "";
  }
}

export async function pickAudioFiles(): Promise<string[]> {
  if (!desktop) return [];
  const r = await open({ multiple: true, directory: false, filters: [{ name: "mp3, wav, flac, ogg, m4a", extensions: ["mp3", "wav", "flac", "ogg", "m4a"] }] });
  return Array.isArray(r) ? r : typeof r === "string" ? [r] : [];
}
