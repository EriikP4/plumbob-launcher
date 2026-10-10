const COLORS = ["#4aa8ff", "#ffd84a", "#b07cff"];
const STAR = "M12 2l2.6 7.4L22 12l-7.4 2.6L12 22l-2.6-7.4L2 12l7.4-2.6z";

function motionAllowed() {
  if (document.documentElement.classList.contains("no-anim")) return false;
  return !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function burst(box: HTMLElement) {
  const rect = box.getBoundingClientRect();
  if (!rect.width) return;
  const layer = document.createElement("div");
  layer.className = "check-burst";
  layer.style.left = `${rect.left + rect.width / 2}px`;
  layer.style.top = `${rect.top + rect.height / 2}px`;
  const count = 7;
  for (let i = 0; i < count; i += 1) {
    const angle = (Math.PI * 2 * i) / count + Math.random() * 0.6 - 0.3;
    const dist = 20 + Math.random() * 14;
    const size = 8 + Math.random() * 5;
    const star = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    star.setAttribute("viewBox", "0 0 24 24");
    star.setAttribute("width", String(size));
    star.setAttribute("height", String(size));
    star.classList.add("check-star");
    star.style.setProperty("--dx", `${Math.cos(angle) * dist}px`);
    star.style.setProperty("--dy", `${Math.sin(angle) * dist}px`);
    star.style.setProperty("--rot", `${Math.round(Math.random() * 180 - 90)}deg`);
    star.style.color = COLORS[i % COLORS.length];
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    path.setAttribute("d", STAR);
    path.setAttribute("fill", "currentColor");
    star.appendChild(path);
    layer.appendChild(star);
  }
  document.body.appendChild(layer);
  window.setTimeout(() => layer.remove(), 800);
}

export function burstAt(el: HTMLElement) {
  if (motionAllowed()) burst(el);
}

export function installCheckEffects() {
  document.addEventListener("change", (e) => {
    const el = e.target;
    if (!(el instanceof HTMLInputElement) || el.type !== "checkbox" || !el.checked || !motionAllowed()) return;
    burst(el);
  });
}
