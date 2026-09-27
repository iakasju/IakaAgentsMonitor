//! tray — icône de barre de menus (Q5-A) : icône FIXE monochrome (mode « template » macOS),
//! titre texte = effectif d'agents en cours (`set_title`, DA-2 : toujours affiché, `0` compris).
//! Aucune rasterisation SVG, aucune dépendance de rendu d'icône. Clic gauche = ouvre DIRECTEMENT
//! la fenêtre, ancrée sous l'icône (amendement A3, point 9) ; menu clic droit « Afficher » /
//! « Quitter ». Amendement A4 (point 1) : la fenêtre bascule entre un mode compact (5 colonnes) et
//! un mode complet (8 colonnes), commande `set_window_mode` appelée par le bouton « agrandir » du
//! frontend. Amendement A6 (point 1) : dans les deux modes, la LARGEUR n'est plus figée — elle
//! s'ajuste au contenu réellement affiché (mesuré côté webview, seule couche qui connaît le rendu
//! réel des polices), bornée 480–1200 px, sans jamais bouger le bord droit de la fenêtre par
//! rapport à l'icône (`anchor_under_icon`, inchangée depuis A4). Seule la HAUTEUR reste fixe par
//! mode.
//!
//! Provenance : patron repris d'iakaTokenCounter (`src-tauri/src/tray.rs`, branche
//! `worktree-agent-a73ae73650ed5e328`, commit `1c9fd0e`), simplifié : plus de composition
//! d'icône (logo + reservoirs), plus de tooltip de quota — juste le compteur en titre.
//!
//! Amendement A7 (point 1) : la HAUTEUR suit désormais la même mécanique que la largeur (A6,
//! point 1) — ajustée au contenu réellement affiché, bornée 200 px–80 % de la hauteur d'écran du
//! moniteur courant, même hystérésis, défilement vertical (CSS) au-delà du maximum. Les deux
//! dimensions sont donc adaptatives par mode (compact/complet), mémorisées séparément dans
//! `AppState`.

use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, Position, Rect, WebviewWindow};

use crate::state::AppState;

/// Bornes de la largeur adaptative de la fenêtre (amendement A6, point 1), dans les DEUX modes.
const WIN_WIDTH_MIN: f64 = 480.0;
const WIN_WIDTH_MAX: f64 = 1200.0;

/// Bornes de la hauteur adaptative de la fenêtre (amendement A7, point 1). Le maximum n'est pas
/// une constante fixe : il dépend de l'écran (`WIN_HEIGHT_MAX_PCT` de sa hauteur), cf.
/// `max_height_from_screen`.
const WIN_HEIGHT_MIN: f64 = 200.0;
const WIN_HEIGHT_MAX_PCT: f64 = 0.8;
/// Hauteur d'écran de repli (px logiques) quand le moniteur n'est pas détectable (rare, best
/// effort) — évite un plafond de hauteur absent plutôt qu'une valeur exacte.
const FALLBACK_SCREEN_HEIGHT: f64 = 900.0;

/// Écart minimal (px logiques) entre la dimension actuellement appliquée et la dimension mesurée
/// du contenu pour déclencher un VRAI redimensionnement (amendement A6, point 1 pour la largeur ;
/// A7, point 1 : MÊME hystérésis pour la hauteur). Hystérésis documentée : `agents.rs` n'émet
/// `tray://agents` que si l'instantané change réellement, mais la colonne « Actif il y a » change
/// pourtant à quasi chaque tick (5 s) tant qu'un agent écrit — sans ce seuil, la mesure du
/// contenu (qui varie de quelques px d'un tick à l'autre, arrondi de police/sous-pixel)
/// redimensionnerait la fenêtre en continu. Sous ce seuil, on garde la taille en place ; au-delà,
/// c'est un vrai changement de contenu (agent apparu/disparu, tâche plus longue…) qu'on suit.
const RESIZE_THRESHOLD_PX: f64 = 12.0;

/// Borne la largeur mesurée du contenu dans la fourchette de l'amendement A6 (point 1).
fn clamp_window_width(natural: f64) -> f64 {
    natural.clamp(WIN_WIDTH_MIN, WIN_WIDTH_MAX)
}

/// Décide si l'écart entre la largeur actuellement appliquée et la largeur cible (déjà bornée)
/// justifie un redimensionnement réel — hystérésis de l'amendement A6, point 1 (cf. doc de
/// `RESIZE_THRESHOLD_PX`). Symétrique : un rétrécissement suffisant redimensionne aussi.
fn should_resize_width(current: f64, target: f64) -> bool {
    (target - current).abs() > RESIZE_THRESHOLD_PX
}

/// Hauteur maximale (px logiques) pour une hauteur d'écran donnée (amendement A7, point 1) :
/// 80 % de la hauteur d'écran, jamais moins que le minimum de la fourchette. Fonction PURE
/// (aucun type Tauri), donc testable indépendamment d'un moniteur réel.
fn max_height_from_screen(screen_height: f64) -> f64 {
    (screen_height * WIN_HEIGHT_MAX_PCT).max(WIN_HEIGHT_MIN)
}

/// Borne la hauteur mesurée du contenu dans la fourchette de l'amendement A7 (point 1) : minimum
/// fixe (200 px), maximum dépendant de l'écran (déjà calculé par `max_height_from_screen`).
fn clamp_window_height(natural: f64, max_height: f64) -> f64 {
    natural.clamp(WIN_HEIGHT_MIN, max_height.max(WIN_HEIGHT_MIN))
}

/// Décide si l'écart entre la hauteur actuellement appliquée et la hauteur cible (déjà bornée)
/// justifie un redimensionnement réel — même hystérésis que la largeur (amendement A7, point 1,
/// cf. doc de `RESIZE_THRESHOLD_PX`).
fn should_resize_height(current: f64, target: f64) -> bool {
    (target - current).abs() > RESIZE_THRESHOLD_PX
}

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

/// Largeur logique courante de la fenêtre (amendement A4, point 1, largeur adaptative depuis A6),
/// déduite du mode et de la largeur mémorisés dans `AppState` plutôt que relue sur la fenêtre
/// (cf. doc de `position_window_under_tray`).
fn current_width(app: &AppHandle) -> f64 {
    let state = app.state::<AppState>();
    let compact = *state.compact.lock().unwrap();
    if compact {
        *state.compact_width.lock().unwrap()
    } else {
        *state.expanded_width.lock().unwrap()
    }
}

/// Hauteur logique courante de la fenêtre (amendement A7, point 1, même mécanique que
/// `current_width`) — jamais relue via `WebviewWindow::outer_size()` (cf. doc de
/// `position_window_under_tray`), toujours déduite d'`AppState`.
fn current_height(app: &AppHandle) -> f64 {
    let state = app.state::<AppState>();
    let compact = *state.compact.lock().unwrap();
    if compact {
        *state.compact_height.lock().unwrap()
    } else {
        *state.expanded_height.lock().unwrap()
    }
}

/// Hauteur maximale (px logiques) pour la fenêtre `popover`, déduite du moniteur qui la porte
/// actuellement (amendement A7, point 1). Best-effort : moniteur introuvable ou facteur d'échelle
/// indisponible -> repli sur `FALLBACK_SCREEN_HEIGHT` plutôt qu'un plafond absent.
fn max_window_height(window: &WebviewWindow) -> f64 {
    let screen_height_logical = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())
        .map(|m| m.size().height as f64 / m.scale_factor())
        .unwrap_or(FALLBACK_SCREEN_HEIGHT);
    max_height_from_screen(screen_height_logical)
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
/// `position_window_under_tray`). Utilise la dernière largeur ADAPTATIVE connue pour le mode
/// visé (amendement A6, point 1 : plus de taille fixe) — `fit_window_width` l'affine ensuite dès
/// la prochaine mesure du contenu réel côté webview. Réancre sous l'icône avec le dernier rect
/// connu (mémorisé à chaque clic sur l'icône par `toggle_window`) ; sans rect connu (avant tout
/// premier clic), redimensionne seulement — la position sera correcte au premier clic, qui
/// repositionne toujours la fenêtre.
#[tauri::command]
pub fn set_window_mode(app: AppHandle, compact: bool) {
    let Some(window) = app.get_webview_window("popover") else {
        return;
    };
    let state = app.state::<AppState>();
    *state.compact.lock().unwrap() = compact;
    let width = current_width(&app);
    let height = current_height(&app);
    let _ = window.set_size(LogicalSize::new(width, height));

    let rect = *state.last_tray_rect.lock().unwrap();
    if let Some(rect) = rect {
        position_window_under_tray(&window, rect, width);
    }
}

/// Ajuste la largeur de la fenêtre au contenu réellement affiché (amendement A6, point 1).
/// Appelée par le frontend après CHAQUE peinture du tableau (mesure DOM du rendu réel, seule
/// couche qui connaît les polices) avec la largeur naturelle mesurée (px logiques) ; agit sur le
/// mode COURANT (`AppState::compact`), jamais sur l'autre mode (une fenêtre cachée ne mesure pas
/// son propre contenu). Borne (`clamp_window_width`) puis n'applique un VRAI redimensionnement
/// que si l'écart dépasse le seuil d'hystérésis (`should_resize_width`) — sinon la fenêtre ne
/// bouge pas, pour éviter tout clignotement à chaque tick (5 s).
#[tauri::command]
pub fn fit_window_width(app: AppHandle, natural_width: f64) {
    let Some(window) = app.get_webview_window("popover") else {
        return;
    };
    let state = app.state::<AppState>();
    let compact = *state.compact.lock().unwrap();
    let target = clamp_window_width(natural_width);

    let applied = {
        let width_lock = if compact {
            &state.compact_width
        } else {
            &state.expanded_width
        };
        let mut current = width_lock.lock().unwrap();
        if !should_resize_width(*current, target) {
            return;
        }
        *current = target;
        target
    };

    let height = current_height(&app);
    let _ = window.set_size(LogicalSize::new(applied, height));

    let rect = *state.last_tray_rect.lock().unwrap();
    if let Some(rect) = rect {
        position_window_under_tray(&window, rect, applied);
    }
}

/// Ajuste la HAUTEUR de la fenêtre au contenu réellement affiché (amendement A7, point 1) — même
/// mécanique que `fit_window_width` (A6, point 1) : mesure envoyée par le frontend à chaque
/// peinture, bornée (200 px–80 % de la hauteur d'écran du moniteur courant, `max_window_height`),
/// hystérésis identique avant tout vrai redimensionnement. Au-delà du maximum, le contenu déborde
/// et défile verticalement (CSS `.agents`, `overflow: auto` — géré côté webview, pas ici). Agit
/// sur le mode COURANT uniquement, comme `fit_window_width`.
#[tauri::command]
pub fn fit_window_height(app: AppHandle, natural_height: f64) {
    let Some(window) = app.get_webview_window("popover") else {
        return;
    };
    let max_height = max_window_height(&window);
    let state = app.state::<AppState>();
    let compact = *state.compact.lock().unwrap();
    let target = clamp_window_height(natural_height, max_height);

    let applied = {
        let height_lock = if compact {
            &state.compact_height
        } else {
            &state.expanded_height
        };
        let mut current = height_lock.lock().unwrap();
        if !should_resize_height(*current, target) {
            return;
        }
        *current = target;
        target
    };

    let width = current_width(&app);
    let _ = window.set_size(LogicalSize::new(width, applied));

    let rect = *state.last_tray_rect.lock().unwrap();
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

    // ------------------------- Amendement A6, point 1 : largeur adaptative -------------------

    #[test]
    fn clamp_window_width_laisse_passer_une_valeur_dans_la_fourchette() {
        assert_eq!(clamp_window_width(700.0), 700.0);
    }

    #[test]
    fn clamp_window_width_borne_au_minimum_480() {
        assert_eq!(clamp_window_width(300.0), 480.0);
    }

    #[test]
    fn clamp_window_width_borne_au_maximum_1200() {
        assert_eq!(clamp_window_width(1500.0), 1200.0);
    }

    #[test]
    fn clamp_window_width_accepte_exactement_les_bornes() {
        assert_eq!(clamp_window_width(480.0), 480.0);
        assert_eq!(clamp_window_width(1200.0), 1200.0);
    }

    #[test]
    fn should_resize_width_ignore_un_ecart_sous_le_seuil_hysteresis() {
        // Bruit de mesure DOM d'un tick à l'autre (arrondi de police) : 5 px < 12 px, ignoré.
        assert!(!should_resize_width(520.0, 525.0));
        assert!(!should_resize_width(520.0, 515.0));
    }

    #[test]
    fn should_resize_width_ignore_pile_le_seuil() {
        // Écart strictement égal au seuil : PAS un redimensionnement (le seuil est exclusif,
        // cf. doc de WIDTH_RESIZE_THRESHOLD_PX — "dépasse le seuil", pas "atteint").
        assert!(!should_resize_width(520.0, 532.0));
    }

    #[test]
    fn should_resize_width_declenche_au_dela_du_seuil() {
        assert!(should_resize_width(520.0, 533.0));
    }

    #[test]
    fn should_resize_width_declenche_symetriquement_en_retrecissement() {
        // Un contenu qui rétrécit nettement (agent parti) doit aussi faire rétrécir la fenêtre.
        assert!(should_resize_width(1000.0, 700.0));
    }

    // ------------------------- Amendement A7, point 1 : hauteur adaptative -------------------

    #[test]
    fn max_height_from_screen_calcule_80_pourcent_de_l_ecran() {
        assert_eq!(max_height_from_screen(1000.0), 800.0);
    }

    #[test]
    fn max_height_from_screen_ne_descend_jamais_sous_le_minimum() {
        // Écran minuscule (improbable) : le plafond ne descend jamais sous WIN_HEIGHT_MIN, sinon
        // clamp_window_height recevrait un maximum inférieur au minimum.
        assert_eq!(max_height_from_screen(100.0), WIN_HEIGHT_MIN);
    }

    #[test]
    fn clamp_window_height_laisse_passer_une_valeur_dans_la_fourchette() {
        assert_eq!(clamp_window_height(300.0, 800.0), 300.0);
    }

    #[test]
    fn clamp_window_height_borne_au_minimum_200() {
        assert_eq!(clamp_window_height(50.0, 800.0), 200.0);
    }

    #[test]
    fn clamp_window_height_borne_au_maximum_donne() {
        assert_eq!(clamp_window_height(1500.0, 800.0), 800.0);
    }

    #[test]
    fn clamp_window_height_accepte_exactement_les_bornes() {
        assert_eq!(clamp_window_height(200.0, 800.0), 200.0);
        assert_eq!(clamp_window_height(800.0, 800.0), 800.0);
    }

    #[test]
    fn should_resize_height_ignore_un_ecart_sous_le_seuil_hysteresis() {
        assert!(!should_resize_height(360.0, 365.0));
        assert!(!should_resize_height(360.0, 355.0));
    }

    #[test]
    fn should_resize_height_ignore_pile_le_seuil() {
        assert!(!should_resize_height(360.0, 372.0));
    }

    #[test]
    fn should_resize_height_declenche_au_dela_du_seuil() {
        assert!(should_resize_height(360.0, 373.0));
    }

    #[test]
    fn should_resize_height_declenche_symetriquement_en_retrecissement() {
        assert!(should_resize_height(700.0, 400.0));
    }
}
