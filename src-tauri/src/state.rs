//! state — état applicatif partagé (managé par Tauri) : l'instantané courant des agents en
//! cours, écrit par le watcher (`agents::run_watcher`) toutes les 5 s, lu par la commande
//! `get_running_agents` à l'ouverture de la fenêtre.

use std::sync::Mutex;

use crate::agents::AgentsSnapshot;

/// Etat applicatif partagé (manage Tauri).
pub struct AppState {
    /// Instantané des agents Claude Code en cours. Le `Mutex` sérialise le watcher (écrit à
    /// chaque tick) et la commande `get_running_agents` (lit).
    pub running_agents: Mutex<AgentsSnapshot>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            running_agents: Mutex::new(AgentsSnapshot::default()),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}
