//! roster — résolution du roster de personas depuis l'ÉQUIPE ACTIVE du projet (Q7-A de
//! specs/instructions/agnostique-mqtt.md) : `<cwd>/.iakaframe` (`frame=`) -> réservoir
//! `frames/<frame>.md` (`teamId:`) -> `teams/<teamId>.md` (`personas:`) -> chaque
//! `library/personas/<id>.md` (`name:`, `pastille:`).
//!
//! Module NEUTRE au sens de la méthode : il ne connaît AUCUN persona en dur (contrairement à
//! l'ancien `ROSTER` figé d'`agents.rs`) — tout vient de la lecture du réservoir. Pointeur
//! absent -> frame `iakaframe` (repli). Réservoir illisible à quelque étape que ce soit -> roster
//! VIDE (jamais de plantage) ; l'appelant (`tree`) traite alors tout persona comme hors équipe
//! (gris + sa lettre).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Identité d'affichage d'un persona, lue dans son fichier `library/personas/<id>.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonaInfo {
    pub name: String,
    /// Émoji de pastille (frontmatter `pastille`), sert de clé dans `labels.toml` `[pastilles]`.
    pub pastille: Option<String>,
}

/// Frame de repli si `<cwd>/.iakaframe` est absent ou illisible.
const DEFAULT_FRAME: &str = "iakaframe";

/// Durée de fraîcheur du cache (Q7-A : « relue au plus toutes les 60 s »).
const CACHE_TTL: Duration = Duration::from_secs(60);

/// Développe un `~` de tête en `$HOME` (le fichier de configuration écrit `iakaframe_root` sous
/// cette forme lisible). Chemin déjà absolu -> inchangé.
pub fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

/// Lit `<cwd>/.iakaframe` -> valeur de `frame=` (ligne exacte, blancs de tête tolérés). Fichier
/// absent/illisible/sans cette ligne -> [`DEFAULT_FRAME`] (jamais `None` : c'est un repli, pas une
/// erreur).
fn read_frame_pointer(cwd: &Path) -> String {
    let path = cwd.join(".iakaframe");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return DEFAULT_FRAME.to_string();
    };
    for line in content.lines() {
        if let Some(v) = line.trim().strip_prefix("frame=") {
            let v = v.trim();
            if !v.is_empty() {
                return v.to_string();
            }
        }
    }
    DEFAULT_FRAME.to_string()
}

/// Extrait la valeur d'une clé `<key>: <valeur>` (frontmatter simple, une ligne). Guillemets
/// simples/doubles ôtés s'ils encadrent toute la valeur.
fn extract_scalar_field(content: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(v) = trimmed.strip_prefix(&prefix) {
            let v = v.trim();
            let v = v
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .or_else(|| v.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                .unwrap_or(v);
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// `<root>/frames/<frame>.md` -> `teamId:`. `None` si absent/illisible/sans ce champ.
fn read_team_id(root: &Path, frame: &str) -> Option<String> {
    let path = root.join("frames").join(format!("{frame}.md"));
    let content = std::fs::read_to_string(path).ok()?;
    extract_scalar_field(&content, "teamId")
}

/// Extrait la liste d'une clé `<key>: [a, b, c]` (une seule ligne, forme YAML inline). Éléments
/// dégagés de leurs guillemets et espaces.
fn extract_list_field(content: &str, key: &str) -> Vec<String> {
    let prefix = format!("{key}:");
    for line in content.lines() {
        let trimmed = line.trim();
        let Some(v) = trimmed.strip_prefix(&prefix) else {
            continue;
        };
        let v = v.trim();
        let Some(inner) = v.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
            continue;
        };
        return inner
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    Vec::new()
}

/// `<root>/teams/<teamId>.md` -> liste `personas:`. Liste vide si absent/illisible/sans ce champ.
fn read_personas_list(root: &Path, team_id: &str) -> Vec<String> {
    let path = root.join("teams").join(format!("{team_id}.md"));
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    extract_list_field(&content, "personas")
}

/// `<root>/library/personas/<id>.md` -> `PersonaInfo` (`name`, `pastille`). `None` si le fichier
/// est absent/illisible ou sans `name`.
fn read_persona_info(root: &Path, persona_id: &str) -> Option<PersonaInfo> {
    let path = root
        .join("library/personas")
        .join(format!("{persona_id}.md"));
    let content = std::fs::read_to_string(path).ok()?;
    let name = extract_scalar_field(&content, "name")?;
    let pastille = extract_scalar_field(&content, "pastille");
    Some(PersonaInfo { name, pastille })
}

/// Résout (sans cache) le roster de l'équipe active du projet dont le répertoire de travail est
/// `cwd`. `cwd` absent (agent sans `cwd` connu) -> réservoir par défaut, frame par défaut.
/// Réservoir illisible à quelque étape -> `HashMap` vide, jamais de panique.
pub fn resolve_uncached(cwd: Option<&Path>, root: &Path) -> HashMap<String, PersonaInfo> {
    let frame = cwd
        .map(read_frame_pointer)
        .unwrap_or_else(|| DEFAULT_FRAME.to_string());
    let Some(team_id) = read_team_id(root, &frame) else {
        return HashMap::new();
    };
    let personas = read_personas_list(root, &team_id);
    personas
        .into_iter()
        .filter_map(|id| read_persona_info(root, &id).map(|info| (id, info)))
        .collect()
}

/// Roster mis en cache par racine de réservoir (Q7-A : au plus une lecture toutes les 60 s).
/// La clé de cache est le `cwd` demandé (deux projets sur deux frames différentes ont chacun leur
/// entrée) — un cache PARTAGÉ suffit pour un usage mono-processus comme cette app.
/// Entrée de cache : instant de dernière lecture + roster résolu à ce moment-là.
type CacheEntry = (Instant, HashMap<String, PersonaInfo>);

#[derive(Default)]
pub struct Roster {
    cache: Mutex<HashMap<String, CacheEntry>>,
}

impl Roster {
    pub fn new() -> Self {
        Self::default()
    }

    /// Résout le roster pour `cwd`, avec la racine `root` (déjà développée, cf. [`expand_home`]).
    /// Rafraîchi au plus toutes les [`CACHE_TTL`] par valeur de `cwd`.
    pub fn resolve(&self, cwd: Option<&str>, root: &Path) -> HashMap<String, PersonaInfo> {
        let key = cwd.unwrap_or("").to_string();
        {
            let cache = self.cache.lock().unwrap();
            if let Some((at, roster)) = cache.get(&key) {
                if at.elapsed() < CACHE_TTL {
                    return roster.clone();
                }
            }
        }
        let roster = resolve_uncached(cwd.map(Path::new), root);
        self.cache
            .lock()
            .unwrap()
            .insert(key, (Instant::now(), roster.clone()));
        roster
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("iaka-am-roster-{name}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Réservoir minimal : une frame, une équipe, deux personas.
    fn make_reservoir(root: &Path) {
        fs::create_dir_all(root.join("frames")).unwrap();
        fs::create_dir_all(root.join("teams")).unwrap();
        fs::create_dir_all(root.join("library/personas")).unwrap();
        fs::write(root.join("frames/iakaframe.md"), "teamId: iakaframe-8\n").unwrap();
        fs::write(
            root.join("teams/iakaframe-8.md"),
            "personas: [odin, gimli, hors-equipe-absent]\n",
        )
        .unwrap();
        fs::write(
            root.join("library/personas/odin.md"),
            "---\nname: Odin\npastille: \"🟡\"\n---\n",
        )
        .unwrap();
        fs::write(
            root.join("library/personas/gimli.md"),
            "---\nname: Gimli\npastille: \"🔴\"\n---\n",
        )
        .unwrap();
    }

    #[test]
    fn resout_le_roster_complet_pointeur_present() {
        let root = tmp_dir("complet");
        make_reservoir(&root);
        let cwd = tmp_dir("projet");
        fs::write(cwd.join(".iakaframe"), "frame=iakaframe\n").unwrap();

        let roster = resolve_uncached(Some(&cwd), &root);
        assert_eq!(roster.len(), 2);
        assert_eq!(roster.get("odin").unwrap().name, "Odin");
        assert_eq!(roster.get("gimli").unwrap().pastille, Some("🔴".to_string()));
        assert!(!roster.contains_key("hors-equipe-absent"), "fichier persona absent -> ignore");
    }

    #[test]
    fn pointeur_absent_replie_sur_la_frame_iakaframe() {
        let root = tmp_dir("repli");
        make_reservoir(&root);
        let cwd = tmp_dir("projet-sans-pointeur");

        let roster = resolve_uncached(Some(&cwd), &root);
        assert_eq!(roster.len(), 2, "meme frame par defaut que le pointeur explicite");
    }

    #[test]
    fn frame_inconnue_donne_un_roster_vide() {
        let root = tmp_dir("frame-inconnue");
        make_reservoir(&root);
        let cwd = tmp_dir("projet-frame-inconnue");
        fs::write(cwd.join(".iakaframe"), "frame=inexistante\n").unwrap();

        let roster = resolve_uncached(Some(&cwd), &root);
        assert!(roster.is_empty());
    }

    #[test]
    fn reservoir_totalement_illisible_donne_un_roster_vide() {
        let root = tmp_dir("absent").join("n-existe-pas-du-tout");
        let roster = resolve_uncached(None, &root);
        assert!(roster.is_empty());
    }

    #[test]
    fn persona_hors_equipe_absent_de_la_map() {
        let root = tmp_dir("hors-equipe");
        make_reservoir(&root);
        let roster = resolve_uncached(None, &root);
        assert!(!roster.contains_key("feanor"));
    }

    #[test]
    fn pastille_absente_du_frontmatter_donne_none() {
        let root = tmp_dir("pastille-absente");
        fs::create_dir_all(root.join("frames")).unwrap();
        fs::create_dir_all(root.join("teams")).unwrap();
        fs::create_dir_all(root.join("library/personas")).unwrap();
        fs::write(root.join("frames/iakaframe.md"), "teamId: t\n").unwrap();
        fs::write(root.join("teams/t.md"), "personas: [solo]\n").unwrap();
        fs::write(root.join("library/personas/solo.md"), "---\nname: Solo\n---\n").unwrap();

        let roster = resolve_uncached(None, &root);
        assert_eq!(roster.get("solo").unwrap().pastille, None);
    }

    #[test]
    fn cache_sert_une_deuxieme_lecture_sans_relire_le_disque() {
        let root = tmp_dir("cache");
        make_reservoir(&root);
        let cwd = tmp_dir("projet-cache");
        fs::write(cwd.join(".iakaframe"), "frame=iakaframe\n").unwrap();

        let roster = Roster::new();
        let first = roster.resolve(Some(cwd.to_str().unwrap()), &root);
        assert_eq!(first.len(), 2);

        // Réservoir cassé APRÈS la première lecture : le cache doit encore servir l'ancien
        // résultat (pas de nouvelle lecture avant CACHE_TTL).
        fs::remove_dir_all(root.join("teams")).unwrap();
        let second = roster.resolve(Some(cwd.to_str().unwrap()), &root);
        assert_eq!(second.len(), 2, "le cache doit encore servir l'ancien resultat");
    }

    #[test]
    fn expand_home_developpe_le_tilde() {
        if let Some(home) = std::env::var_os("HOME") {
            let expanded = expand_home("~/work/iakaframe");
            assert_eq!(expanded, PathBuf::from(home).join("work/iakaframe"));
        }
    }

    #[test]
    fn expand_home_laisse_un_chemin_absolu_inchange() {
        assert_eq!(expand_home("/déjà/absolu"), PathBuf::from("/déjà/absolu"));
    }
}
