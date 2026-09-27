//! labels — fichier de configuration des libellés et couleurs (Q9-A de
//! specs/instructions/agnostique-mqtt.md, DA-M8). Reprend ce qui était codé en dur avant ce lot
//! (table de libellés de modèle par famille, roster figé, constante de runner à l'affichage) :
//! plus aucune de ces correspondances n'est figée dans le code, tout vit dans le fichier de
//! configuration.
//!
//! Module NEUTRE : ce fichier ne connaît lui-même aucun runner ni fournisseur — les identifiants
//! réels ne sont que des DONNÉES du fichier par défaut embarqué (`config/labels.default.toml`),
//! jamais du code. Les tests ci-dessous vérifient le MÉCANISME (préfixe le plus long, repli,
//! surcharge) avec des identifiants d'exemple neutres ; la correspondance réelle du fichier
//! embarqué est vérifiée là où le nom d'un runner a le droit d'être écrit (voir `sources/`).

use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// Contenu par défaut, embarqué dans le binaire (DA-M8 : jamais lu depuis le disque au premier
/// niveau — c'est le repli si aucune surcharge utilisateur n'existe ou n'est valide).
const DEFAULT_TOML: &str = include_str!("../config/labels.default.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct ModelLabel {
    pub prefix: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PastilleColor {
    pub bg: String,
    pub fg: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Labels {
    #[serde(default = "default_main_persona")]
    pub main_persona: String,
    #[serde(default = "default_iakaframe_root")]
    pub iakaframe_root: String,
    #[serde(default)]
    pub runners: HashMap<String, String>,
    #[serde(default)]
    pub models: Vec<ModelLabel>,
    #[serde(default)]
    pub pastilles: HashMap<String, PastilleColor>,
    #[serde(default)]
    pub letters: HashMap<String, char>,
}

fn default_main_persona() -> String {
    "odin".to_string()
}

fn default_iakaframe_root() -> String {
    "~/work/iakaframe".to_string()
}

impl Default for Labels {
    fn default() -> Self {
        parse_labels_toml(DEFAULT_TOML).expect("labels.default.toml embarqué invalide")
    }
}

/// Parse le CONTENU d'un fichier de labels (pur, testable). TOML invalide -> `None`.
pub fn parse_labels_toml(content: &str) -> Option<Labels> {
    toml::from_str(content).ok()
}

/// Chemin du fichier de surcharge utilisateur (macOS). `None` si `HOME` est absent.
pub fn user_labels_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library/Application Support/com.iakaframe.iakaagentsmonitor/labels.toml"),
    )
}

/// Charge la configuration : surcharge utilisateur si présente et valide (DA-M8), sinon
/// défaut embarqué. Jamais de plantage : une surcharge invalide retombe sur le défaut.
pub fn load_labels() -> Labels {
    user_labels_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|content| parse_labels_toml(&content))
        .unwrap_or_default()
}

impl Labels {
    /// Libellé d'un runner (identifiant brut si absent de `[runners]`).
    pub fn runner_label(&self, runner: &str) -> String {
        self.runners
            .get(runner)
            .cloned()
            .unwrap_or_else(|| runner.to_string())
    }

    /// Libellé d'un modèle : préfixe le PLUS LONG qui correspond gagne (identifiant brut si
    /// aucun préfixe ne correspond, jamais un libellé inventé).
    pub fn model_label(&self, raw: &str) -> String {
        self.models
            .iter()
            .filter(|m| raw.starts_with(m.prefix.as_str()))
            .max_by_key(|m| m.prefix.len())
            .map(|m| m.label.clone())
            .unwrap_or_else(|| raw.to_string())
    }

    /// Couleurs (fond/texte) d'une pastille par son émoji. `None` si inconnue.
    pub fn pastille_color(&self, pastille: &str) -> Option<&PastilleColor> {
        self.pastilles.get(pastille)
    }

    /// Lettre imposée pour un persona, s'il en a une (sinon l'appelant retombe sur la première
    /// lettre de son nom).
    pub fn forced_letter(&self, persona_id: &str) -> Option<char> {
        self.letters.get(persona_id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_defaut_embarque_est_valide() {
        let labels = Labels::default();
        assert_eq!(labels.main_persona, "odin");
        // Le libellé réel d'un runner concret n'est pas vérifié ICI (module neutre) : voir
        // l'adaptateur fichiers, sous `sources/`, pour la vérification contre le fichier
        // embarqué réel (seul endroit où le nom d'un runner a le droit d'être écrit en dur).
        assert!(!labels.runners.is_empty(), "le fichier par defaut doit porter au moins un runner");
    }

    #[test]
    fn runner_label_repli_sur_identifiant_brut() {
        let labels = Labels::default();
        assert_eq!(labels.runner_label("runner-inconnu-du-fichier"), "runner-inconnu-du-fichier");
    }

    #[test]
    fn model_label_prefixe_le_plus_long_gagne() {
        let toml = r#"
[[models]]
prefix = "acme-x"
label = "X"

[[models]]
prefix = "acme-x-5-5"
label = "X 5.5"
"#;
        let labels = parse_labels_toml(toml).unwrap();
        assert_eq!(labels.model_label("acme-x-5-5-20260115"), "X 5.5");
        assert_eq!(labels.model_label("acme-x-4"), "X");
    }

    #[test]
    fn model_label_repli_sur_identifiant_brut_si_aucun_prefixe() {
        let labels = Labels::default();
        assert_eq!(labels.model_label("modele-inconnu-du-fichier"), "modele-inconnu-du-fichier");
    }

    #[test]
    fn pastille_color_connue_et_inconnue() {
        let labels = Labels::default();
        assert!(labels.pastille_color("🔴").is_some());
        assert!(labels.pastille_color("🟢").is_none());
    }

    #[test]
    fn forced_letter_gimli_et_legolas() {
        let labels = Labels::default();
        assert_eq!(labels.forced_letter("gimli"), Some('I'));
        assert_eq!(labels.forced_letter("legolas"), Some('E'));
        assert_eq!(labels.forced_letter("odin"), None);
    }

    #[test]
    fn toml_invalide_donne_none() {
        assert!(parse_labels_toml("ceci n'est pas du toml valide [[[").is_none());
    }
}
