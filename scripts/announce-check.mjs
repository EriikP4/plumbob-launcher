import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const file = join(root, "announcements", "announcements.json");
const errors = [];
const bad = (message) => errors.push(message);

let text = "";
try {
  text = readFileSync(file, "utf8").replace(/^\uFEFF/, "");
} catch {
  console.error(`No se puede leer ${file}`);
  process.exit(1);
}
if (Buffer.byteLength(text) > 64 * 1024) bad("El archivo supera 64 KB");

let data;
try {
  data = JSON.parse(text);
} catch (e) {
  console.error(`JSON inválido: ${e.message}`);
  process.exit(1);
}

const KINDS = ["info", "update", "event", "warning"];
const isDate = (v) => typeof v === "string" && /^\d{4}-\d{2}-\d{2}$/.test(v) && !Number.isNaN(Date.parse(v)) && new Date(v).toISOString().startsWith(v);
const isVersion = (v) => typeof v === "string" && /^v?\d+(\.\d+){0,2}(-[\w.]+)?$/.test(v);
const isHttps = (v) => {
  try {
    const u = new URL(v);
    return u.protocol === "https:" && !u.username && !u.password && v.length <= 300 && !/\s/.test(v);
  } catch {
    return false;
  }
};
const text2 = (obj, name, max, required) => {
  if (typeof obj !== "object" || obj === null || Array.isArray(obj)) return bad(`${name}: debe ser un objeto con es y en`);
  for (const lang of ["es", "en"]) {
    const v = obj[lang];
    if (v === undefined || v === "") {
      if (required) bad(`${name}.${lang}: falta`);
      continue;
    }
    if (typeof v !== "string") bad(`${name}.${lang}: debe ser texto`);
    else if (v.length > max) bad(`${name}.${lang}: ${v.length} caracteres, máximo ${max}`);
    else if (/[\u0000-\u0009\u000b-\u001f\u007f]/.test(v)) bad(`${name}.${lang}: contiene caracteres de control`);
  }
};

if (data?.version !== 1) bad("version debe ser 1");
if (!Array.isArray(data?.items)) {
  bad("items debe ser una lista");
} else {
  if (data.items.length > 30) bad(`Hay ${data.items.length} anuncios, máximo 30`);
  const ids = new Set();
  data.items.forEach((item, i) => {
    const at = `items[${i}]${item?.id ? ` (${item.id})` : ""}`;
    if (typeof item !== "object" || item === null) return bad(`${at}: debe ser un objeto`);
    if (typeof item.id !== "string" || !item.id.trim() || item.id.length > 64) bad(`${at}: id vacío o de más de 64 caracteres`);
    else if (ids.has(item.id)) bad(`${at}: id repetido`);
    else ids.add(item.id);
    if (!isDate(item.date)) bad(`${at}: date debe ser AAAA-MM-DD válida`);
    if (item.type !== undefined && !KINDS.includes(item.type)) bad(`${at}: type debe ser ${KINDS.join(", ")}`);
    if (item.pinned !== undefined && typeof item.pinned !== "boolean") bad(`${at}: pinned debe ser true o false`);
    text2(item.title, `${at}.title`, 120, true);
    text2(item.body, `${at}.body`, 2000, true);
    if (item.expires !== undefined && !isDate(item.expires)) bad(`${at}: expires debe ser AAAA-MM-DD válida`);
    for (const k of ["minVersion", "maxVersion"]) if (item[k] !== undefined && !isVersion(item[k])) bad(`${at}: ${k} no es una versión válida`);
    if (item.link !== undefined) {
      if (!isHttps(item.link?.url)) bad(`${at}.link.url: debe ser https y válida`);
      text2(item.link?.label, `${at}.link.label`, 60, true);
    }
  });
}

if (errors.length) {
  console.error(`announcements.json tiene ${errors.length} problema(s):`);
  for (const e of errors) console.error(`- ${e}`);
  process.exit(1);
}
console.log(`announcements.json correcto (${data.items.length} anuncio(s))`);
