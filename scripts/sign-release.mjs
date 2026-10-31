import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const version = process.argv[2] ?? pkg.version;
const key = process.env.PLUMBOB_UPDATER_KEY ?? join(homedir(), ".plumbob-launcher", "updater.key");
const password = process.env.PLUMBOB_UPDATER_PASSWORD ?? "";
const installer = join(root, "release", `Plumbob-Launcher-${version}-windows-x64-setup.exe`);

const fail = (message) => {
  console.error(message);
  process.exit(1);
};

if (!existsSync(key)) fail(`No se encuentra la clave privada: ${key}`);
if (!existsSync(installer)) fail(`No se encuentra el instalador: ${installer}`);

const result = spawnSync(`npx tauri signer sign -f "${key}" -p "${password}" "${installer}"`, { cwd: root, shell: true, stdio: "inherit" });
if (result.status !== 0) fail("No se pudo firmar el instalador.");

const signature = `${installer}.sig`;
if (!existsSync(signature)) fail("No se generó el archivo de firma.");
console.log(`Firma lista: ${signature}`);
console.log("Sube a la release de GitHub el instalador y este archivo .sig.");
