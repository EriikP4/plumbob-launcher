import { useSettings } from "../lib/settings";
import { assetUrl } from "../lib/assets";
import type { Instance } from "../lib/types";

export default function InstanceThumb({ instance }: { instance: Instance }) {
  const { settings } = useSettings();
  const icon = settings.instanceIcons[instance.id];
  return (
    <span className="thumb" style={{ background: `linear-gradient(135deg,${instance.colors[0]},${instance.colors[1]})` }}>
      {icon ? <img className="thumb-img" src={assetUrl(icon)} alt="" decoding="async" /> : instance.name[0]}
    </span>
  );
}
