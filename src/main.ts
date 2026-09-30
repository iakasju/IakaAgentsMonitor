// Point d'entree de la fenetre. La webview est un pur consommateur :
//  - instantane initial via la commande `get_running_agents`,
//  - mises a jour via l'evenement `tray://agents`.
// Aucun calcul metier ici (D9) : voir render.ts.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  measureContentHeight,
  measureContentWidth,
  renderAgents,
  renderFleet,
  setCompactMode,
} from "./render";
import type { AgentsSnapshot, RemoteView } from "./types";

// ---- Largeur et hauteur de fenetre ajustees au contenu (amendements A6 point 1 et A7 point 1) --
// Mesurees ici (webview = seule couche qui connait le rendu reel des polices), appliquees cote
// Rust (`fit_window_width` / `fit_window_height`) qui bornent (largeur 480-1200px, hauteur
// 200px-80% de l'ecran) et appliquent l'hysteresis documentee dans tray.rs : ces fonctions se
// contentent d'envoyer la mesure a chaque peinture, jamais de decider seules.
function fitWindowToContent(): void {
  const width = measureContentWidth();
  if (width !== null) {
    invoke("fit_window_width", { naturalWidth: width }).catch((e) =>
      console.error("fit_window_width a echoue", e),
    );
  }
  const height = measureContentHeight();
  if (height !== null) {
    invoke("fit_window_height", { naturalHeight: height }).catch((e) =>
      console.error("fit_window_height a echoue", e),
    );
  }
}

async function refreshAgents(): Promise<void> {
  try {
    renderAgents(await invoke<AgentsSnapshot>("get_running_agents"));
    fitWindowToContent();
  } catch (e) {
    console.error("get_running_agents a echoue", e);
  }
}

listen<AgentsSnapshot>("tray://agents", (event) => {
  renderAgents(event.payload);
  fitWindowToContent();
}).catch((e) => console.error("listen tray://agents a echoue", e));

// « Autres machines » (central) : même mécanique — instantané initial + événement. Sans central
// configuré, le backend renvoie `not_configured` et la section reste cachée : rien ne change.
async function refreshFleet(): Promise<void> {
  try {
    const view = await invoke<RemoteView | null>("get_fleet");
    if (view) {
      renderFleet(view);
      fitWindowToContent();
    }
  } catch (e) {
    console.error("get_fleet a echoue", e);
  }
}

listen<RemoteView>("tray://fleet", (event) => {
  renderFleet(event.payload);
  fitWindowToContent();
}).catch((e) => console.error("listen tray://fleet a echoue", e));

void refreshAgents();
void refreshFleet();

// ---- Fenetre compacte (5 colonnes, 520x360) / complete (8 colonnes, ~1000x420) -----------
// Amendement A4, point 1. Le choix est memorise (localStorage, meme mecanique que le theme) et
// applique aux DEUX bouts : cote rendu (colonnes affichees, render.ts) et cote fenetre native
// (taille + reancrage sous l'icone, commande Rust `set_window_mode`).
const COMPACT_KEY = "iakaam-compact";

function loadCompactPref(): boolean {
  try {
    // Pas de valeur memorisee -> mode par defaut = compact (meme taille que tauri.conf.json).
    return localStorage.getItem(COMPACT_KEY) !== "0";
  } catch {
    return true;
  }
}

function saveCompactPref(value: boolean): void {
  try {
    localStorage.setItem(COMPACT_KEY, value ? "1" : "0");
  } catch {
    /* localStorage indisponible : le choix ne survit pas au redemarrage de l'app. */
  }
}

function applyExpandButtonLabel(compact: boolean): void {
  const btn = document.getElementById("expandBtn");
  if (!btn) return;
  btn.textContent = compact ? "⤢" : "⤡";
  btn.title = compact ? "Agrandir (tableau complet)" : "Réduire (vue compacte)";
}

function setWindowMode(compact: boolean): void {
  invoke("set_window_mode", { compact }).catch((e) => console.error("set_window_mode a echoue", e));
}

let compactMode = loadCompactPref();
setCompactMode(compactMode);
applyExpandButtonLabel(compactMode);
setWindowMode(compactMode);

document.getElementById("expandBtn")?.addEventListener("click", () => {
  compactMode = !compactMode;
  setCompactMode(compactMode); // repeint avec le nouveau jeu de colonnes
  applyExpandButtonLabel(compactMode);
  saveCompactPref(compactMode);
  setWindowMode(compactMode); // bascule la largeur/hauteur cote Rust vers le mode vise
  fitWindowToContent(); // puis affine la largeur sur le contenu reellement repeint (A6)
});

// ---- Taille du texte : trois crans 14 / 16 / 18 px (ce-poste-vue-locale-lisibilite.md D6.3) ----
// Memorise comme le theme (localStorage, meme mecanique) ; `data-size` sur <html> pilote l'echelle
// `--sc-fs-*` de styles.css. Defaut : m (16 px). Le changement ré-ajuste la fenetre (A6/A7) une
// fois la mise en page refaite.
const SIZE_KEY = "iakaam-size";
type TextSize = "s" | "m" | "l";

function loadSizePref(): TextSize {
  try {
    const v = localStorage.getItem(SIZE_KEY);
    return v === "s" || v === "l" ? v : "m";
  } catch {
    return "m";
  }
}

function applySize(size: TextSize, persist: boolean): void {
  document.documentElement.setAttribute("data-size", size);
  document.querySelectorAll<HTMLButtonElement>("#sizeSeg button").forEach((b) => {
    b.setAttribute("aria-pressed", String(b.dataset.size === size));
  });
  if (persist) {
    try {
      localStorage.setItem(SIZE_KEY, size);
    } catch {
      /* localStorage indisponible : le cran ne survit pas au redemarrage de l'app. */
    }
    // Deux images : laisser le navigateur refaire la mise en page avant de mesurer le contenu.
    requestAnimationFrame(() => requestAnimationFrame(fitWindowToContent));
  }
}

applySize(loadSizePref(), false);
document.querySelectorAll<HTMLButtonElement>("#sizeSeg button").forEach((b) => {
  b.addEventListener("click", () => applySize(b.dataset.size as TextSize, true));
});

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
