export type Tier = "LQ" | "HD" | "FHD" | "2K" | "4K" | "8K";

export interface Quality {
  tier: Tier;
  label: string;
  color: string;
  glow: boolean;
}

const TIERS: Record<Tier, Omit<Quality, "tier">> = {
  LQ: { label: "480p LQ", color: "#ffa21f", glow: false },
  HD: { label: "720p HD", color: "#3ddc84", glow: false },
  FHD: { label: "1080p FHD", color: "#4da3ff", glow: true },
  "2K": { label: "2K", color: "#b57cff", glow: true },
  "4K": { label: "4K", color: "#ff6fb5", glow: true },
  "8K": { label: "8K", color: "#ff4d4d", glow: true },
};

export const qualityOf = (width: number, height: number): Quality | null => {
  const p = Math.min(width, height);
  if (!p) return null;
  const tier: Tier = p < 600 ? "LQ" : p < 900 ? "HD" : p < 1260 ? "FHD" : p < 1800 ? "2K" : p < 3500 ? "4K" : "8K";
  return { tier, ...TIERS[tier] };
};
