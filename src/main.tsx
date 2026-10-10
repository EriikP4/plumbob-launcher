import { createRoot } from "react-dom/client";
import App from "./App";
import Boot from "./components/Boot";
import { SettingsProvider } from "./lib/settings";
import "./styles.css";
import { installCheckEffects } from "./lib/checkfx";
import { installModalEffects } from "./lib/modalfx";

installCheckEffects();
installModalEffects();

document.addEventListener("contextmenu", (e) => {
  const el = e.target;
  if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) return;
  e.preventDefault();
});

createRoot(document.getElementById("root")!).render(
  <SettingsProvider>
    <App />
    <Boot />
  </SettingsProvider>,
);
