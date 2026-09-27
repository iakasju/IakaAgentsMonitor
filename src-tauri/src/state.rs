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
    /// Mode courant de la fenêtre (amendement A4, point 1) : `true` = compacte, `false` =
    /// complète. Sert à savoir QUELLE largeur adaptative utiliser (`compact_width` /
    /// `expanded_width`) pour réancrer la fenêtre au prochain clic sur l'icône
    /// (`toggle_window`) — jamais interrogé via `WebviewWindow::outer_size()` juste après un
    /// `set_size()`, qui peut encore renvoyer l'ANCIENNE taille (les deux passent par le même
    /// event loop mais l'ordre de prise d'effet n'est pas garanti côté AppKit).
    pub compact: Mutex<bool>,
    /// Largeur adaptative courante du mode compact (amendement A6, point 1 : la largeur s'ajuste
    /// au contenu réellement affiché, bornée 480–1200 px, plutôt que figée). Valeur de départ =
    /// l'ancienne taille fixe (`tauri.conf.json`), avant toute mesure du contenu réel côté
    /// webview ; recalculée par `tray::fit_window_width`.
    pub compact_width: Mutex<f64>,
    /// Idem pour le mode complet (amendement A4 : 1000 px de départ).
    pub expanded_width: Mutex<f64>,
    /// Hauteur adaptative courante du mode compact (amendement A7, point 1 : même mécanique que
    /// la largeur — la hauteur s'ajuste au contenu réellement affiché, bornée 200 px–80 % de la
    /// hauteur d'écran, plutôt que figée). Valeur de départ = l'ancienne hauteur fixe (360 px),
    /// avant toute mesure du contenu réel côté webview ; recalculée par `tray::fit_window_height`.
    pub compact_height: Mutex<f64>,
    /// Idem pour le mode complet (420 px de départ, ancienne hauteur fixe du mode complet).
    pub expanded_height: Mutex<f64>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            running_agents: Mutex::new(AgentsSnapshot::default()),
            last_tray_rect: Mutex::new(None),
            compact: Mutex::new(true),
            compact_width: Mutex::new(520.0),
            expanded_width: Mutex::new(1000.0),
            compact_height: Mutex::new(360.0),
            expanded_height: Mutex::new(420.0),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}
