//! watcher — assemble les DEUX sources (specs/instructions/agnostique-mqtt.md, étape 7) selon la
//! bascule de repli (Q3-A, repli STRICT) et pousse l'instantané construit par le cœur neutre
//! (`tree::build_snapshot`) à la webview. Seul endroit qui décide QUELLE source alimente l'app à
//! un instant donné — ni `tree`, ni les sources elles-mêmes, ne connaissent cette décision.

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

use crate::labels;
use crate::roster::Roster;
use crate::sources::{bus::BusSource, claude_code_files};
use crate::state::AppState;
use crate::tree::{self, AgentsSnapshot};

/// Nom d'événement poussé à la webview à chaque changement d'instantané agents.
pub const AGENTS_EVENT: &str = "tray://agents";

/// Cadence du watcher dédié.
const TICK: Duration = Duration::from_secs(5);

/// Variable d'environnement de seuil de liveness (DA-1 de liste-des-agents.md).
const LIVENESS_ENV: &str = "IAKAAM_LIVENESS_SECS";
const DEFAULT_LIVENESS_SECS: u64 = 90;

/// Seuil de liveness (s), lu depuis `IAKAAM_LIVENESS_SECS`. Valeur absente/invalide -> défaut 90.
pub fn liveness_secs_from_env() -> u64 {
    std::env::var(LIVENESS_ENV)
        .ok()
        .filter(|s| !s.is_empty())
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_LIVENESS_SECS)
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Commande : instantané des agents en cours à l'ouverture de la fenêtre. Lit l'instantané déjà
/// posé par le watcher dans `AppState` : pas de re-scan disque/bus synchrone au moment de l'appel.
#[tauri::command]
pub fn get_running_agents(state: tauri::State<'_, AppState>) -> AgentsSnapshot {
    state.running_agents.lock().unwrap().clone()
}

/// Lance la source bus + le watcher dédié dans un thread détaché.
pub fn start_watcher(app: AppHandle) {
    let bus = BusSource::start(crate::sources::bus::port_from_env());
    std::thread::spawn(move || run_watcher(app, bus));
}

/// Libellé de source affiché dans l'infobulle du tray (étape 7).
pub fn source_label(bus_connected: bool) -> &'static str {
    if bus_connected {
        "bus"
    } else {
        "fichiers (bus absent)"
    }
}

fn run_watcher(app: AppHandle, bus: BusSource) {
    let labels = labels::load_labels();
    let roster = Roster::new();
    let mut last: Option<AgentsSnapshot> = None;
    let mut last_source: &str = "";

    loop {
        let now = now_secs();
        let liveness = liveness_secs_from_env() as i64;

        // Bascule de repli STRICTE (Q3-A) : broker joignable -> source bus SEULE ; broker
        // injoignable -> source fichiers SEULE. Jamais de fusion des deux.
        let bus_connected = bus.is_connected();
        let cards = if bus_connected {
            bus.cards()
        } else {
            claude_code_files::claude_projects_dir()
                .map(|dir| claude_code_files::cards_from_dir(&dir))
                .unwrap_or_default()
        };
        let source = source_label(bus_connected);

        let snap = tree::build_snapshot(&cards, now, liveness, &labels, &roster);
        {
            let state = app.state::<AppState>();
            *state.running_agents.lock().unwrap() = snap.clone();
        }
        if last.as_ref() != Some(&snap) || last_source != source {
            let _ = app.emit(AGENTS_EVENT, &snap);
            crate::tray::update_title(&app, snap.count, source);
            last = Some(snap);
            last_source = source;
        }
        std::thread::sleep(TICK);
    }
}
