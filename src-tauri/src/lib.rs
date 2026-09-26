//! IakaAgentsMonitor — GUI tray (Tauri v2) : qui travaille en ce moment, pour quel projet, pour
//! qui. App autonome, en lecture seule sur les transcripts Claude Code déjà écrits localement.
//! Aucun service, aucun bus, aucun port. Cf. specs/instructions/liste-des-agents.md.

mod agents;
mod claude_paths;
mod state;
mod tray;

use state::AppState;

/// Point d'entree de l'app (appele par `main.rs`).
pub fn run() {
    let app_state = AppState::new();

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![agents::get_running_agents])
        .setup(move |app| {
            // macOS : app tray-only. Politique d'activation `Accessory` (equiv. LSUIElement) =>
            // pas d'icone au Dock ni d'entree dans le selecteur d'apps (Cmd-Tab).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            tray::build_tray(&handle)?;

            // Agents Claude Code en cours : watcher dedie, aucune persistance (l'instantane se
            // reconstruit a chaud depuis les sidecars a chaque tick).
            agents::start_watcher(handle.clone());

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
