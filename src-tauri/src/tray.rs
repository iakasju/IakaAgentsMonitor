//! tray — icône de barre de menus (Q5-A) : icône FIXE monochrome (mode « template » macOS),
//! titre texte = effectif d'agents en cours (`set_title`, DA-2 : toujours affiché, `0` compris).
//! Aucune rasterisation SVG, aucune dépendance de rendu d'icône. Clic gauche = ouvre DIRECTEMENT
//! la fenêtre, ancrée sous l'icône (amendement A3, point 9) ; menu clic droit « Afficher » /
//! « Quitter ».
//!
//! Provenance : patron repris d'iakaTokenCounter (`src-tauri/src/tray.rs`, branche
//! `worktree-agent-a73ae73650ed5e328`, commit `1c9fd0e`), simplifié : plus de composition
//! d'icône (logo + reservoirs), plus de tooltip de quota — juste le compteur en titre.

use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition, Position, Rect, WebviewWindow};

/// Identifiant du tray, pour le retrouver via `app.tray_by_id`.
pub const TRAY_ID: &str = "main";

/// Construit l'icône tray dans le hook `setup`.
pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItemBuilder::with_id("show", "Afficher").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quitter").build(app)?;
    let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("0 agent en cours")
        .menu(&menu)
        // Réserve le clic gauche à la fenêtre (le menu ne s'ouvre qu'au clic droit).
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                toggle_window(tray.app_handle(), rect);
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    let tray = builder.build(app)?;
    // Icone fixe monochrome (modele macOS, Q5-A) : le systeme la teinte selon le theme/l'etat,
    // aucun rendu custom.
    let _ = tray.set_icon_as_template(true);
    // Titre affiche toujours l'effectif, 0 compris (DA-2) : pas de bascule de largeur a
    // l'apparition du premier agent.
    let _ = tray.set_title(Some("0"));
    Ok(())
}

/// Met à jour le titre du tray = effectif courant (DA-2 : toujours affiché, `0` compris) +
/// tooltip lisible. Appelé par `agents::run_watcher` à chaque changement d'instantané.
pub fn update_title(app: &AppHandle, count: u32) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let _ = tray.set_title(Some(count.to_string()));
    let label = if count == 1 {
        "1 agent en cours".to_string()
    } else {
        format!("{count} agents en cours")
    };
    let _ = tray.set_tooltip(Some(label));
}

fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("popover") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Ancre la fenêtre juste sous l'icône (amendement A3, point 9) : bord droit aligné sur le bord
/// droit de l'icône (convention macOS des apps de barre de menus), quelques pixels sous elle.
/// Best-effort : une erreur de positionnement (taille de fenêtre indisponible…) n'empêche jamais
/// l'ouverture, elle garde juste l'emplacement précédent.
fn position_window_under_tray(window: &WebviewWindow, icon_rect: Rect) {
    let (Ok(win_size), Ok(scale)) = (window.outer_size(), window.scale_factor()) else {
        return;
    };
    let icon_pos: PhysicalPosition<f64> = icon_rect.position.to_physical(scale);
    let icon_size: tauri::PhysicalSize<f64> = icon_rect.size.to_physical(scale);
    let x = (icon_pos.x + icon_size.width - win_size.width as f64).max(0.0);
    let y = icon_pos.y + icon_size.height + 4.0;
    let _ = window.set_position(Position::Physical(PhysicalPosition {
        x: x.round() as i32,
        y: y.round() as i32,
    }));
}

/// Clic gauche sur l'icône : ouvre DIRECTEMENT la fenêtre (amendement A3, point 9 — plus de
/// popover intermédiaire), ancrée sous l'icône ; un second clic la masque (bascule).
fn toggle_window(app: &AppHandle, icon_rect: Rect) {
    if let Some(w) = app.get_webview_window("popover") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            position_window_under_tray(&w, icon_rect);
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}
