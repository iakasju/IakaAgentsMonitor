//! watcher — assemble les sources de « Ce poste » selon la règle D2 de
//! `specs/instructions/ce-poste-vue-locale-lisibilite.md` (la session fait foi, les fiches la
//! peuplent : fichiers de session du démon + fiches du publieur, unis aux fiches du bus local s'il
//! est joint ; repli par transcripts POUR UNE SESSION sans fiche, jamais pour le répertoire entier)
//! et pousse l'instantané construit par le cœur neutre (`tree`) à la webview. Remplace la bascule
//! stricte bus-OU-fichiers (Q3-A de `agnostique-mqtt.md`, remplacée par cette union). Seul endroit
//! qui décide QUELLES sources alimentent l'app à un instant donné.

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

use crate::labels;
use crate::roster::Roster;
use crate::local_view;
use crate::sources::{
    bus::BusSource,
    claude_code_files,
    local_sessions::{self, LocalInput},
    remote::{RemoteSource, RemoteView},
};
use crate::state::AppState;
use crate::tree::AgentsSnapshot;

/// Nom d'événement poussé à la webview à chaque changement d'instantané agents.
pub const AGENTS_EVENT: &str = "tray://agents";

/// Nom d'événement poussé à la webview quand la vue « Autres machines » change.
pub const FLEET_EVENT: &str = "tray://fleet";

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

/// Commande : vue « Autres machines » courante (central). Lit ce que le watcher a posé.
#[tauri::command]
pub fn get_fleet(state: tauri::State<'_, AppState>) -> Option<RemoteView> {
    state.remote_fleet.lock().unwrap().clone()
}

/// Lance la source bus + le watcher dédié dans un thread détaché.
pub fn start_watcher(app: AppHandle) {
    let bus = BusSource::start(crate::sources::bus::port_from_env());
    // Central : `~/.iakafleet/.env` + environnement. Sans identifiants → aucune connexion, le
    // Monitor marche seul (CA-7) ; configuration refusée ou broker injoignable → dit, jamais vide.
    let cfg = crate::fleet_config::load();
    let remote = RemoteSource::start(&cfg.remote, cfg.node);
    std::thread::spawn(move || run_watcher(app, bus, remote));
}

/// Libellé de source affiché dans l'infobulle du tray : les fichiers du poste sont TOUJOURS lus
/// (la session fait foi) ; le bus local s'y ajoute quand il est joint (union `D2`, plus de bascule).
pub fn source_label(bus_connected: bool) -> &'static str {
    if bus_connected {
        "fichiers + bus"
    } else {
        "fichiers (bus absent)"
    }
}

fn run_watcher(app: AppHandle, bus: BusSource, remote: RemoteSource) {
    let labels = labels::load_labels();
    let roster = Roster::new();
    let node = crate::fleet_config::load().node;
    let projects_dir = claude_code_files::claude_projects_dir();
    let sessions_dir = local_sessions::sessions_dir();
    let cards_dir = local_sessions::cards_dir();
    let mut last: Option<AgentsSnapshot> = None;
    let mut last_source: &str = "";
    let mut last_fleet: Option<RemoteView> = None;

    loop {
        let now = now_secs();
        let liveness = liveness_secs_from_env() as i64;

        // D2 : sessions vivantes (pid) + fiches du disque, UNIES aux fiches du bus local s'il est
        // joint. Coût O(sessions vivantes) : aucun parcours de `~/.claude/projects` (D3).
        let bus_connected = bus.is_connected();
        let sessions = sessions_dir
            .as_deref()
            .and_then(|d| local_sessions::read_sessions(d, &local_sessions::pid_alive));
        let notice = local_notice(sessions_dir.as_deref(), sessions.is_some());
        let sessions = sessions.unwrap_or_default();
        let disk_cards = cards_dir.as_deref().map(local_sessions::read_disk_cards).unwrap_or_default();
        let bus_cards = if bus_connected { bus.cards() } else { Vec::new() };
        let mut fallback = |s: &local_sessions::LocalSession| match (&projects_dir, s.cwd.as_deref(), s.session_id.as_deref()) {
            (Some(dir), Some(cwd), Some(sid)) => claude_code_files::cards_for_session(dir, cwd, sid),
            _ => Vec::new(),
        };
        let outcome = local_sessions::build_local_snapshot(
            LocalInput { sessions: &sessions, disk_cards, bus_cards },
            &mut fallback,
            now,
            liveness,
            &labels,
            &roster,
        );
        let source = source_label(bus_connected);

        let mut snap = outcome.snapshot;
        let daemon = remote.own_daemon_label(now);
        snap.header = Some(local_view::build_header(node.as_deref(), &snap, &daemon, notice));
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

        // « Autres machines » : posé à chaque tick, poussé seulement quand la vue change.
        let fleet = remote.view(now);
        {
            let state = app.state::<AppState>();
            *state.remote_fleet.lock().unwrap() = Some(fleet.clone());
        }
        if last_fleet.as_ref() != Some(&fleet) {
            let _ = app.emit(FLEET_EVENT, &fleet);
            last_fleet = Some(fleet);
        }
        std::thread::sleep(TICK);
    }
}

/// Avertissement de la source locale quand elle est dégradée : le vide ne se déguise jamais en
/// « aucune session ».
fn local_notice(dir: Option<&std::path::Path>, dir_ok: bool) -> Option<String> {
    if !cfg!(unix) {
        return Some("vivacité des sessions non prise en charge sur ce système".to_string());
    }
    match (dir, dir_ok) {
        (None, _) => Some("dossier personnel introuvable : sessions non lues".to_string()),
        (Some(d), false) => Some(format!("répertoire de sessions illisible ({})", d.display())),
        (Some(_), true) => None,
    }
}
