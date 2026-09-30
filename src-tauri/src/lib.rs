//! IakaAgentsMonitor — GUI tray (Tauri v2) : qui travaille en ce moment, pour quel projet, pour
//! qui. Agnostique du runner (specs/instructions/agnostique-mqtt.md) : les fiches d'agent
//! arrivent soit par le bus MQTT local (tout runner qui publie sur `iaka/<node>/out/agents/…`), soit,
//! en repli quand le broker est absent, par l'adaptateur Claude Code (lecture des transcripts
//! déjà écrits localement). Aucun autre service, aucun port en écoute côté app.

pub mod card;
mod claude_paths;
pub mod contract;
pub mod estimate;
pub mod fleet_config;
pub mod fleet_model;
pub mod labels;
pub mod roster;
pub mod sources;
mod state;
mod tray;
pub mod tree;
mod watcher;

use state::AppState;

/// Point d'entree de l'app (appele par `main.rs`).
pub fn run() {
    let app_state = AppState::new();

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            watcher::get_running_agents,
            watcher::get_fleet,
            tray::set_window_mode,
            tray::fit_window_width,
            tray::fit_window_height
        ])
        .setup(move |app| {
            // macOS : app tray-only. Politique d'activation `Accessory` (equiv. LSUIElement) =>
            // pas d'icone au Dock ni d'entree dans le selecteur d'apps (Cmd-Tab).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            tray::build_tray(&handle)?;

            // Agents en cours : watcher dedie (source bus + repli fichiers), aucune persistance
            // (l'instantane se reconstruit a chaud a chaque tick).
            watcher::start_watcher(handle.clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            // App tray persistante : la fenetre se **masque** au lieu de quitter.
            if window.label() == "popover" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("erreur au build de l'app Tauri")
        .run(|_app, _event| {});
}
