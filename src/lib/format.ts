export const parseVersion = (v: string) => v.split(".").map((n) => Number.parseInt(n, 10) || 0);

export const compareVersions = (a: string, b: string) => {
  const x = parseVersion(a);
  const y = parseVersion(b);
  for (let i = 0; i < Math.max(x.length, y.length); i += 1) {
    const d = (x[i] ?? 0) - (y[i] ?? 0);
    if (d !== 0) return d;
  }
  return 0;
};

export const newestVersion = (versions: (string | null)[]) =>
  versions.reduce<string | null>((best, v) => (v && (!best || compareVersions(v, best) > 0) ? v : best), null);
