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
