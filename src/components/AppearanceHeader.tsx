import { useSettings } from "../lib/settings";
import { assetUrl } from "../lib/assets";
import { gradientOf, isHex, pickBannerChoice, pickIconChoice, type Appearance } from "../lib/appearance";
import Icon from "./Icon";

interface Props {
  value: Appearance;
  onChange: (next: Appearance) => void;
  fallback: [string, string];
  letter: string;
  immediate: string;
  editable: boolean;
  existingIcon?: string;
  existingBanner?: string;
  onError: (message: string) => void;
  children: React.ReactNode;
}

export default function AppearanceHeader({ value, onChange, fallback, letter, immediate, editable, existingIcon, existingBanner, onError, children }: Props) {
  const { t, msg } = useSettings();

  const colors = isHex(value.color) ? gradientOf(value.color) : fallback;
  const icon = value.icon.mode === "keep" ? existingIcon : value.icon.mode === "stored" ? value.icon.path : undefined;
  const customBanner = value.banner.mode === "keep" ? existingBanner : value.banner.mode === "stored" ? value.banner.path : undefined;

  const run = async (job: () => Promise<void>) => {
    onError("");
    try {
      await job();
    } catch (err) {
      onError(msg(err instanceof Error ? err.message : String(err)));
    }
  };

  const changeIcon = () =>
    run(async () => {
      const choice = await pickIconChoice(immediate);
      if (choice) onChange({ ...value, icon: choice });
    });

  const changeBanner = () =>
    run(async () => {
      const choice = await pickBannerChoice(immediate);
      if (choice) onChange({ ...value, banner: choice });
    });

  const thumb = (
    <span className="thumb" style={{ background: `linear-gradient(135deg,${colors[0]},${colors[1]})` }}>
      {icon ? <img className="thumb-img" src={assetUrl(icon)} alt="" decoding="async" /> : letter}
    </span>
  );

  return (
    <header className="edit-head">
      <div className="edit-head-bg" aria-hidden="true" style={{ background: `linear-gradient(135deg,${colors[0]},${colors[1]})` }}>
        {customBanner && <img src={assetUrl(customBanner)} alt="" decoding="async" />}
      </div>
      {editable ? (
        <div className="edit-thumb">
          <button type="button" className="edit-thumb-btn tip below left" data-tip={t("appearance.editIcon")} aria-label={t("appearance.editIcon")} onClick={() => void changeIcon()}>
            {thumb}
            <span className="thumb-edit"><Icon name="edit" size={18} /></span>
          </button>
          {icon && (
            <button type="button" className="thumb-clear tip below left" data-tip={t("appearance.removeIcon")} aria-label={t("appearance.removeIcon")} onClick={() => onChange({ ...value, icon: { mode: "none" } })}><Icon name="close" size={10} /></button>
          )}
        </div>
      ) : (
        <div className="edit-thumb">{thumb}</div>
      )}
      {children}
      {editable && (
        <div className="edit-banner-actions">
          {customBanner && (
            <button type="button" className="edit-banner-btn square tip below right" data-tip={t("appearance.removeBanner")} aria-label={t("appearance.removeBanner")} onClick={() => onChange({ ...value, banner: { mode: "none" } })}><Icon name="trash" size={14} /></button>
          )}
          <button type="button" className="edit-banner-btn" aria-label={t("appearance.editBanner")} onClick={() => void changeBanner()}>
            <Icon name="image" size={14} /> {t("appearance.bannerBtn")}
          </button>
        </div>
      )}
    </header>
  );
}
