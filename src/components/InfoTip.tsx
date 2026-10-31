export default function InfoTip({ text, side = "left" }: { text: string; side?: "left" | "right" }) {
  return (
    <span className={`tip ${side} info-dot info-tip`} tabIndex={0} data-tip={text} aria-label={text} onClick={(e) => e.preventDefault()}>i</span>
  );
}
