//! estimate — cœur NEUTRE de la barre « temps restant » (specs/instructions/temps-restant.md).
//!
//! Ce module ne connaît **aucun runner**, **aucun format de fichier** : il reçoit deux textes
//! bruts (la consigne complète transmise au délégué, et un libellé court optionnel) via
//! [`DelegationText`], et rend une estimation déjà mise en forme. La lecture de ces deux textes
//! (où les trouver, comment les lire, quelle fenêtre lire) est un problème de **source par
//! runner**, réglé ailleurs — ce fichier ne doit jamais avoir besoin d'en savoir plus.
//!
//! Grammaire fermée de la valeur d'estimation, commune aux deux formes (ligne de consigne ou
//! suffixe de libellé) : `~`? N (`-`|`–` M)? unité, `unité` ∈ {s, min, h}, N/M sur 1 à 3 chiffres
//! décimaux, N < M pour une fourchette, borne haute finale entre 10 s et 12 h inclus.

use serde::Serialize;

/// Les deux textes qu'une source par runner peut fournir pour un délégué donné. Le cœur ne fait
/// aucune supposition sur leur origine.
#[derive(Debug, Clone, Default)]
pub struct DelegationText {
    /// Texte complet transmis au délégué au moment de la délégation (peut porter, en tête, une
    /// ligne d'estimation normalisée).
    pub consigne: Option<String>,
    /// Libellé court de la délégation, quand le runner en fournit un (peut porter, en fin de
    /// texte, un suffixe d'estimation entre parenthèses).
    pub short_label: Option<String>,
}

/// Une estimation résolue, en secondes. `lo_secs` absent pour une valeur unique (la borne haute
/// sert alors seule de référence de progression).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Estimate {
    pub lo_secs: Option<i64>,
    pub hi_secs: i64,
}

/// Estimation déjà mise en forme pour l'affichage (aucune décision de présentation ne doit
/// rester à faire côté peinture).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateView {
    /// Remplissage de la barre, 0 à 100, plafonné (un dépassement reste à 100).
    pub fill_pct: u8,
    /// `"ok"` (vert), `"warn"` (orange) ou `"over"` (rouge, dépassement).
    pub level: &'static str,
    /// Texte court de cellule (`~4 min`, `< 1 min`, `+3 min`).
    pub short_label: String,
    /// Phrase longue accessible (`~4 min restantes`, `moins d'1 min restante`,
    /// `+3 min de dépassement`).
    pub long_label: String,
    /// Secondes restantes ; négatif en dépassement.
    pub remaining_secs: i64,
}

// ============================== Grammaire de la valeur ==============================

#[derive(Debug, Clone, Copy, PartialEq)]
enum Unit {
    Sec,
    Min,
    Hour,
}

fn to_secs(n: u32, unit: Unit) -> i64 {
    let n = n as i64;
    match unit {
        Unit::Sec => n,
        Unit::Min => n * 60,
        Unit::Hour => n * 3600,
    }
}

const MIN_TOTAL_SECS: i64 = 10;
const MAX_TOTAL_SECS: i64 = 12 * 3600;

fn skip_ws(s: &str) -> &str {
    s.trim_start_matches(' ')
}

/// 1 à 3 chiffres ASCII en tête de `s`. `None` si absent ou trop long (grammaire : "1 à 3
/// chiffres" — un 4ᵉ chiffre invalide la forme entière, il ne se contente pas de tronquer).
fn take_digits(s: &str) -> Option<(u32, &str)> {
    let end = s
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(s.len());
    if end == 0 || end > 3 {
        return None;
    }
    let n: u32 = s[..end].parse().ok()?;
    Some((n, &s[end..]))
}

/// Séparateur de fourchette : trait d'union ASCII ou tiret demi-cadratin.
fn take_sep(s: &str) -> Option<&str> {
    s.strip_prefix('-').or_else(|| s.strip_prefix('–'))
}

fn take_unit(s: &str) -> Option<(Unit, &str)> {
    if let Some(rest) = s.strip_prefix("min") {
        return Some((Unit::Min, rest));
    }
    if let Some(rest) = s.strip_prefix('s') {
        return Some((Unit::Sec, rest));
    }
    if let Some(rest) = s.strip_prefix('h') {
        return Some((Unit::Hour, rest));
    }
    None
}

/// Analyse une valeur d'estimation (tilde de tête optionnel, valeur unique ou fourchette, unité
/// unique). Rejette tout écart à la grammaire fermée (unité inconnue, texte en trop, bornes hors
/// plage, fourchette non croissante) plutôt que de deviner une intention.
pub fn parse_value(input: &str) -> Option<Estimate> {
    let s = input.trim();
    let s = s.strip_prefix('~').unwrap_or(s);
    let s = skip_ws(s);
    let (n1, s) = take_digits(s)?;
    let s = skip_ws(s);

    if let Some(after_sep) = take_sep(s) {
        let after_sep = skip_ws(after_sep);
        let (n2, s2) = take_digits(after_sep)?;
        let s2 = skip_ws(s2);
        let (unit, s2) = take_unit(s2)?;
        if !s2.is_empty() {
            return None;
        }
        if n1 >= n2 {
            return None;
        }
        let lo_secs = to_secs(n1, unit);
        let hi_secs = to_secs(n2, unit);
        if !(MIN_TOTAL_SECS..=MAX_TOTAL_SECS).contains(&hi_secs) {
            return None;
        }
        return Some(Estimate {
            lo_secs: Some(lo_secs),
            hi_secs,
        });
    }

    let (unit, s) = take_unit(s)?;
    if !s.is_empty() {
        return None;
    }
    let hi_secs = to_secs(n1, unit);
    if !(MIN_TOTAL_SECS..=MAX_TOTAL_SECS).contains(&hi_secs) {
        return None;
    }
    Some(Estimate {
        lo_secs: None,
        hi_secs,
    })
}

// ============================== Forme 1 : ligne de consigne ==============================

/// Nombre de lignes non vides examinées en tête de la consigne (tolérance à un titre ou un badge
/// placé avant la ligne d'estimation).
const CONSIGNE_SCAN_LINES: usize = 5;

/// Retire, caractère par caractère et **insensible à la casse et à l'accentuation** (les deux
/// graphies acceptées ne diffèrent que par le é/e), le préfixe d'étiquette d'un texte. Travaille
/// directement sur la chaîne d'origine (jamais sur une version reconstruite) pour ne jamais
/// désynchroniser les décalages d'octets d'un caractère accentué multi-octets.
fn strip_label_prefix(s: &str) -> Option<&str> {
    const LABEL: &str = "duree estimee";
    let mut chars = s.char_indices();
    for expected in LABEL.chars() {
        let (_, c) = chars.next()?;
        let normalized = match c.to_ascii_lowercase() {
            'é' => 'e',
            other => other.to_lowercase().next().unwrap_or(other),
        };
        // `é` est multi-octet : `to_ascii_lowercase` le laisse tel quel (il n'est pas ASCII), le
        // premier bras du match ci-dessus le traite donc explicitement.
        let normalized = if c == 'É' { 'e' } else { normalized };
        if normalized != expected {
            return None;
        }
    }
    match chars.next() {
        Some((idx, _)) => Some(&s[idx..]),
        None => Some(""),
    }
}

/// Une ligne isolée respecte-t-elle la forme 1 (étiquette, `:`, valeur, fin de ligne) ?
pub fn parse_consigne_line(line: &str) -> Option<Estimate> {
    let trimmed = line.trim_start();
    let rest = strip_label_prefix(trimmed)?;
    let rest = skip_ws(rest);
    let rest = rest.strip_prefix(':')?;
    let rest = rest.trim(); // blancs de part et d'autre, y compris finaux.
    parse_value(rest)
}

/// Cherche la ligne d'estimation parmi les [`CONSIGNE_SCAN_LINES`] premières lignes NON VIDES de
/// la consigne complète.
fn find_consigne_estimate(consigne: &str) -> Option<Estimate> {
    consigne
        .lines()
        .filter(|l| !l.trim().is_empty())
        .take(CONSIGNE_SCAN_LINES)
        .find_map(parse_consigne_line)
}

// ============================== Forme 2 : suffixe de libellé ==============================

/// Fenêtre examinée en fin de libellé (en caractères, pas en octets — un libellé accentué ne
/// doit jamais fausser la mesure).
const SUFFIX_WINDOW_CHARS: usize = 40;

/// Sépare un libellé court de son éventuel suffixe d'estimation `" (~valeur)"`. Le tilde est
/// **obligatoire** dans le suffixe (il écarte les parenthèses déjà en usage dans les libellés
/// réels). Un suffixe invalide laisse le libellé **intact** ; un suffixe valide est retiré, avec
/// le blanc séparateur qui le précède s'il y en a un.
pub fn split_label_suffix(label: &str) -> (String, Option<Estimate>) {
    let trimmed = label.trim_end();
    let chars: Vec<char> = trimmed.chars().collect();
    let window_start = chars.len().saturating_sub(SUFFIX_WINDOW_CHARS);
    let window: String = chars[window_start..].iter().collect();

    let Some(without_close) = window.strip_suffix(')') else {
        return (trimmed.to_string(), None);
    };
    let Some(open_byte_pos) = without_close.rfind('(') else {
        return (trimmed.to_string(), None);
    };
    let open_char_count = without_close[..open_byte_pos].chars().count();
    let inner: String = without_close.chars().skip(open_char_count + 1).collect();
    let Some(value_str) = inner.trim().strip_prefix('~') else {
        return (trimmed.to_string(), None);
    };
    let Some(estimate) = parse_value(value_str) else {
        return (trimmed.to_string(), None);
    };

    let open_abs = window_start + open_char_count;
    let mut end = open_abs;
    if end > 0 && chars[end - 1] == ' ' {
        end -= 1;
    }
    let remaining: String = chars[..end].iter().collect();
    (remaining, Some(estimate))
}

// ==================== Extraction BRUTE (pour une source, avant validation) ====================
//
// Une source (publieur ou adaptateur) doit transmettre l'estimation TELLE QU'ANNONCÉE, sans la
// valider elle-même (Q5-A de agnostique-mqtt.md : « une seule grammaire, côté app »). Les deux
// fonctions suivantes renvoient donc le texte BRUT de la valeur repérée (déjà confirmé conforme à
// la grammaire, pour ne jamais transmettre un texte qui ne se réanalyserait pas), et non une
// structure déjà analysée.

/// Cherche la ligne d'estimation (mêmes règles que [`find_consigne_estimate`]) et renvoie le
/// texte BRUT de la valeur (`"~10 min"`, jamais la structure). `None` si rien de conforme dans
/// la fenêtre de lignes examinée.
pub fn find_consigne_estimate_raw(consigne: &str) -> Option<String> {
    consigne
        .lines()
        .filter(|l| !l.trim().is_empty())
        .take(CONSIGNE_SCAN_LINES)
        .find_map(|line| {
            let trimmed = line.trim_start();
            let rest = strip_label_prefix(trimmed)?;
            let rest = skip_ws(rest);
            let rest = rest.strip_prefix(':')?;
            let value = rest.trim();
            parse_value(value)?;
            Some(value.to_string())
        })
}

/// Extrait le texte BRUT du suffixe d'estimation d'un libellé (`"X (~10 min)"` -> `Some("~10
/// min")`), SANS le retirer du libellé (contrairement à [`split_label_suffix`]) — une source n'a
/// besoin que de la valeur annoncée. `None` si le suffixe est absent ou invalide.
pub fn suffix_estimate_raw(label: &str) -> Option<String> {
    let trimmed = label.trim_end();
    let chars: Vec<char> = trimmed.chars().collect();
    let window_start = chars.len().saturating_sub(SUFFIX_WINDOW_CHARS);
    let window: String = chars[window_start..].iter().collect();
    let without_close = window.strip_suffix(')')?;
    let open_byte_pos = without_close.rfind('(')?;
    let inner = without_close[open_byte_pos + 1..].trim();
    let value_str = inner.strip_prefix('~')?;
    parse_value(value_str)?;
    Some(inner.to_string())
}

// ============================== Résolution (priorité) ==============================

/// Combine les deux textes d'une délégation : la ligne de consigne l'emporte sur le suffixe du
/// libellé quand les deux sont valides ; le libellé est **toujours** rendu débarrassé de son
/// suffixe (quand il est valide), quelle que soit l'estimation retenue.
pub fn resolve(text: &DelegationText) -> (Option<String>, Option<Estimate>) {
    let consigne_estimate = text.consigne.as_deref().and_then(find_consigne_estimate);
    let (task, suffix_estimate) = match &text.short_label {
        Some(label) => {
            let (stripped, est) = split_label_suffix(label);
            (Some(stripped), est)
        }
        None => (None, None),
    };
    (task, consigne_estimate.or(suffix_estimate))
}

// ============================== Mise en forme (calcul + peinture) ==============================

fn ceil_minutes(secs: i64) -> i64 {
    (secs.max(0) + 59) / 60
}

fn format_minutes_hm(total_min: i64) -> String {
    if total_min < 60 {
        return format!("{total_min} min");
    }
    let hours = total_min / 60;
    let rem_min = total_min % 60;
    if rem_min == 0 {
        format!("{hours} h")
    } else {
        format!("{hours} h {rem_min} min")
    }
}

/// Même style que « démarré il y a », sans arrondi (ce sont des bornes exactes, pas un compte à
/// rebours) : secondes sous la minute, minutes sous l'heure, heures + minutes au-delà.
fn format_duration_secs(secs: i64) -> String {
    let secs = secs.max(0);
    if secs < 60 {
        return format!("{secs} s");
    }
    let minutes = secs / 60;
    if minutes < 60 {
        return format!("{minutes} min");
    }
    let hours = minutes / 60;
    let rem_min = minutes % 60;
    if rem_min == 0 {
        format!("{hours} h")
    } else {
        format!("{hours} h {rem_min} min")
    }
}

/// Texte lisible de l'estimation ANNONCÉE (pas du reste), pour l'infobulle (`estimé ~10 min`).
pub fn format_estimate_announced(estimate: &Estimate) -> String {
    match estimate.lo_secs {
        Some(lo) => format!(
            "~{}-{}",
            format_duration_secs(lo),
            format_duration_secs(estimate.hi_secs)
        ),
        None => format!("~{}", format_duration_secs(estimate.hi_secs)),
    }
}

fn format_short_label(remaining_secs: i64) -> String {
    if remaining_secs < 0 {
        let over_min = ceil_minutes(-remaining_secs).max(1);
        format!("+{over_min} min")
    } else if remaining_secs < 60 {
        "< 1 min".to_string()
    } else {
        format!("~{}", format_minutes_hm(ceil_minutes(remaining_secs)))
    }
}

fn format_long_label(remaining_secs: i64, short_label: &str) -> String {
    if remaining_secs < 0 {
        format!("{short_label} de dépassement")
    } else if remaining_secs < 60 {
        "moins d'1 min restante".to_string()
    } else {
        format!("{short_label} restantes")
    }
}

/// Calcule la vue d'affichage d'une estimation à un instant donné (écoulé en secondes depuis le
/// début). Trois paliers nets (DA-T3) : vert sous le seuil, orange jusqu'à la borne haute
/// incluse, rouge strictement au-delà.
pub fn estimate_view(estimate: &Estimate, elapsed_secs: i64) -> EstimateView {
    let elapsed = elapsed_secs.max(0);
    let hi = estimate.hi_secs;
    let warn_threshold = estimate.lo_secs.unwrap_or((hi * 3) / 4);

    let level: &'static str = if elapsed > hi {
        "over"
    } else if elapsed >= warn_threshold {
        "warn"
    } else {
        "ok"
    };

    let fill_pct = if hi <= 0 {
        100
    } else {
        ((elapsed.min(hi) * 100) / hi) as u8
    };

    let remaining_secs = hi - elapsed;
    let short_label = format_short_label(remaining_secs);
    let long_label = format_long_label(remaining_secs, &short_label);

    EstimateView {
        fill_pct,
        level,
        short_label,
        long_label,
        remaining_secs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn est(hi: i64) -> Estimate {
        Estimate {
            lo_secs: None,
            hi_secs: hi,
        }
    }
    fn range(lo: i64, hi: i64) -> Estimate {
        Estimate {
            lo_secs: Some(lo),
            hi_secs: hi,
        }
    }

    // ---------------------------- Grammaire de la valeur ----------------------------

    #[test]
    fn valeurs_acceptees() {
        assert_eq!(parse_value("~10 min"), Some(est(600)));
        assert_eq!(parse_value("10 min"), Some(est(600)));
        assert_eq!(parse_value("~45 s"), Some(est(45)));
        assert_eq!(parse_value("~2 h"), Some(est(7200)));
        assert_eq!(parse_value("~10-15 min"), Some(range(600, 900)));
        assert_eq!(parse_value("~10–15 min"), Some(range(600, 900)));
        assert_eq!(parse_value("~ 10 - 15 min"), Some(range(600, 900)));
        assert_eq!(parse_value("~10min"), Some(est(600)));
    }

    #[test]
    fn valeurs_refusees() {
        assert_eq!(parse_value("~1.5 h"), None);
        assert_eq!(parse_value("~0 min"), None);
        assert_eq!(parse_value("~5 s"), None, "sous les 10 s");
        assert_eq!(parse_value("~13 h"), None, "au-dela de 12 h");
        assert_eq!(parse_value("~1000 min"), None, "plus de 3 chiffres");
        assert_eq!(parse_value("~15-10 min"), None, "N doit etre < M");
        assert_eq!(parse_value("~10 mins"), None, "unite invalide");
        assert_eq!(parse_value("~10 Min"), None, "casse invalide");
        assert_eq!(parse_value("~45 min-1 h"), None, "une seule unite pour la fourchette");
        assert_eq!(parse_value(""), None);
    }

    // ---------------------------- Ligne de consigne ----------------------------

    #[test]
    fn ligne_en_premiere_position() {
        assert_eq!(parse_consigne_line("Durée estimée : ~10 min"), Some(est(600)));
    }

    #[test]
    fn ligne_apres_titre_et_ligne_vide() {
        let consigne = "Titre\n\nDurée estimée : ~10 min\nsuite";
        assert_eq!(find_consigne_estimate(consigne), Some(est(600)));
    }

    #[test]
    fn ligne_casse_et_accent_indifferents() {
        assert_eq!(parse_consigne_line("DURÉE ESTIMÉE : ~10 min"), Some(est(600)));
        assert_eq!(parse_consigne_line("Duree estimee: 10 min"), Some(est(600)));
    }

    #[test]
    fn ligne_refusee_en_6e_ligne_non_vide() {
        let consigne = "1\n2\n3\n4\n5\nDurée estimée : ~10 min";
        assert_eq!(find_consigne_estimate(consigne), None);
    }

    #[test]
    fn ligne_refusee_si_texte_apres_la_valeur() {
        assert_eq!(parse_consigne_line("Durée estimée : ~10 min environ"), None);
    }

    #[test]
    fn ligne_refusee_au_milieu_d_une_phrase() {
        assert_eq!(parse_consigne_line("Pense a la Durée estimée : ~10 min"), None);
    }

    #[test]
    fn consigne_absente_ne_donne_rien() {
        assert_eq!(find_consigne_estimate(""), None);
    }

    // ---------------------------- Suffixe de libellé ----------------------------

    #[test]
    fn suffixe_accepte_retire_du_libelle() {
        assert_eq!(split_label_suffix("X (~10 min)"), ("X".to_string(), Some(est(600))));
        assert_eq!(
            split_label_suffix("( ~ 10 min )"),
            (String::new(), Some(est(600))),
            "libelle reduit au seul suffixe -> tache vide"
        );
    }

    #[test]
    fn suffixe_refuse_laisse_le_libelle_inchange() {
        let cases = [
            "Legolas passe 2 sur 3 (relance)",
            "Gate qualité lot 6 (6·1, 6·3)",
            "Rédaction Partie II (Lot 4)",
            "X (10 min)",
            "X (~10 min) suite",
            "Cadrage lot 1c (ratification clavier + instruction socle)",
        ];
        for label in cases {
            let (stripped, estimate) = split_label_suffix(label);
            assert_eq!(stripped, label, "libelle modifie a tort pour {label:?}");
            assert_eq!(estimate, None, "estimation extraite a tort de {label:?}");
        }
    }

    // ---------------------------- Priorité ----------------------------

    #[test]
    fn priorite_ligne_seule() {
        let t = DelegationText {
            consigne: Some("Durée estimée : ~10 min\nreste".to_string()),
            short_label: Some("Tache".to_string()),
        };
        assert_eq!(resolve(&t), (Some("Tache".to_string()), Some(est(600))));
    }

    #[test]
    fn priorite_suffixe_seul() {
        let t = DelegationText {
            consigne: Some("juste du texte".to_string()),
            short_label: Some("Tache (~10 min)".to_string()),
        };
        assert_eq!(resolve(&t), (Some("Tache".to_string()), Some(est(600))));
    }

    #[test]
    fn priorite_les_deux_egaux() {
        let t = DelegationText {
            consigne: Some("Durée estimée : ~10 min".to_string()),
            short_label: Some("Tache (~10 min)".to_string()),
        };
        assert_eq!(resolve(&t), (Some("Tache".to_string()), Some(est(600))));
    }

    #[test]
    fn priorite_les_deux_differents_ligne_gagne_suffixe_retire() {
        let t = DelegationText {
            consigne: Some("Durée estimée : ~20 min".to_string()),
            short_label: Some("Tache (~10 min)".to_string()),
        };
        assert_eq!(resolve(&t), (Some("Tache".to_string()), Some(est(1200))));
    }

    #[test]
    fn priorite_ligne_invalide_suffixe_valide() {
        let t = DelegationText {
            consigne: Some("Durée estimée : ~1000 min".to_string()),
            short_label: Some("Tache (~10 min)".to_string()),
        };
        assert_eq!(resolve(&t), (Some("Tache".to_string()), Some(est(600))));
    }

    #[test]
    fn priorite_rien_ne_donne_rien() {
        let t = DelegationText::default();
        assert_eq!(resolve(&t), (None, None));
    }

    // ---------------------------- Paliers ----------------------------

    #[test]
    fn paliers_valeur_unique() {
        assert_eq!(estimate_view(&est(400), 0).level, "ok");
        assert_eq!(estimate_view(&est(400), 296).level, "ok"); // 74 %
        assert_eq!(estimate_view(&est(400), 300).level, "warn"); // 75 %
        assert_eq!(estimate_view(&est(400), 400).level, "warn"); // 100 %
        assert_eq!(estimate_view(&est(400), 404).level, "over"); // 101 %
    }

    #[test]
    fn paliers_fourchette_10_15_min() {
        let e = range(600, 900);
        assert_eq!(estimate_view(&e, 540).level, "ok"); // 9 min
        assert_eq!(estimate_view(&e, 600).level, "warn"); // 10 min
        assert_eq!(estimate_view(&e, 960).level, "over"); // 16 min
    }

    // ---------------------------- Textes ----------------------------

    #[test]
    fn textes_restants() {
        assert_eq!(estimate_view(&est(1_000_000), 1_000_000 - 239).short_label, "~4 min");
        assert_eq!(estimate_view(&est(1_000_000), 1_000_000 - 30).short_label, "< 1 min");
        assert_eq!(estimate_view(&est(1_000_000), 1_000_000 - 3_900).short_label, "~1 h 5 min");
        assert_eq!(estimate_view(&est(1_000_000), 1_000_000 + 170).short_label, "+3 min");
    }

    #[test]
    fn textes_longs() {
        let e = est(1_000_000);
        assert_eq!(estimate_view(&e, 1_000_000 - 239).long_label, "~4 min restantes");
        assert_eq!(estimate_view(&e, 1_000_000 - 30).long_label, "moins d'1 min restante");
        assert_eq!(estimate_view(&e, 1_000_000 + 170).long_label, "+3 min de dépassement");
    }

    #[test]
    fn plus_une_min_des_la_premiere_seconde_de_depassement() {
        assert_eq!(estimate_view(&est(600), 601).short_label, "+1 min");
    }

    #[test]
    fn fill_pct_plafonne_a_100() {
        assert_eq!(estimate_view(&est(600), 900).fill_pct, 100);
        assert_eq!(estimate_view(&est(600), 300).fill_pct, 50);
    }

    // ---------------------------- Extraction brute (source) ----------------------------

    #[test]
    fn find_consigne_estimate_raw_renvoie_le_texte_brut() {
        assert_eq!(
            find_consigne_estimate_raw("Durée estimée : ~10 min\nreste"),
            Some("~10 min".to_string())
        );
        assert_eq!(find_consigne_estimate_raw("rien ici"), None);
        assert_eq!(find_consigne_estimate_raw("Durée estimée : ~1000 min"), None);
    }

    #[test]
    fn suffix_estimate_raw_renvoie_le_texte_brut_sans_retirer_le_libelle() {
        assert_eq!(suffix_estimate_raw("Tache (~10 min)"), Some("~10 min".to_string()));
        assert_eq!(suffix_estimate_raw("Legolas passe 2 sur 3 (relance)"), None);
        assert_eq!(suffix_estimate_raw("X (10 min)"), None, "tilde obligatoire");
    }

    #[test]
    fn format_estimate_announced_valeur_unique_et_fourchette() {
        assert_eq!(format_estimate_announced(&est(600)), "~10 min");
        assert_eq!(format_estimate_announced(&range(600, 900)), "~10 min-15 min");
    }
}
