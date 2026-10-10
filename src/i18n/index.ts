import es from "./es";
import en from "./en";

export type Key = keyof typeof es;
export const LANGUAGES: { code: string; name: string }[] = [
  { code: "es", name: es["lang.name"] },
  { code: "en", name: en["lang.name"] },
];

const dictionaries: Record<string, Record<Key, string>> = { es, en };

export const translate = (lang: string, key: Key, vars?: Record<string, string | number>) => {
  const dict = dictionaries[lang] ?? es;
  const raw = dict[key] ?? es[key] ?? key;
  return vars ? raw.replace(/\{(\w+)\}/g, (_, k) => String(vars[k] ?? "")) : raw;
};

export const translateMessage = (lang: string, msg: string) => {
  if (msg.startsWith("snapshot.")) {
    const [key, arg] = msg.split(":");
    return key in es ? translate(lang, key as Key, { id: arg ?? "" }) : msg;
  }
  if (msg.startsWith("err.")) {
    const [key, ...rest] = msg.split(":");
    return key in es ? translate(lang, key as Key, { p: rest.join(":") }) : msg;
  }
  return msg;
};
