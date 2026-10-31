export const parseVersion = (tag: string): [number, number, number] | null => {
  const core = tag.trim().replace(/^[vV]/, "").split(/[-+]/)[0];
  const parts = core.split(".");
  if (parts.length > 3 || parts.some((p) => !/^\d+$/.test(p))) return null;
  const [a, b = "0", c = "0"] = parts;
  return [Number(a), Number(b), Number(c)];
};

export const isNewer = (candidate: string, than: string) => {
  const x = parseVersion(candidate);
  const y = parseVersion(than);
  if (!x || !y) return false;
  for (let i = 0; i < 3; i += 1) if (x[i] !== y[i]) return x[i] > y[i];
  return false;
};

export const shouldNotify = (version: string, ignored: string | null | undefined) => (ignored ? isNewer(version, ignored) : true);
