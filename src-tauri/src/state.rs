//! state — état applicatif partagé (managé par Tauri) : l'instantané courant des agents en
//! cours, écrit par le watcher (`agents::run_watcher`) toutes les 5 s, lu par la commande
//! `get_running_agents` à l'ouverture de la fenêtre.

use std::sync::Mutex;

use tauri::Rect;

use crate::agents::AgentsSnapshot;

/// Etat applicatif partagé (manage Tauri).
pub struct AppState {
    /// Instantané des agents Claude Code en cours. Le `Mutex` sérialise le watcher (écrit à
    /// chaque tick) et la commande `get_running_agents` (lit).
    pub running_agents: Mutex<AgentsSnapshot>,
    /// Dernier rect connu de l'icône de barre de menus (amendement A4, point 1). Mémorisé à
    /// chaque clic sur l'icône (`tray::toggle_window`) ; réutilisé par `tray::set_window_mode`
    /// pour réancrer la fenêtre sous l'icône après un changement de taille déclenché depuis le
    /// bouton « agrandir » du frontend, qui lui ne reçoit aucun rect (il ne vient pas d'un
    /// événement sur l'icône).
    pub last_tray_rect: Mutex<Option<Rect>>,
    /// Mode courant de la fenêtre (amendement A4, point 1) : `true` = compacte (520×360, valeur
    /// par défaut, cohérente avec `tauri.conf.json`), `false` = complète (1000×420). Sert
    /// UNIQUEMENT à connaître la largeur à utiliser pour réancrer la fenêtre au prochain clic sur
    /// l'icône (`toggle_window`) — jamais interrogé via `WebviewWindow::outer_size()` juste après
    /// un `set_size()`, qui peut encore renvoyer l'ANCIENNE taille (les deux passent par le même
    /// event loop mais l'ordre de prise d'effet n'est pas garanti côté AppKit).
    pub compact: Mutex<bool>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            running_agents: Mutex::new(AgentsSnapshot::default()),
            last_tray_rect: Mutex::new(None),
            compact: Mutex::new(true),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}
