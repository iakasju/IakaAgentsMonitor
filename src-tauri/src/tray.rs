//! tray — icône de barre de menus (Q5-A) : icône FIXE monochrome (mode « template » macOS),
//! titre texte = effectif d'agents en cours (`set_title`, DA-2 : toujours affiché, `0` compris).
//! Aucune rasterisation SVG, aucune dépendance de rendu d'icône. Clic gauche = ouvre DIRECTEMENT
//! la fenêtre, ancrée sous l'icône (amendement A3, point 9) ; menu clic droit « Afficher » /
//! « Quitter ». Amendement A4 (point 1) : la fenêtre bascule entre un mode compact (520×360,
//! 5 colonnes) et un mode complet (1000×420, 8 colonnes), commande `set_window_mode` appelée par
//! le bouton « agrandir » du frontend.
//!
//! Provenance : patron repris d'iakaTokenCounter (`src-tauri/src/tray.rs`, branche
//! `worktree-agent-a73ae73650ed5e328`, commit `1c9fd0e`), simplifié : plus de composition
//! d'icône (logo + reservoirs), plus de tooltip de quota — juste le compteur en titre.

use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, Position, Rect, WebviewWindow};

use crate::state::AppState;

/// Taille de la fenêtre en mode compact (amendement A4, point 1) : 5 colonnes
/// (Agent · Projet · Runner · Actif · Reste), même valeur que `tauri.conf.json`.
const COMPACT_SIZE: (f64, f64) = (520.0, 360.0);
/// Taille de la fenêtre en mode complet (amendement A4, point 1) : 8 colonnes
/// (Agent · Projet · Runner · Modèle · Tâche · Démarré · Actif · Reste).
const EXPANDED_SIZE: (f64, f64) = (1000.0, 420.0);

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

/// Calcule la position (x, y) qui ancre la fenêtre sous l'icône : bord droit aligné sur le bord
/// droit de l'icône (convention macOS des apps de barre de menus), quelques pixels sous elle —
/// jamais à gauche de l'écran (`.max(0.0)`). Fonction PURE (aucun type Tauri), donc testable
/// indépendamment d'une fenêtre/tray réels ; toutes les coordonnées doivent être dans la même
/// unité (physique ou logique, peu importe pourvu qu'elle soit cohérente d'un appel à l'autre).
/// Propriété clé de l'amendement A4, point 1 (couverte par les tests ci-dessous) : le bord droit
/// résultant ne dépend PAS de `win_w` — agrandir/réduire la fenêtre ne doit jamais la décaler par
/// rapport à l'icône, seul son bord gauche bouge.
fn anchor_under_icon(icon_x: f64, icon_y: f64, icon_w: f64, icon_h: f64, win_w: f64) -> (f64, f64) {
    let x = (icon_x + icon_w - win_w).max(0.0);
    let y = icon_y + icon_h + 4.0;
    (x, y)
}

/// Ancre la fenêtre juste sous l'icône (amendement A3, point 9), pour la largeur logique
/// `win_width_logical` donnée. Best-effort : une erreur de positionnement (facteur d'échelle
/// indisponible…) n'empêche jamais l'ouverture, elle garde juste l'emplacement précédent.
///
/// Prend la largeur en PARAMÈTRE plutôt que de la relire via `window.outer_size()` — piège
/// rencontré en recette (amendement A4, point 1) : juste après un `set_size()` (bouton
/// « agrandir »), `outer_size()` peut encore renvoyer l'ANCIENNE taille (l'ordre de prise d'effet
/// entre les deux n'est pas garanti côté AppKit), ce qui décalait le réancrage. L'appelant connaît
/// toujours la largeur qu'il vient de demander (ou qui est déjà en place) : `toggle_window` et
/// `set_window_mode` la lisent dans `AppState::compact`.
fn position_window_under_tray(window: &WebviewWindow, icon_rect: Rect, win_width_logical: f64) {
    let Ok(scale) = window.scale_factor() else {
        return;
    };
    let icon_pos: PhysicalPosition<f64> = icon_rect.position.to_physical(scale);
    let icon_size: tauri::PhysicalSize<f64> = icon_rect.size.to_physical(scale);
    let (x, y) = anchor_under_icon(
        icon_pos.x,
        icon_pos.y,
        icon_size.width,
        icon_size.height,
        win_width_logical * scale,
    );
    let _ = window.set_position(Position::Physical(PhysicalPosition {
        x: x.round() as i32,
        y: y.round() as i32,
    }));
}

/// Largeur logique courante de la fenêtre (amendement A4, point 1), déduite du mode mémorisé dans
/// `AppState` plutôt que relue sur la fenêtre (cf. doc de `position_window_under_tray`).
fn current_width(app: &AppHandle) -> f64 {
    let compact = *app.state::<AppState>().compact.lock().unwrap();
    if compact { COMPACT_SIZE.0 } else { EXPANDED_SIZE.0 }
}

/// Clic gauche sur l'icône : ouvre DIRECTEMENT la fenêtre (amendement A3, point 9 — plus de
/// popover intermédiaire), ancrée sous l'icône ; un second clic la masque (bascule). Mémorise
/// aussi le rect de l'icône (amendement A4, point 1) : `set_window_mode` le réutilise pour
/// réancrer la fenêtre après un changement de taille déclenché depuis le bouton « agrandir », qui
/// lui ne reçoit aucun rect.
fn toggle_window(app: &AppHandle, icon_rect: Rect) {
    app.state::<AppState>().last_tray_rect.lock().unwrap().replace(icon_rect);
    if let Some(w) = app.get_webview_window("popover") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            position_window_under_tray(&w, icon_rect, current_width(app));
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

/// Bascule la taille de la fenêtre entre les deux modes de l'amendement A4 (point 1). Appelée par
/// le bouton « agrandir » du frontend (le choix — compact/complet — est mémorisé côté webview,
/// `localStorage`, pas ici ; `AppState::compact` n'existe que pour réancrer, cf. doc de
/// `position_window_under_tray`). Réancre sous l'icône avec le dernier rect connu (mémorisé à
/// chaque clic sur l'icône par `toggle_window`) ; sans rect connu (avant tout premier clic),
/// redimensionne seulement — la position sera correcte au premier clic, qui repositionne toujours
/// la fenêtre.
#[tauri::command]
pub fn set_window_mode(app: AppHandle, compact: bool) {
    let Some(window) = app.get_webview_window("popover") else {
        return;
    };
    let (width, height) = if compact { COMPACT_SIZE } else { EXPANDED_SIZE };
    let _ = window.set_size(LogicalSize::new(width, height));
    *app.state::<AppState>().compact.lock().unwrap() = compact;

    let rect = *app.state::<AppState>().last_tray_rect.lock().unwrap();
    if let Some(rect) = rect {
        position_window_under_tray(&window, rect, width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_under_icon_aligne_le_bord_droit_sur_celui_de_l_icone() {
        let (x, y) = anchor_under_icon(1586.0, 6.0, 102.0, 48.0, 1040.0);
        assert_eq!(x, 1586.0 + 102.0 - 1040.0);
        assert_eq!(y, 6.0 + 48.0 + 4.0);
    }

    #[test]
    fn anchor_under_icon_jamais_a_gauche_de_l_ecran() {
        // Fenêtre plus large que la distance entre l'icône et le bord gauche de l'écran : bornée
        // à 0, jamais négative (l'ancienne fenêtre "hors écran" serait pire que mal ancrée).
        let (x, _) = anchor_under_icon(100.0, 0.0, 50.0, 20.0, 2000.0);
        assert_eq!(x, 0.0);
    }

    #[test]
    fn anchor_under_icon_meme_bord_droit_quelle_que_soit_la_largeur_de_fenetre() {
        // Propriété centrale de l'amendement A4, point 1 : agrandir (bouton "agrandir") ou
        // réduire la fenêtre ne doit JAMAIS décaler son bord droit par rapport à l'icône — seul
        // le bord gauche bouge. Vérifié ici en pur Rust, faute de pouvoir simuler de façon fiable
        // un vrai clic gauche matériel sur l'icône depuis un script d'automatisation (l'un des
        // deux chemins de code qui appellent `anchor_under_icon`, `toggle_window`, n'est déclenché
        // que par ce clic réel — cf. jalon de remise).
        // icon_x assez grand pour que ni la fenêtre compacte ni l'élargie ne butent sur le bord
        // gauche de l'écran (cf. test précédent pour le cas où ça arrive).
        let (icon_x, icon_y, icon_w, icon_h) = (1200.0, 6.0, 50.0, 20.0);
        let (x_compact, _) = anchor_under_icon(icon_x, icon_y, icon_w, icon_h, 520.0);
        let (x_expanded, _) = anchor_under_icon(icon_x, icon_y, icon_w, icon_h, 1000.0);
        assert_eq!(x_compact + 520.0, icon_x + icon_w);
        assert_eq!(x_expanded + 1000.0, icon_x + icon_w);
    }
}
