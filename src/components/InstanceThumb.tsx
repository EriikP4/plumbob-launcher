import { useSettings } from "../lib/settings";
import { assetUrl } from "../lib/assets";
import type { Instance } from "../lib/types";
import { colorsFor } from "../lib/appearance";

export default function InstanceThumb({ instance }: { instance: Instance }) {
  const { settings } = useSettings();
  const icon = settings.instanceIcons[instance.id];
  const colors = colorsFor(instance, settings);
  return (
    <span className="thumb" style={{ background: `linear-gradient(135deg,${colors[0]},${colors[1]})` }}>
      {icon ? <img className="thumb-img" src={assetUrl(icon)} alt="" decoding="async" /> : instance.name[0]}
    </span>
  );
}
