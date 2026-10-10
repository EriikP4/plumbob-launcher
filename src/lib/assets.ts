import { convertFileSrc } from "@tauri-apps/api/core";

export const assetUrl = (path: string) => (path.startsWith("data:") ? path : convertFileSrc(path));
