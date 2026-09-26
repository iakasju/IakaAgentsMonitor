// Point d'entree de la fenetre. La webview est un pur consommateur :
//  - instantane initial via la commande `get_running_agents`,
//  - mises a jour via l'evenement `tray://agents`.
// Aucun calcul metier ici (D9) : voir render.ts.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { renderAgents } from "./render";
import type { AgentsSnapshot } from "./types";

async function refreshAgents(): Promise<void> {
  try {
    renderAgents(await invoke<AgentsSnapshot>("get_running_agents"));
  } catch (e) {
    console.error("get_running_agents a echoue", e);
  }
}

listen<AgentsSnapshot>("tray://agents", (event) => {
  renderAgents(event.payload);
}).catch((e) => console.error("listen tray://agents a echoue", e));

void refreshAgents();

// ---- Theme clair/sombre (amendement A3, point 1) ----
// Par defaut, suit macOS (media query CSS, aucun JS requis). Bascule manuelle memorisee dans
// localStorage : le bouton alterne clair <-> sombre <-> (retour au suivi automatique de macOS).
const THEME_KEY = "iakaam-theme";

function currentTheme(): "light" | "dark" | null {
  const attr = document.documentElement.getAttribute("data-theme");
  return attr === "light" || attr === "dark" ? attr : null;
}

function systemPrefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

function applyTheme(theme: "light" | "dark" | null): void {
  if (theme === null) {
    document.documentElement.removeAttribute("data-theme");
    try {
      localStorage.removeItem(THEME_KEY);
    } catch {
      /* localStorage indisponible : rien a memoriser. */
    }
    return;
  }
  document.documentElement.setAttribute("data-theme", theme);
  try {
    localStorage.setItem(THEME_KEY, theme);
  } catch {
    /* localStorage indisponible : la bascule reste valable pour cette session seule. */
  }
}

document.getElementById("themeBtn")?.addEventListener("click", () => {
  const current = currentTheme();
  if (current === null) {
    // Pas de preference memorisee -> bascule vers l'oppose de ce que montre macOS actuellement.
    applyTheme(systemPrefersDark() ? "light" : "dark");
  } else if (current === "light") {
    applyTheme("dark");
  } else {
    // Deja force en sombre : revient au suivi automatique de macOS plutot que de boucler.
    applyTheme(systemPrefersDark() ? null : "light");
  }
});
