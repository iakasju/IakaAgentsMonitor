//! IakaAgentsMonitor — GUI tray (Tauri v2) : qui travaille en ce moment, pour quel projet.
//! Squelette (etape 1 de specs/instructions/liste-des-agents.md) : sera complete par les etapes
//! suivantes (claude_paths, agents, tray).

/// Point d'entree de l'app (appele par `main.rs`).
pub fn run() {
    tauri::Builder::default()
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("erreur au build de l'app Tauri");
}
