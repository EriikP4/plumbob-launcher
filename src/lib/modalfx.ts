const DURATION = 220;

function motionAllowed() {
  if (document.documentElement.classList.contains("no-anim")) return false;
  return !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function fadeOut(node: HTMLElement) {
  node.classList.add("closing");
  node.setAttribute("inert", "");
  node.style.pointerEvents = "none";
  document.body.appendChild(node);
  window.setTimeout(() => node.remove(), DURATION + 40);
}

export function installModalEffects() {
  new MutationObserver((records) => {
    if (!motionAllowed()) return;
    for (const record of records) {
      record.removedNodes.forEach((node) => {
        if (!(node instanceof HTMLElement) || node.classList.contains("closing")) return;
        if (node.classList.contains("overlay")) return fadeOut(node);
        [...node.querySelectorAll<HTMLElement>(".overlay")]
          .filter((o) => !o.parentElement?.closest(".overlay"))
          .forEach(fadeOut);
      });
    }
  }).observe(document.body, { childList: true, subtree: true });
}
