//! agents — agents Claude Code « en cours » (specs/instructions/liste-des-agents.md) : roster de
//! sprites, lecture des sidecars `agent-<id>.meta.json`, liveness par mtime, construction de
//! l'arbre de délégation (`parentAgentId`, exact à toute profondeur), instantané + commande +
//! watcher.
//!
//! ┌────────────────────────────────────────────────────────────────────────────────────────┐
//! │ PROVENANCE : recopié depuis iakaTokenCounter, `src-tauri/src/agents.rs`, branche         │
//! │ `worktree-agent-a73ae73650ed5e328`, commit `1c9fd0e` (Q1-A/Q3-A de l'instruction).       │
//! │ Adaptations : (a) import depuis `crate::claude_paths` au lieu d'`iakatc_core` ; (b) plus  │
//! │ de relecture du magasin de réservoirs — `run_watcher` met juste à jour le titre du tray   │
//! │ avec l'effectif ; (c) horloge/seuil lus localement (`IAKAAM_LIVENESS_SECS`, DA-1) ;       │
//! │ (d) modèle enrichi de champs structurés (persona, tâche, libellés d'activité déjà         │
//! │ formatés) pour le tableau indenté par délégation (amendement A1 du 26/09) — la popover    │
//! │ V1 (bracket imbriqué) n'est plus le mode d'affichage retenu, mais la structure d'arbre     │
//! │ reste identique, sprite/roster/liveness/tests inchangés.                                  │
//! └────────────────────────────────────────────────────────────────────────────────────────┘

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

use crate::claude_paths::{claude_projects_dir, claude_transcript_files, project_of};
use crate::estimate;
use crate::state::AppState;

/// Nom d'événement poussé à la webview à chaque changement d'instantané agents.
pub const AGENTS_EVENT: &str = "tray://agents";

/// Cadence du watcher dédié.
const TICK: Duration = Duration::from_secs(5);

/// Bornes d'affichage : au-delà, un marqueur « +N ».
const MAX_SESSIONS: usize = 6;
const MAX_CHILDREN: usize = 8;

/// Plafond de lecture en tête d'un transcript (uniquement pour les sessions vivantes) : sert à
/// la fois à chercher le `cwd` (label de projet) et le `message.model` (colonne Modèle), jamais
/// à lire le fichier entier. Les transcripts réels commencent souvent par plusieurs lignes de
/// service (`mode`, `permission-mode`, `atis-latch`, `file-history-snapshot`…) avant la première
/// ligne utile — d'où un plafond en OCTETS plutôt qu'un nombre de lignes fixe (passe corrective
/// post-gate Legolas du 26/09 : `project_label_for_session` ne lisait que la 1re ligne physique).
const TRANSCRIPT_HEAD_CAP: usize = 1024 * 1024; // 1 MiB

/// Variable d'environnement de seuil de liveness (DA-1). Défaut : [`DEFAULT_LIVENESS_SECS`].
const LIVENESS_ENV: &str = "IAKAAM_LIVENESS_SECS";
const DEFAULT_LIVENESS_SECS: u64 = 90;

/// Seuil de liveness (s) des agents en cours, lu depuis `IAKAAM_LIVENESS_SECS`. Valeur
/// absente/invalide -> [`DEFAULT_LIVENESS_SECS`].
pub fn liveness_secs_from_env() -> u64 {
    std::env::var(LIVENESS_ENV)
        .ok()
        .filter(|s| !s.is_empty())
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_LIVENESS_SECS)
}

// ============================== Roster et palette ==============================

/// Sprite résolu d'un agent : lettre + couleurs (fond/texte), déjà prêtes à peindre. Rendu
/// DOM/CSS côté webview (carré arrondi) — point d'extension unique pour un futur remplacement par
/// une icône pixel art 4×4 (amendement A1, choix de Loki/Stéphane, non implémenté ici).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sprite {
    pub letter: char,
    pub bg: &'static str,
    pub fg: &'static str,
}

/// Table figée `agentType -> (lettre, fond, texte)`. Codée en dur : l'app reste autonome, aucune
/// lecture de `~/work/iakaframe` à l'exécution. Source : frontmatter `pastille` de
/// `~/work/iakaframe/library/personas/*.md`.
///
/// Passe corrective — amendement A3 (contraste, 26/09) :
/// - **Texte des lettres en `#0B0D12`** (quasi-noir) pour gandalf/gimli/legolas/helm : le blanc
///   sur leurs fonds (bleu/rouge/violet) ne passait PAS le AA normal-text (4,5:1) — mesuré
///   3,52–3,65:1. Sombre sur ces quatre fonds donne 5,3–5,5:1, largement au-dessus du seuil.
/// - **Lettre de Gimli changée de `G` à `I`** (2ᵉ lettre du nom) : elle collisionnait visuellement
///   avec le `G` de Gandalf (fonds différents, mais même lettre — repéré par Stéphane). `I` reste
///   mnémotechnique (toujours tirée du nom) et ne collisionne avec aucune autre lettre du roster.
///
/// Amendement A4 (point 2, 26/09) : **lettre de Legolas changée de `L` à `E`** (2ᵉ lettre du nom,
/// même logique que Gimli/`I`) — elle collisionnait visuellement avec Loki, qui garde `L`.
const ROSTER: &[(&str, char, &str, &str)] = &[
    ("odin", 'O', "#FFD60A", "#0B0D12"),
    ("aragorn", 'A', "#FF9F0A", "#0B0D12"),
    ("gandalf", 'G', "#0A84FF", "#0B0D12"),
    ("gimli", 'I', "#FF3B30", "#0B0D12"),
    ("legolas", 'E', "#FF3B30", "#0B0D12"),
    ("helm", 'H', "#BF5AF2", "#0B0D12"),
    ("loki", 'L', "#FF9F0A", "#0B0D12"),
    ("nathalie", 'N', "#FF9F0A", "#0B0D12"),
    ("feanor", 'F', "#FF9F0A", "#0B0D12"),
];

/// Nom d'affichage d'un persona, avec majuscule (amendement A3, point 7). Table explicite plutôt
/// qu'un simple `titlecase` : porte les accents (« Fëanor ») que `titlecase` ne saurait pas
/// ajouter. Type hors roster -> `titlecase` du type brut (repli honnête, jamais un nom inventé).
fn display_persona(agent_type: &str) -> String {
    match agent_type.to_ascii_lowercase().as_str() {
        "odin" => "Odin".to_string(),
        "aragorn" => "Aragorn".to_string(),
        "gandalf" => "Gandalf".to_string(),
        "gimli" => "Gimli".to_string(),
        "legolas" => "Legolas".to_string(),
        "helm" => "Helm".to_string(),
        "loki" => "Loki".to_string(),
        "nathalie" => "Nathalie".to_string(),
        "feanor" => "Fëanor".to_string(),
        "" => "Inconnu".to_string(),
        other => titlecase(other),
    }
}

/// Fond/texte du repli « hors roster » (gris neutre) : jamais déguisé en persona de la frame.
const FALLBACK_BG: &str = "#6F6F78";
const FALLBACK_FG: &str = "#FFFFFF";

/// Résout le sprite d'un `agentType` (insensible à la casse). Type du roster -> table ; type
/// hors roster -> première lettre du type (majuscule) sur fond gris ; type vide (sidecar
/// absent/invalide) -> `?` sur fond gris. Jamais de panique, jamais de faux-persona.
pub fn sprite_for(agent_type: &str) -> Sprite {
    let lower = agent_type.to_ascii_lowercase();
    if let Some(&(_, letter, bg, fg)) = ROSTER.iter().find(|(t, ..)| *t == lower) {
        return Sprite { letter, bg, fg };
    }
    let letter = agent_type
        .chars()
        .next()
        .map(|c| c.to_ascii_uppercase())
        .unwrap_or('?');
    Sprite {
        letter,
        bg: FALLBACK_BG,
        fg: FALLBACK_FG,
    }
}

/// Sprite conventionnel du coordinateur de session : « un coordinateur *est* un agent — Odin,
/// cf. CLAUDE.md global ». Le coordinateur ne porte aucun sidecar de persona ; la méthode le
/// désigne par convention comme Odin.
pub fn coordinator_sprite() -> Sprite {
    sprite_for("odin")
}

// ========================= Lecture du sidecar meta.json =========================

/// Identité + parenté d'un sous-agent, lue depuis son sidecar `agent-<id>.meta.json`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentMeta {
    /// `agentType` (nom du persona). Vide si absent/illisible/invalide -> type inconnu.
    pub agent_type: String,
    pub description: Option<String>,
    /// Absent -> le parent est le coordinateur de la session.
    pub parent_agent_id: Option<String>,
    pub spawn_depth: u32,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawAgentMeta {
    #[serde(default)]
    agent_type: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    parent_agent_id: Option<String>,
    #[serde(default)]
    spawn_depth: u32,
}

impl From<RawAgentMeta> for AgentMeta {
    fn from(r: RawAgentMeta) -> Self {
        AgentMeta {
            agent_type: r.agent_type,
            description: r.description,
            parent_agent_id: r.parent_agent_id,
            spawn_depth: r.spawn_depth,
        }
    }
}

/// Parse le CONTENU d'un sidecar (pur, testable). JSON invalide -> `None`. Un objet JSON valide
/// mais partiel (champs manquants) produit un `AgentMeta` avec les valeurs par défaut.
pub fn parse_agent_meta(content: &str) -> Option<AgentMeta> {
    serde_json::from_str::<RawAgentMeta>(content)
        .ok()
        .map(AgentMeta::from)
}

/// Lit et parse le sidecar sur disque. Fichier absent/illisible/JSON invalide -> `None`.
fn read_agent_meta(path: &Path) -> Option<AgentMeta> {
    let content = std::fs::read_to_string(path).ok()?;
    parse_agent_meta(&content)
}

// ============================ Liveness par mtime ============================

/// Un agent est vivant ssi `now - mtime <= n`. Défensif face à une horloge en avance (mtime
/// futur, diff négative) : traité comme vivant.
pub fn is_live(now: i64, mtime: i64, n: i64) -> bool {
    now - mtime <= n
}

fn epoch_of(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn mtime_of(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .map(epoch_of)
}

fn created_of(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .and_then(|m| m.created())
        .ok()
        .map(epoch_of)
}

// ===================== Formatage des durées (fait en Rust, D9) =====================

/// Cœur commun de formatage d'une durée en secondes : secondes sous 60 s, minutes au-delà,
/// `h min` au-delà de 60 min (`h` sec si le reste est nul). Partagé par « Actif il y a » et
/// « Démarré il y a » (amendement A5, point 1 : même format au-delà de 60 s, ex. « 6 min », «
/// 1 h 5 min » — jamais « 371 s »).
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

/// « Actif il y a » : secondes sous 60 s, puis même format que « Démarré il y a » au-delà
/// (amendement A5, point 1 — constat réel : « 371 s » affiché en compact, corrigé en « 6 min »).
pub fn format_active_ago(secs: i64) -> String {
    format_duration_secs(secs)
}

/// « Démarré il y a » : secondes sous 60 s, minutes au-delà, `h min` au-delà de 60 min. `None`
/// (date de création indisponible) -> chaîne vide (mention omise côté appelant).
pub fn format_started_ago(secs: Option<i64>) -> String {
    let Some(secs) = secs else {
        return String::new();
    };
    format_duration_secs(secs)
}

// ===================== Lecture bornée en tête de transcript =====================

/// Lit au plus `cap` octets en tête d'un fichier (jamais le fichier entier). `None` si le
/// fichier est illisible ou vide.
fn read_head_capped(path: &Path, cap: usize) -> Option<String> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; cap];
    let n = f.read(&mut buf).ok()?;
    if n == 0 {
        return None;
    }
    buf.truncate(n);
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// Cherche `cwd` (chaîne) dans les lignes de tête d'un transcript — PAS seulement la première
/// ligne physique : un transcript réel commence souvent par des lignes de service sans `cwd`
/// avant la première ligne `user` qui le porte. Bornée en octets (`cap`), jamais tout le fichier.
/// Ligne finale tronquée par la borne -> JSON invalide -> ignorée (défensif).
fn first_cwd_in_transcript(path: &Path, cap: usize) -> Option<String> {
    let head = read_head_capped(path, cap)?;
    for line in head.lines() {
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(cwd) = v.get("cwd").and_then(|c| c.as_str()) {
            return Some(cwd.to_string());
        }
    }
    None
}

/// Cherche `message.model` dans le premier enregistrement `"type":"assistant"` des lignes d'une
/// tête de transcript DÉJÀ LUE. Pure (aucune E/S) : c'est la fonction bornée
/// [`first_assistant_model_in_transcript`] qui lit le disque, et la lecture combinée de
/// `resolve_subagent` (DA-T7 de temps-restant.md) qui réutilise celle-ci sans relire.
fn model_from_head(head: &str) -> Option<String> {
    for line in head.lines() {
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }
        if let Some(model) = v
            .get("message")
            .and_then(|m| m.get("model"))
            .and_then(|m| m.as_str())
        {
            return Some(model.to_string());
        }
    }
    None
}

/// Cherche `message.model` dans le premier enregistrement `"type":"assistant"` des lignes de
/// tête d'un transcript (bornée en octets). `None` si absent/illisible dans la fenêtre lue.
fn first_assistant_model_in_transcript(path: &Path, cap: usize) -> Option<String> {
    read_head_capped(path, cap).and_then(|head| model_from_head(&head))
}

fn titlecase(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// Réduit un identifiant de modèle brut (ex. `claude-opus-5-5-20260115`) à un libellé court
/// lisible (« Opus 5.5 »). Repli : l'identifiant brut tel quel si la famille (opus/sonnet/haiku)
/// n'est pas reconnue — jamais un libellé inventé. Le repli « — » (aucun modèle trouvé du tout)
/// est géré par l'appelant, cf. [`model_label_for_transcript`].
pub fn format_model_label(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    let parts: Vec<&str> = lower.split('-').collect();
    let Some(family_idx) = parts.iter().position(|p| matches!(*p, "opus" | "sonnet" | "haiku"))
    else {
        return raw.to_string();
    };
    let family = titlecase(parts[family_idx]);
    let mut version_parts: Vec<&str> = Vec::new();
    for p in &parts[family_idx + 1..] {
        if p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()) {
            break;
        }
        // Un identifiant de date (8 chiffres, ex. 20260115) n'est pas un numéro de version.
        if p.len() == 8 {
            break;
        }
        version_parts.push(p);
    }
    if version_parts.is_empty() {
        family
    } else {
        format!("{family} {}", version_parts.join("."))
    }
}

/// Libellé de modèle d'UN transcript (coordinateur ou sous-agent, toujours le SIEN, jamais celui
/// d'un autre agent). `"—"` si aucun modèle n'a été trouvé dans la fenêtre lue (défensif, jamais
/// inventé).
fn model_label_for_transcript(path: &Path) -> String {
    first_assistant_model_in_transcript(path, TRANSCRIPT_HEAD_CAP)
        .map(|m| format_model_label(&m))
        .unwrap_or_else(|| "—".to_string())
}

// ===================== Tâche de la session principale (amendement A3, point 11) =====================

/// Nombre maximal de caractères conservés dans le libellé de tâche (point 11), avant troncature
/// avec « … ».
const TASK_SNIPPET_MAX_CHARS: usize = 60;

/// Extrait le texte d'UN enregistrement `"type":"user"` : `message.content` est soit une chaîne
/// directe, soit un tableau de blocs (`[{"type":"text","text":"..."}]`, forme la plus courante
/// des transcripts réels) — on prend le premier bloc texte rencontré. `None` si ni l'une ni
/// l'autre forme n'est exploitable.
fn extract_user_text(v: &serde_json::Value) -> Option<String> {
    let content = v.get("message")?.get("content")?;
    if let Some(s) = content.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = content.as_array() {
        for block in arr {
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                    return Some(t.to_string());
                }
            }
        }
    }
    None
}

/// Une balise de commande/système (`<command-message>`, `<system-reminder>`…) plutôt qu'un
/// message utilisateur réel : heuristique volontairement générique (un vrai message écrit par un
/// humain ou un agent ne commence quasiment jamais par `<`) plutôt qu'une énumération de balises
/// connues, forcément incomplète et vouée à se périmer.
fn is_service_tag(text: &str) -> bool {
    text.trim_start().starts_with('<')
}

/// Réduit un message à un libellé d'une ligne, tronqué à `max_chars` CARACTÈRES (pas octets, un
/// caractère accentué ne doit jamais couper le compte), avec « … » si tronqué. Les retours à la
/// ligne internes sont aplatis (une tâche tient sur une ligne de tableau).
fn truncate_task(text: &str, max_chars: usize) -> String {
    let one_line: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let char_count = one_line.chars().count();
    if char_count <= max_chars {
        one_line
    } else {
        let truncated: String = one_line.chars().take(max_chars).collect();
        format!("{truncated}…")
    }
}

/// Cherche, dans les lignes d'une tête de transcript DÉJÀ LUE, le premier message UTILISATEUR
/// RÉEL — hors lignes de service (`type` différent de `"user"`, déjà écarté structurellement) et
/// hors balises de commande/système ([`is_service_tag`]) — SANS troncature (réemployé tel quel
/// pour la grammaire de `estimate.rs`, qui a besoin des lignes complètes, pas d'un extrait
/// tronqué à 60 caractères). Pure (aucune E/S), réutilisable pour la lecture combinée
/// (DA-T7 de temps-restant.md).
fn raw_user_snippet_from_head(head: &str) -> Option<String> {
    for line in head.lines() {
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v.get("type").and_then(|t| t.as_str()) != Some("user") {
            continue;
        }
        let Some(text) = extract_user_text(&v) else {
            continue;
        };
        if text.trim().is_empty() || is_service_tag(&text) {
            continue;
        }
        return Some(text);
    }
    None
}

/// Cherche, dans les lignes de tête (bornées en octets) d'un transcript, le premier message
/// utilisateur réel, tronqué (point 11 de liste-des-agents.md). `None` si rien d'exploitable
/// n'a été trouvé dans la fenêtre lue.
fn first_user_message_snippet(path: &Path, cap: usize) -> Option<String> {
    let head = read_head_capped(path, cap)?;
    if let Some(text) = raw_user_snippet_from_head(&head) {
        return Some(truncate_task(&text, TASK_SNIPPET_MAX_CHARS));
    }
    None
}

/// Libellé de tâche du coordinateur (point 11) : début du premier message utilisateur réel,
/// repli `"coordinateur"` si le transcript est absent ou si rien d'exploitable n'y a été trouvé.
fn coordinator_task_label(coordinator_transcript: Option<&Path>) -> String {
    coordinator_transcript
        .and_then(|p| first_user_message_snippet(p, TRANSCRIPT_HEAD_CAP))
        .unwrap_or_else(|| "coordinateur".to_string())
}

// ===================== Découverte des fichiers de session =====================

/// Un transcript de sous-agent identifié structurellement : `<sid>/subagents/agent-<id>.jsonl`.
/// Extrait l'identifiant `<id>` du nom de fichier. `None` si le chemin ne correspond pas à ce
/// patron.
fn subagent_id(path: &Path) -> Option<String> {
    let parent = path.parent()?;
    if parent.file_name()?.to_str()? != "subagents" {
        return None;
    }
    let stem = path.file_stem()?.to_str()?;
    stem.strip_prefix("agent-").map(str::to_string)
}

type SessionKey = (PathBuf, String);

struct RawSubagent {
    agent_id: String,
    transcript_path: PathBuf,
}

#[derive(Default)]
struct RawSession {
    coordinator_transcript: Option<PathBuf>,
    subagents: Vec<RawSubagent>,
}

/// Classe tous les transcripts sous `projects_dir` par session : `<sid>.jsonl` = transcript du
/// coordinateur ; `<sid>/subagents/agent-<id>.jsonl` = transcript de sous-agent.
fn group_sessions(projects_dir: &Path) -> HashMap<SessionKey, RawSession> {
    let mut sessions: HashMap<SessionKey, RawSession> = HashMap::new();
    for path in claude_transcript_files(projects_dir) {
        if let Some(agent_id) = subagent_id(&path) {
            let Some(subagents_dir) = path.parent() else {
                continue;
            };
            let Some(sid_dir) = subagents_dir.parent() else {
                continue;
            };
            let Some(escaped_dir) = sid_dir.parent() else {
                continue;
            };
            let Some(sid) = sid_dir.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let key = (escaped_dir.to_path_buf(), sid.to_string());
            sessions
                .entry(key)
                .or_default()
                .subagents
                .push(RawSubagent {
                    agent_id,
                    transcript_path: path,
                });
        } else {
            let Some(escaped_dir) = path.parent() else {
                continue;
            };
            let Some(sid) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let key = (escaped_dir.to_path_buf(), sid.to_string());
            sessions.entry(key).or_default().coordinator_transcript = Some(path);
        }
    }
    sessions
}

// ========================== Arbre de délégation (pur, testé) ==========================

/// Nœud d'agent déjà résolu pour l'affichage : sprite + champs structurés (persona, tâche,
/// libellés d'activité déjà formatés en Rust, D9) + enfants imbriqués (indentation de délégation,
/// amendement A1 du 26/09).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentNode {
    pub sprite: Sprite,
    /// Nom du persona (`agentType`), `"odin"` pour le coordinateur, `"inconnu"` si sidecar
    /// absent/invalide.
    pub persona: String,
    /// Tâche confiée (`description` du sidecar). `"coordinateur"` pour la racine (aucune tâche
    /// propre), vide si le sidecar n'en porte pas.
    pub task: String,
    /// « Actif il y a », déjà formaté (D9).
    pub active_ago: String,
    /// Secondes brutes derrière `active_ago` — sert UNIQUEMENT au tri côté webview (aucune
    /// décision de présentation, la mise en forme reste entièrement en Rust, D9).
    pub active_secs: i64,
    /// « Démarré il y a », déjà formaté (D9) ; vide si la date de création est indisponible.
    pub started_ago: String,
    /// Secondes brutes derrière `started_ago` — sert UNIQUEMENT au tri côté webview. `None` si
    /// la date de création est indisponible (trié en dernier, cf. `render.ts`).
    pub started_secs: Option<i64>,
    /// Modèle utilisé par cet agent, déjà résolu en libellé lisible (amendement A1, ex.
    /// « Opus 5.5 ») depuis `message.model` du premier enregistrement assistant de SON PROPRE
    /// transcript (coordinateur ou sous-agent — jamais celui d'un autre). `"—"` si absent/illisible.
    pub model: String,
    /// Temps restant estimé (specs/instructions/temps-restant.md), `None` pour le coordinateur,
    /// le marqueur de débordement, ou tout sous-agent sans estimation annoncée (colonne « Reste »
    /// affiche alors un tiret, jamais de barre inventée).
    pub estimate: Option<estimate::EstimateView>,
    /// Infobulle complète (persona — tâche — actif il y a — remontée d'ancêtre mort le cas
    /// échéant, estimation le cas échéant) : conservée pour le survol, en plus des colonnes
    /// structurées.
    pub tooltip: String,
    pub children: Vec<AgentNode>,
    /// Nombre d'agents au-delà de la borne (8 enfants max) — `None` si aucun débordement.
    pub overflow: Option<u32>,
}

/// Une session vivante affichée dans le tableau : libellé de projet + racine (coordinateur).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionNode {
    pub project: String,
    pub coordinator: AgentNode,
}

/// Sous-agent résolu (meta + liveness + horodatages), prêt pour la construction de l'arbre.
#[derive(Debug, Clone)]
struct ResolvedSubagent {
    agent_id: String,
    meta: AgentMeta,
    live: bool,
    mtime: Option<i64>,
    created: Option<i64>,
    /// Modèle de CE sous-agent (amendement A1), déjà résolu en libellé lisible.
    model: String,
    /// Libellé court (ex. `description`) débarrassé de son éventuel suffixe d'estimation valide
    /// (specs/instructions/temps-restant.md) — devient la tâche affichée.
    task: String,
    /// Estimation résolue (grammaire de la consigne ou du suffixe, priorité à la consigne),
    /// avant mise en forme (celle-ci a besoin du temps écoulé, connu seulement au moment de
    /// construire l'arbre, cf. `build_children`).
    estimate: Option<estimate::Estimate>,
}

/// Résout un `RawSubagent` (lecture du sidecar + mtime + liveness + modèle + estimation).
/// Défensif : sidecar absent/illisible/invalide -> `AgentMeta::default()` (type inconnu), jamais
/// ignoré. DA-T7 (temps-restant.md) : la tête du transcript est lue **une seule fois** ici, et
/// sert à la fois au modèle et à la consigne (source par runner, ici Claude Code).
fn resolve_subagent(raw: &RawSubagent, now: i64, liveness_secs: i64) -> ResolvedSubagent {
    let sidecar = raw.transcript_path.with_extension("meta.json");
    let meta = read_agent_meta(&sidecar).unwrap_or_default();
    let mtime = mtime_of(&raw.transcript_path);
    let live = mtime.is_some_and(|m| is_live(now, m, liveness_secs));
    let created = created_of(&raw.transcript_path);

    let head = read_head_capped(&raw.transcript_path, TRANSCRIPT_HEAD_CAP);
    let model = head
        .as_deref()
        .and_then(model_from_head)
        .map(|m| format_model_label(&m))
        .unwrap_or_else(|| "—".to_string());
    let raw_consigne = head.as_deref().and_then(raw_user_snippet_from_head);

    // Source Claude Code du contrat neutre estimate::DelegationText (specs/instructions/
    // temps-restant.md § Couche « source par runner ») : consigne = premier message utilisateur
    // réel de CE sous-agent ; libellé court = description du compagnon.
    let delegation = estimate::DelegationText {
        consigne: raw_consigne,
        short_label: meta.description.clone(),
    };
    let (resolved_task, resolved_estimate) = estimate::resolve(&delegation);

    ResolvedSubagent {
        agent_id: raw.agent_id.clone(),
        meta,
        live,
        mtime,
        created,
        model,
        task: resolved_task.unwrap_or_default(),
        estimate: resolved_estimate,
    }
}

/// Construit le nœud de session affiché + son activité (pour le tri), ou `None` si aucun agent
/// n'y est vivant.
fn build_session_node(
    escaped_dir: &Path,
    raw: &RawSession,
    now: i64,
    liveness_secs: i64,
) -> Option<(SessionNode, i64)> {
    let coordinator_mtime = raw.coordinator_transcript.as_deref().and_then(mtime_of);
    let coordinator_created = raw.coordinator_transcript.as_deref().and_then(created_of);
    let coordinator_own_live = coordinator_mtime.is_some_and(|m| is_live(now, m, liveness_secs));

    let resolved: Vec<ResolvedSubagent> = raw
        .subagents
        .iter()
        .map(|s| resolve_subagent(s, now, liveness_secs))
        .collect();

    let any_descendant_live = resolved.iter().any(|s| s.live);
    if !coordinator_own_live && !any_descendant_live {
        return None; // coordinateur froid, aucun descendant vivant -> session absente.
    }

    let by_id: HashMap<String, &ResolvedSubagent> = resolved
        .iter()
        .map(|s| (s.agent_id.clone(), s))
        .collect();

    // Parent d'affichage de chaque agent VIVANT : on remonte la chaîne `parentAgentId` jusqu'au
    // premier ancêtre vivant ; à défaut, coordinateur (`None`).
    let mut display_parent: HashMap<String, Option<String>> = HashMap::new();
    for s in resolved.iter().filter(|s| s.live) {
        let mut current = s.meta.parent_agent_id.clone();
        let mut seen: HashSet<String> = HashSet::new();
        let mut found: Option<String> = None;
        while let Some(pid) = current {
            if !seen.insert(pid.clone()) {
                break; // cycle défensif -> rattaché au coordinateur.
            }
            match by_id.get(&pid) {
                Some(p) if p.live => {
                    found = Some(pid);
                    break;
                }
                Some(p) => current = p.meta.parent_agent_id.clone(),
                None => break,
            }
        }
        display_parent.insert(s.agent_id.clone(), found);
    }

    let mut children_of: HashMap<Option<String>, Vec<String>> = HashMap::new();
    for s in resolved.iter().filter(|s| s.live) {
        let parent = display_parent.get(&s.agent_id).cloned().flatten();
        children_of.entry(parent).or_default().push(s.agent_id.clone());
    }

    let coordinator_children = build_children(None, &children_of, &by_id, &display_parent, now);
    let elapsed = coordinator_mtime.map(|m| (now - m).max(0)).unwrap_or(0);
    let started_secs = coordinator_created.map(|c| (now - c).max(0));
    let coordinator_model = raw
        .coordinator_transcript
        .as_deref()
        .map(model_label_for_transcript)
        .unwrap_or_else(|| "—".to_string());
    // Tâche de la session principale (amendement A3, point 11) : début du premier message
    // utilisateur réel (hors lignes de service, hors balises de commande/système), repli
    // "coordinateur" si rien d'exploitable n'a été trouvé dans la fenêtre lue.
    let coordinator_task = coordinator_task_label(raw.coordinator_transcript.as_deref());
    let coordinator = AgentNode {
        sprite: coordinator_sprite(),
        persona: display_persona("odin"),
        task: coordinator_task,
        active_ago: format_active_ago(elapsed),
        active_secs: elapsed,
        started_ago: format_started_ago(started_secs),
        started_secs,
        model: coordinator_model,
        // DA-T6 (temps-restant.md) : la session principale n'a jamais d'estimation.
        estimate: None,
        tooltip: format!("Odin (coordinateur) — actif il y a {elapsed} s"),
        children: coordinator_children,
        overflow: None,
    };

    let project = project_label_for_session(escaped_dir, raw.coordinator_transcript.as_deref());

    let activity = std::iter::once(coordinator_mtime.unwrap_or(i64::MIN))
        .chain(resolved.iter().filter_map(|s| s.mtime))
        .max()
        .unwrap_or(i64::MIN);

    Some((SessionNode { project, coordinator }, activity))
}

/// Combine une estimation résolue (grammaire) et une date de début en une vue affichable
/// (DA-T1, temps-restant.md) : l'une ou l'autre absente -> pas de barre, jamais de valeur
/// inventée à partir d'une base de temps inconnue.
fn resolve_estimate_view(
    parsed: Option<&estimate::Estimate>,
    started_secs: Option<i64>,
) -> Option<estimate::EstimateView> {
    match (parsed, started_secs) {
        (Some(e), Some(started)) => Some(estimate::estimate_view(e, started)),
        _ => None,
    }
}

/// Construit les enfants (triés, bornés à 8 + marqueur `+N`) d'un parent d'affichage donné
/// (`None` = coordinateur). Récursif.
fn build_children(
    parent: Option<String>,
    children_of: &HashMap<Option<String>, Vec<String>>,
    by_id: &HashMap<String, &ResolvedSubagent>,
    display_parent: &HashMap<String, Option<String>>,
    now: i64,
) -> Vec<AgentNode> {
    let Some(ids) = children_of.get(&parent) else {
        return Vec::new();
    };
    let mut kids: Vec<&ResolvedSubagent> = ids
        .iter()
        .filter_map(|id| by_id.get(id).copied())
        .collect();
    // Tri : date de création du transcript croissante (ordre de délégation), repli agentId
    // lexicographique.
    kids.sort_by(|a, b| {
        a.created
            .unwrap_or(i64::MAX)
            .cmp(&b.created.unwrap_or(i64::MAX))
            .then_with(|| a.agent_id.cmp(&b.agent_id))
    });
    let overflow = kids.len().saturating_sub(MAX_CHILDREN);
    let mut nodes: Vec<AgentNode> = kids
        .into_iter()
        .take(MAX_CHILDREN)
        .map(|s| {
            let sprite = sprite_for(&s.meta.agent_type);
            let elapsed = s.mtime.map(|m| (now - m).max(0)).unwrap_or(0);
            // Clé brute (minuscule) pour la prose de l'infobulle ; nom capitalisé (amendement A3,
            // point 7) pour la colonne Agent du tableau — deux usages, deux formes.
            let persona_key = if s.meta.agent_type.is_empty() {
                "inconnu".to_string()
            } else {
                s.meta.agent_type.to_ascii_lowercase()
            };
            let persona = display_persona(&s.meta.agent_type);
            // Tâche déjà débarrassée d'un éventuel suffixe d'estimation valide
            // (specs/instructions/temps-restant.md, resolve() posé dans resolve_subagent).
            let desc = s.task.clone();
            let mut tooltip = if desc.is_empty() {
                format!("{persona_key} — actif il y a {elapsed} s")
            } else {
                format!("{persona_key} — {desc} — actif il y a {elapsed} s")
            };
            // Parent réel différent du parent d'affichage (remontée) : le mentionner.
            let display_p = display_parent.get(&s.agent_id).cloned().flatten();
            if s.meta.parent_agent_id != display_p {
                let real = s
                    .meta
                    .parent_agent_id
                    .as_ref()
                    .and_then(|pid| by_id.get(pid))
                    .map(|p| {
                        if p.meta.agent_type.is_empty() {
                            "inconnu".to_string()
                        } else {
                            p.meta.agent_type.clone()
                        }
                    })
                    .unwrap_or_else(|| "coordinateur".to_string());
                tooltip.push_str(&format!(" (parent réel : {real}, terminé)"));
            }
            let children = build_children(
                Some(s.agent_id.clone()),
                children_of,
                by_id,
                display_parent,
                now,
            );
            let started_secs = s.created.map(|c| (now - c).max(0));
            // Vue d'estimation (colonne « Reste », specs/instructions/temps-restant.md) : DA-T1,
            // le temps écoulé se calcule depuis le DÉBUT de la délégation (même source que
            // « démarré il y a »), pas depuis la dernière activité. Date de début indisponible
            // -> pas de barre, même si une estimation a bien été annoncée (DA-T1 : "indisponible
            // ⇒ pas de barre").
            let estimate_view = resolve_estimate_view(s.estimate.as_ref(), started_secs);
            if let (Some(est), Some(view)) = (&s.estimate, &estimate_view) {
                let announced = estimate::format_estimate_announced(est);
                if view.remaining_secs < 0 {
                    tooltip.push_str(&format!(" — estimé {announced}, dépassement {}", view.short_label));
                } else {
                    tooltip.push_str(&format!(" — estimé {announced}, reste {}", view.short_label));
                }
            }
            AgentNode {
                sprite,
                persona,
                task: desc,
                active_ago: format_active_ago(elapsed),
                active_secs: elapsed,
                started_ago: format_started_ago(started_secs),
                started_secs,
                model: s.model.clone(),
                estimate: estimate_view,
                tooltip,
                children,
                overflow: None,
            }
        })
        .collect();
    if overflow > 0 {
        nodes.push(AgentNode {
            sprite: Sprite {
                letter: '+',
                bg: FALLBACK_BG,
                fg: FALLBACK_FG,
            },
            persona: "+".to_string(),
            task: String::new(),
            active_ago: String::new(),
            active_secs: 0,
            started_ago: String::new(),
            started_secs: None,
            model: String::new(),
            // DA-T6 (temps-restant.md) : le marqueur de débordement n'a jamais d'estimation.
            estimate: None,
            tooltip: format!("+{overflow} agent(s) supplémentaire(s)"),
            children: Vec::new(),
            overflow: Some(overflow as u32),
        });
    }
    nodes
}

// ============================ Label de projet ============================

/// Cherche le `cwd` dans les lignes de tête (bornées) d'un transcript — PAS seulement la
/// première ligne physique (bug corrigé post-gate Legolas du 26/09 : les transcripts réels
/// commencent souvent par des lignes de service sans `cwd`) — et le projette via `project_of`.
/// Repli : nom du dossier de projet échappé, tel quel. Amendement A5, point 2 : une session dont
/// le dossier de travail n'est pas un projet iakaframe (racine de portefeuille type `work` ou
/// `Desktop`) affiche ce nom de dossier tel quel (ex. « work »), plus jamais le générique
/// « hors projet » — `project_of` seul, sans passer par `bucket_project`.
fn project_label_for_session(escaped_dir: &Path, coordinator_transcript: Option<&Path>) -> String {
    let fallback = || {
        escaped_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    };
    let Some(path) = coordinator_transcript else {
        return fallback();
    };
    let Some(cwd) = first_cwd_in_transcript(path, TRANSCRIPT_HEAD_CAP) else {
        return fallback();
    };
    project_of(&cwd).unwrap_or_else(fallback)
}

// ================= Instantané + commande + watcher =================

/// Instantané complet poussé à la webview.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentsSnapshot {
    /// Effectif total = tous les agents vivants, coordinateurs compris. Overflow inclus.
    pub count: u32,
    pub sessions: Vec<SessionNode>,
    /// Nombre de sessions vivantes au-delà de la borne d'affichage (6 max).
    pub overflow_sessions: Option<u32>,
}

/// Compte les agents réellement vivants d'un nœud (récursif) : la racine + chaque enfant, un
/// marqueur de débordement comptant pour son `overflow` plutôt que pour 1.
fn count_live_agents(node: &AgentNode) -> u32 {
    let mut total = 1;
    for child in &node.children {
        total += child.overflow.unwrap_or_else(|| count_live_agents(child));
    }
    total
}

/// Construit l'instantané courant (pur, sauf l'horloge/le disque passés en paramètre) : marche +
/// résolution + tri + bornes. `now`/`liveness_secs` explicites -> testable sans horloge murale.
pub fn snapshot_at(projects_dir: &Path, now: i64, liveness_secs: i64) -> AgentsSnapshot {
    let raw_sessions = group_sessions(projects_dir);
    let mut with_activity: Vec<(i64, SessionNode)> = Vec::new();
    let mut count: u32 = 0;

    for (key, raw) in &raw_sessions {
        let Some((node, activity)) = build_session_node(&key.0, raw, now, liveness_secs) else {
            continue;
        };
        count += count_live_agents(&node.coordinator);
        with_activity.push((activity, node));
    }

    // Tri par activité décroissante. `key.1` sert de repli déterministe en cas d'égalité.
    with_activity.sort_by_key(|(activity, _)| std::cmp::Reverse(*activity));

    let overflow = with_activity.len().saturating_sub(MAX_SESSIONS);
    let sessions: Vec<SessionNode> = with_activity
        .into_iter()
        .take(MAX_SESSIONS)
        .map(|(_, n)| n)
        .collect();

    AgentsSnapshot {
        count,
        sessions,
        overflow_sessions: if overflow > 0 {
            Some(overflow as u32)
        } else {
            None
        },
    }
}

/// Instantané courant, horloge murale + seuil configuré (`IAKAAM_LIVENESS_SECS`, DA-1).
/// Répertoire des projets introuvable (HOME absent) -> instantané vide (défensif).
fn current_snapshot() -> AgentsSnapshot {
    let now = now_secs();
    let liveness = liveness_secs_from_env() as i64;
    match claude_projects_dir() {
        Some(dir) => snapshot_at(&dir, now, liveness),
        None => AgentsSnapshot::default(),
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Commande : instantané des agents en cours à l'ouverture de la fenêtre. Lit l'instantané déjà
/// posé par le watcher dans `AppState` : pas de re-scan disque synchrone au moment de l'appel.
#[tauri::command]
pub fn get_running_agents(state: tauri::State<'_, AppState>) -> AgentsSnapshot {
    state.running_agents.lock().unwrap().clone()
}

/// Lance le watcher dédié dans un thread détaché. Tick 5 s, métadonnées seules ; émet
/// `tray://agents` UNIQUEMENT si l'instantané a changé et met à jour le titre du tray (compteur).
pub fn start_watcher(app: AppHandle) {
    std::thread::spawn(move || run_watcher(app));
}

fn run_watcher(app: AppHandle) {
    let mut last: Option<AgentsSnapshot> = None;
    loop {
        let snap = current_snapshot();
        {
            let state = app.state::<AppState>();
            *state.running_agents.lock().unwrap() = snap.clone();
        }
        if last.as_ref() != Some(&snap) {
            let _ = app.emit(AGENTS_EVENT, &snap);
            crate::tray::update_title(&app, snap.count);
            last = Some(snap);
        }
        std::thread::sleep(TICK);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ---------------------------- Roster ----------------------------

    #[test]
    fn sprite_for_rend_les_neuf_personas_du_roster() {
        // Amendement A3 (contraste AA + lettre distincte pour Gimli) puis A4 (lettre distincte
        // pour Legolas), cf. doc du ROSTER.
        assert_eq!(sprite_for("odin"), Sprite { letter: 'O', bg: "#FFD60A", fg: "#0B0D12" });
        assert_eq!(sprite_for("aragorn"), Sprite { letter: 'A', bg: "#FF9F0A", fg: "#0B0D12" });
        assert_eq!(sprite_for("gandalf"), Sprite { letter: 'G', bg: "#0A84FF", fg: "#0B0D12" });
        assert_eq!(sprite_for("gimli"), Sprite { letter: 'I', bg: "#FF3B30", fg: "#0B0D12" });
        assert_eq!(sprite_for("legolas"), Sprite { letter: 'E', bg: "#FF3B30", fg: "#0B0D12" });
        assert_eq!(sprite_for("helm"), Sprite { letter: 'H', bg: "#BF5AF2", fg: "#0B0D12" });
        assert_eq!(sprite_for("loki"), Sprite { letter: 'L', bg: "#FF9F0A", fg: "#0B0D12" });
        assert_eq!(sprite_for("nathalie"), Sprite { letter: 'N', bg: "#FF9F0A", fg: "#0B0D12" });
        assert_eq!(sprite_for("feanor"), Sprite { letter: 'F', bg: "#FF9F0A", fg: "#0B0D12" });
    }

    /// Contraste AA (≥ 4,5:1, WCAG normal-text) de CHAQUE couple (fond, texte) du roster + du
    /// repli hors-roster — amendement A3, point 10. Calcul WCAG standard (luminance relative
    /// sRGB), testé ici plutôt que laissé à une vérification manuelle ponctuelle.
    #[test]
    fn roster_et_repli_respectent_le_contraste_aa() {
        fn srgb_to_lin(c: f64) -> f64 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        fn luminance(hex: &str) -> f64 {
            let hex = hex.trim_start_matches('#');
            let r = i64::from_str_radix(&hex[0..2], 16).unwrap() as f64 / 255.0;
            let g = i64::from_str_radix(&hex[2..4], 16).unwrap() as f64 / 255.0;
            let b = i64::from_str_radix(&hex[4..6], 16).unwrap() as f64 / 255.0;
            0.2126 * srgb_to_lin(r) + 0.7152 * srgb_to_lin(g) + 0.0722 * srgb_to_lin(b)
        }
        fn contrast(c1: &str, c2: &str) -> f64 {
            let (l1, l2) = (luminance(c1), luminance(c2));
            let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
            (hi + 0.05) / (lo + 0.05)
        }
        for &(agent_type, _, bg, fg) in ROSTER {
            let ratio = contrast(bg, fg);
            assert!(
                ratio >= 4.5,
                "{agent_type} : contraste {ratio:.2}:1 sous le seuil AA (4,5:1) — bg={bg} fg={fg}"
            );
        }
        let ratio = contrast(FALLBACK_BG, FALLBACK_FG);
        assert!(
            ratio >= 4.5,
            "repli hors-roster : contraste {ratio:.2}:1 sous le seuil AA — bg={FALLBACK_BG} fg={FALLBACK_FG}"
        );
    }

    #[test]
    fn gimli_a_une_lettre_distincte_de_gandalf() {
        // Point 10 de l'amendement A3, exigence explicite de Stéphane.
        assert_ne!(sprite_for("gimli").letter, sprite_for("gandalf").letter);
    }

    #[test]
    fn display_persona_capitalise_le_roster_et_gere_feanor() {
        assert_eq!(display_persona("odin"), "Odin");
        assert_eq!(display_persona("GIMLI"), "Gimli");
        assert_eq!(display_persona("feanor"), "Fëanor");
        assert_eq!(display_persona(""), "Inconnu");
        assert_eq!(display_persona("general-purpose"), "General-purpose");
    }

    #[test]
    fn sprite_for_type_hors_roster_prend_sa_propre_lettre_en_gris() {
        let s = sprite_for("claude-code-guide");
        assert_eq!(s, Sprite { letter: 'C', bg: FALLBACK_BG, fg: FALLBACK_FG });
        let s2 = sprite_for("general-purpose");
        assert_eq!(s2.letter, 'G');
        assert_eq!(s2.bg, FALLBACK_BG);
    }

    #[test]
    fn sprite_for_type_vide_ne_panique_pas() {
        let s = sprite_for("");
        assert_eq!(s, Sprite { letter: '?', bg: FALLBACK_BG, fg: FALLBACK_FG });
    }

    #[test]
    fn sprite_for_est_insensible_a_la_casse() {
        assert_eq!(sprite_for("GIMLI"), sprite_for("gimli"));
    }

    #[test]
    fn roster_couples_lettre_fond_deux_a_deux_distincts() {
        let mut seen: HashSet<(char, &'static str)> = HashSet::new();
        for &(agent_type, letter, bg, _) in ROSTER {
            let key = (letter, bg);
            assert!(
                seen.insert(key),
                "couple (lettre, fond) duplique pour {agent_type} : {key:?}"
            );
        }
        assert_eq!(seen.len(), ROSTER.len());
    }

    #[test]
    fn coordinator_sprite_est_odin() {
        assert_eq!(coordinator_sprite(), sprite_for("odin"));
        assert_eq!(coordinator_sprite().letter, 'O');
    }

    // ---------------------------- Sidecar meta.json ----------------------------

    #[test]
    fn parse_agent_meta_profondeur_1_sans_parent_agent_id() {
        let json = r#"{"agentType":"gandalf","description":"Cadrer la feature agents en cours",
 "toolUseId":"toolu_013irH9ZFSiCUTwm3qKK8YXT","spawnDepth":1,
 "requestShape":"background","requestNonInteractive":true}"#;
        let meta = parse_agent_meta(json).expect("json valide");
        assert_eq!(meta.agent_type, "gandalf");
        assert_eq!(meta.description.as_deref(), Some("Cadrer la feature agents en cours"));
        assert_eq!(meta.parent_agent_id, None);
        assert_eq!(meta.spawn_depth, 1);
    }

    #[test]
    fn parse_agent_meta_profondeur_2_avec_parent_agent_id() {
        let json = r#"{"agentType":"loki","description":"Stand down Loki","toolUseId":"toolu_01ARZQBw",
 "parentAgentId":"a32f41f2eb6ff10ea","spawnDepth":2,
 "requestShape":"background","requestNonInteractive":true}"#;
        let meta = parse_agent_meta(json).expect("json valide");
        assert_eq!(meta.agent_type, "loki");
        assert_eq!(meta.parent_agent_id.as_deref(), Some("a32f41f2eb6ff10ea"));
        assert_eq!(meta.spawn_depth, 2);
    }

    #[test]
    fn parse_agent_meta_json_invalide_est_none() {
        assert_eq!(parse_agent_meta("pas du json"), None);
        assert_eq!(parse_agent_meta(""), None);
        assert_eq!(parse_agent_meta("{"), None);
    }

    #[test]
    fn parse_agent_meta_champ_manquant_retombe_sur_le_defaut() {
        let meta = parse_agent_meta(r#"{"agentType":"gimli"}"#).expect("json valide");
        assert_eq!(meta.agent_type, "gimli");
        assert_eq!(meta.description, None);
        assert_eq!(meta.parent_agent_id, None);
        assert_eq!(meta.spawn_depth, 0);
        let empty = parse_agent_meta("{}").expect("objet vide valide");
        assert_eq!(empty.agent_type, "");
    }

    #[test]
    fn read_agent_meta_fichier_absent_est_none() {
        assert_eq!(read_agent_meta(Path::new("/dossier/inexistant/agent-x.meta.json")), None);
    }

    // ---------------------------- Liveness ----------------------------

    #[test]
    fn is_live_bornes_0_89_90_91() {
        let now = 1_000_000i64;
        assert!(is_live(now, now, 90));
        assert!(is_live(now, now - 89, 90));
        assert!(is_live(now, now - 90, 90));
        assert!(!is_live(now, now - 91, 90));
    }

    #[test]
    fn is_live_horloge_en_avance_reste_vivant() {
        assert!(is_live(1_000_000, 1_000_100, 90));
    }

    // ---------------------------- Formatage des durées (D9, etape 4) ----------------------------

    #[test]
    fn format_started_ago_45_secondes() {
        assert_eq!(format_started_ago(Some(45)), "45 s");
    }

    #[test]
    fn format_started_ago_5_minutes() {
        assert_eq!(format_started_ago(Some(5 * 60)), "5 min");
    }

    #[test]
    fn format_started_ago_1h12() {
        assert_eq!(format_started_ago(Some(3600 + 12 * 60)), "1 h 12 min");
    }

    #[test]
    fn format_started_ago_heure_ronde() {
        assert_eq!(format_started_ago(Some(2 * 3600)), "2 h");
    }

    #[test]
    fn format_started_ago_date_absente_est_omise() {
        assert_eq!(format_started_ago(None), "");
    }

    #[test]
    fn format_active_ago_sous_60_secondes() {
        assert_eq!(format_active_ago(0), "0 s");
        assert_eq!(format_active_ago(45), "45 s");
    }

    /// Amendement A5, point 1 : au-delà de 60 s, même format que « Démarré il y a » — constat
    /// réel corrigé, jamais « 371 s ».
    #[test]
    fn format_active_ago_au_dela_de_60_secondes_comme_demarre() {
        assert_eq!(format_active_ago(60), "1 min");
        assert_eq!(format_active_ago(371), "6 min");
        assert_eq!(format_active_ago(3600 + 5 * 60), "1 h 5 min");
        assert_eq!(format_active_ago(2 * 3600), "2 h");
    }

    #[test]
    fn liveness_secs_from_env_defaut_sans_variable() {
        std::env::remove_var(LIVENESS_ENV);
        assert_eq!(liveness_secs_from_env(), DEFAULT_LIVENESS_SECS);
    }

    /// Dossier temporaire unique pour un test de découverte de sessions.
    fn tmp_projects_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("iaka-am-agents-{name}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_with_mtime(path: &Path, content: &str, age_secs: i64) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
        let mtime = SystemTime::now() - Duration::from_secs(age_secs.max(0) as u64);
        let f = fs::OpenOptions::new().write(true).open(path).unwrap();
        f.set_times(std::fs::FileTimes::new().set_modified(mtime)).unwrap();
    }

    fn session_line(cwd: &str) -> String {
        format!(r#"{{"type":"user","cwd":"{cwd}","message":{{"content":"hello"}}}}"#)
    }

    #[test]
    fn group_sessions_classe_coordinateur_et_sous_agent() {
        let dir = tmp_projects_dir("group");
        let escaped = dir.join("-w-proj");
        write_with_mtime(&escaped.join("sidA.jsonl"), &session_line("/w/proj"), 0);
        write_with_mtime(&escaped.join("sidA/subagents/agent-1.jsonl"), "{}", 0);
        let sessions = group_sessions(&dir);
        assert_eq!(sessions.len(), 1);
        let (_key, raw) = sessions.iter().next().unwrap();
        assert!(raw.coordinator_transcript.is_some());
        assert_eq!(raw.subagents.len(), 1);
        assert_eq!(raw.subagents[0].agent_id, "1");
    }

    #[test]
    fn group_sessions_ignore_un_jsonl_de_subagents_sans_patron_agent() {
        let dir = tmp_projects_dir("journal");
        let escaped = dir.join("-w-proj");
        write_with_mtime(&escaped.join("sidA.jsonl"), &session_line("/w/proj"), 0);
        write_with_mtime(&escaped.join("sidA/subagents/journal.jsonl"), "{}", 0);
        let sessions = group_sessions(&dir);
        let (_key, raw) = sessions.iter().next().unwrap();
        assert!(raw.subagents.is_empty());
    }

    // ---------------------------- Arbre de délégation ----------------------------

    fn write_meta(path: &Path, agent_type: &str, parent: Option<&str>, depth: u32) {
        let parent_json = match parent {
            Some(p) => format!(r#","parentAgentId":"{p}""#),
            None => String::new(),
        };
        let json = format!(
            r#"{{"agentType":"{agent_type}","spawnDepth":{depth}{parent_json}}}"#
        );
        fs::write(path, json).unwrap();
    }

    #[test]
    fn arbre_parent_plus_deux_enfants() {
        let dir = tmp_projects_dir("arbre-2-enfants");
        let escaped = dir.join("-w-proj");
        write_with_mtime(&escaped.join("sid.jsonl"), &session_line("/w/proj"), 0);
        write_with_mtime(&escaped.join("sid/subagents/agent-1.jsonl"), "{}", 0);
        write_meta(&escaped.join("sid/subagents/agent-1.meta.json"), "gandalf", None, 1);
        write_with_mtime(&escaped.join("sid/subagents/agent-2.jsonl"), "{}", 0);
        write_meta(&escaped.join("sid/subagents/agent-2.meta.json"), "loki", None, 1);

        let snap = snapshot_at(&dir, now_for_tests(), 90);
        assert_eq!(snap.sessions.len(), 1);
        let coord = &snap.sessions[0].coordinator;
        assert_eq!(coord.sprite, coordinator_sprite());
        assert_eq!(coord.children.len(), 2);
        let letters: Vec<char> = coord.children.iter().map(|c| c.sprite.letter).collect();
        assert!(letters.contains(&'G')); // gandalf
        assert!(letters.contains(&'L')); // loki
        assert_eq!(snap.count, 3); // coordinateur + 2 enfants
    }

    #[test]
    fn arbre_petit_enfant_profondeur_2_est_imbrique() {
        let dir = tmp_projects_dir("arbre-imbrique");
        let escaped = dir.join("-w-proj");
        write_with_mtime(&escaped.join("sid.jsonl"), &session_line("/w/proj"), 0);
        write_with_mtime(&escaped.join("sid/subagents/agent-parent.jsonl"), "{}", 0);
        write_meta(&escaped.join("sid/subagents/agent-parent.meta.json"), "gandalf", None, 1);
        write_with_mtime(&escaped.join("sid/subagents/agent-child.jsonl"), "{}", 0);
        write_meta(
            &escaped.join("sid/subagents/agent-child.meta.json"),
            "nathalie",
            Some("parent"),
            2,
        );

        let snap = snapshot_at(&dir, now_for_tests(), 90);
        let coord = &snap.sessions[0].coordinator;
        assert_eq!(coord.children.len(), 1, "un seul enfant direct : gandalf");
        let gandalf = &coord.children[0];
        assert_eq!(gandalf.sprite.letter, 'G');
        assert_eq!(gandalf.children.len(), 1, "nathalie imbriquee sous gandalf");
        assert_eq!(gandalf.children[0].sprite.letter, 'N');
        assert_eq!(snap.count, 3);
    }

    #[test]
    fn arbre_parent_mort_enfant_vivant_remonte_au_plus_proche_ancetre_vivant() {
        let dir = tmp_projects_dir("parent-mort");
        let escaped = dir.join("-w-proj");
        write_with_mtime(&escaped.join("sid.jsonl"), &session_line("/w/proj"), 0);
        write_with_mtime(&escaped.join("sid/subagents/agent-parent.jsonl"), "{}", 200);
        write_meta(&escaped.join("sid/subagents/agent-parent.meta.json"), "gandalf", None, 1);
        write_with_mtime(&escaped.join("sid/subagents/agent-child.jsonl"), "{}", 0);
        write_meta(
            &escaped.join("sid/subagents/agent-child.meta.json"),
            "nathalie",
            Some("parent"),
            2,
        );

        let snap = snapshot_at(&dir, now_for_tests(), 90);
        let coord = &snap.sessions[0].coordinator;
        assert_eq!(coord.children.len(), 1);
        assert_eq!(coord.children[0].sprite.letter, 'N');
        assert!(coord.children[0].tooltip.contains("parent réel"));
        assert_eq!(snap.count, 2);
    }

    #[test]
    fn arbre_coordinateur_froid_avec_enfant_vivant_session_affichee() {
        let dir = tmp_projects_dir("coord-froid");
        let escaped = dir.join("-w-proj");
        write_with_mtime(&escaped.join("sid.jsonl"), &session_line("/w/proj"), 200);
        write_with_mtime(&escaped.join("sid/subagents/agent-1.jsonl"), "{}", 0);
        write_meta(&escaped.join("sid/subagents/agent-1.meta.json"), "helm", None, 1);

        let snap = snapshot_at(&dir, now_for_tests(), 90);
        assert_eq!(snap.sessions.len(), 1, "session affichee malgre coordinateur froid");
        assert_eq!(snap.sessions[0].coordinator.children.len(), 1);
    }

    #[test]
    fn arbre_session_sans_agent_vivant_est_absente() {
        let dir = tmp_projects_dir("tout-mort");
        let escaped = dir.join("-w-proj");
        write_with_mtime(&escaped.join("sid.jsonl"), &session_line("/w/proj"), 200);
        write_with_mtime(&escaped.join("sid/subagents/agent-1.jsonl"), "{}", 200);
        write_meta(&escaped.join("sid/subagents/agent-1.meta.json"), "helm", None, 1);

        let snap = snapshot_at(&dir, now_for_tests(), 90);
        assert!(snap.sessions.is_empty());
        assert_eq!(snap.count, 0);
    }

    #[test]
    fn arbre_meta_invalide_est_type_inconnu_gris_jamais_ignore() {
        let dir = tmp_projects_dir("meta-invalide");
        let escaped = dir.join("-w-proj");
        write_with_mtime(&escaped.join("sid.jsonl"), &session_line("/w/proj"), 0);
        write_with_mtime(&escaped.join("sid/subagents/agent-1.jsonl"), "{}", 0);
        fs::write(escaped.join("sid/subagents/agent-1.meta.json"), "pas du json").unwrap();

        let snap = snapshot_at(&dir, now_for_tests(), 90);
        let coord = &snap.sessions[0].coordinator;
        assert_eq!(coord.children.len(), 1, "agent affiche malgre sidecar invalide");
        assert_eq!(coord.children[0].sprite.bg, FALLBACK_BG);
        assert_eq!(coord.children[0].persona, "Inconnu");
    }

    #[test]
    fn arbre_borne_huit_enfants_avec_marqueur_overflow() {
        let dir = tmp_projects_dir("overflow-enfants");
        let escaped = dir.join("-w-proj");
        write_with_mtime(&escaped.join("sid.jsonl"), &session_line("/w/proj"), 0);
        for i in 0..10 {
            write_with_mtime(&escaped.join(format!("sid/subagents/agent-{i}.jsonl")), "{}", 0);
            write_meta(
                &escaped.join(format!("sid/subagents/agent-{i}.meta.json")),
                "helm",
                None,
                1,
            );
        }
        let snap = snapshot_at(&dir, now_for_tests(), 90);
        let coord = &snap.sessions[0].coordinator;
        assert_eq!(coord.children.len(), 9);
        let marker = coord.children.last().unwrap();
        assert_eq!(marker.overflow, Some(2));
        assert_eq!(snap.count, 11);
    }

    #[test]
    fn arbre_borne_six_sessions_avec_marqueur_overflow_sessions() {
        let dir = tmp_projects_dir("overflow-sessions");
        for i in 0..8 {
            let escaped = dir.join(format!("-w-proj{i}"));
            write_with_mtime(&escaped.join("sid.jsonl"), &session_line(&format!("/w/proj{i}")), 0);
        }
        let snap = snapshot_at(&dir, now_for_tests(), 90);
        assert_eq!(snap.sessions.len(), 6);
        assert_eq!(snap.overflow_sessions, Some(2));
    }

    // ---------------------------- Label de projet ----------------------------

    #[test]
    fn projet_cwd_normal_donne_le_nom_de_projet() {
        let dir = tmp_projects_dir("label-normal");
        let escaped = dir.join("-w-iaka-demo");
        write_with_mtime(&escaped.join("sid.jsonl"), &session_line("/w/iaka-demo"), 0);
        let snap = snapshot_at(&dir, now_for_tests(), 90);
        assert_eq!(snap.sessions[0].project, "iaka-demo");
    }

    /// Amendement A5, point 2 : plus de bucket générique « hors projet » — le nom du dossier de
    /// travail (dernier segment du cwd) est affiché tel quel, même pour une racine de
    /// portefeuille (`work`, `Desktop`).
    #[test]
    fn projet_racine_de_portefeuille_affiche_le_nom_du_dossier() {
        let dir = tmp_projects_dir("label-portefeuille");
        let escaped = dir.join("-w-work");
        write_with_mtime(&escaped.join("sid.jsonl"), &session_line("/w/work"), 0);
        let snap = snapshot_at(&dir, now_for_tests(), 90);
        assert_eq!(snap.sessions[0].project, "work");
    }

    #[test]
    fn projet_fichier_vide_replie_sur_le_dossier_echappe() {
        let dir = tmp_projects_dir("label-vide");
        let escaped = dir.join("-w-mystere");
        write_with_mtime(&escaped.join("sid.jsonl"), "", 0);
        let snap = snapshot_at(&dir, now_for_tests(), 90);
        assert_eq!(snap.sessions[0].project, "-w-mystere");
    }

    #[test]
    fn projet_premiere_ligne_non_json_replie_sur_le_dossier_echappe() {
        let dir = tmp_projects_dir("label-non-json");
        let escaped = dir.join("-w-etrange");
        write_with_mtime(&escaped.join("sid.jsonl"), "pas du json\n", 0);
        let snap = snapshot_at(&dir, now_for_tests(), 90);
        assert_eq!(snap.sessions[0].project, "-w-etrange");
    }

    /// Angle mort du bug post-gate Legolas (26/09) : `session_line(cwd)` (le helper de test
    /// ci-dessus) met TOUJOURS `cwd` en toute première ligne, ce qui masquait le défaut. Un
    /// transcript réel commence par plusieurs lignes de service SANS `cwd` (mode,
    /// permission-mode, atis-latch, file-history-snapshot…) avant la première ligne `user` qui le
    /// porte : le nom de projet doit être trouvé malgré ce préambule, pas seulement en ligne 1.
    #[test]
    fn projet_lu_au_dela_du_preambule_de_service_reel() {
        let dir = tmp_projects_dir("label-preambule");
        let escaped = dir.join("-w-iaka-demo");
        let content = concat!(
            r#"{"type":"mode","mode":"interactive"}"#,
            "\n",
            r#"{"type":"permission-mode","mode":"default"}"#,
            "\n",
            r#"{"type":"atis-latch","latch":false}"#,
            "\n",
            r#"{"type":"file-history-snapshot","files":[]}"#,
            "\n",
            r#"{"type":"user","cwd":"/w/iaka-demo","message":{"content":"hello"}}"#,
        );
        write_with_mtime(&escaped.join("sid.jsonl"), content, 0);
        let snap = snapshot_at(&dir, now_for_tests(), 90);
        assert_eq!(
            snap.sessions[0].project, "iaka-demo",
            "le cwd doit etre trouve au-dela des lignes de service, pas seulement en ligne 1"
        );
    }

    // ---------------------------- Colonne Modele (amendement A1) ----------------------------

    #[test]
    fn format_model_label_opus_avec_date_donne_opus_5_5() {
        assert_eq!(format_model_label("claude-opus-5-5-20260115"), "Opus 5.5");
    }

    #[test]
    fn format_model_label_sonnet_sans_date() {
        assert_eq!(format_model_label("claude-sonnet-4-5"), "Sonnet 4.5");
    }

    #[test]
    fn format_model_label_haiku() {
        assert_eq!(format_model_label("claude-haiku-4-5-20251001"), "Haiku 4.5");
    }

    #[test]
    fn format_model_label_famille_inconnue_repli_identifiant_brut() {
        // Jamais de libellé inventé : famille non reconnue -> l'identifiant brut tel quel.
        assert_eq!(format_model_label("mystery-model-x1"), "mystery-model-x1");
    }

    #[test]
    fn first_assistant_model_in_transcript_au_dela_du_preambule() {
        let dir = tmp_projects_dir("modele-preambule");
        let path = dir.join("agent-1.jsonl");
        let content = concat!(
            r#"{"type":"mode","mode":"interactive"}"#,
            "\n",
            r#"{"type":"user","cwd":"/w/p","message":{"content":"hello"}}"#,
            "\n",
            r#"{"type":"assistant","message":{"model":"claude-opus-5-5-20260115","content":[]}}"#,
        );
        fs::write(&path, content).unwrap();
        assert_eq!(
            first_assistant_model_in_transcript(&path, TRANSCRIPT_HEAD_CAP),
            Some("claude-opus-5-5-20260115".to_string())
        );
        assert_eq!(model_label_for_transcript(&path), "Opus 5.5");
    }

    #[test]
    fn model_label_for_transcript_absent_est_un_tiret() {
        let dir = tmp_projects_dir("modele-absent");
        let path = dir.join("agent-1.jsonl");
        fs::write(&path, r#"{"type":"user","cwd":"/w/p","message":{"content":"hi"}}"#).unwrap();
        assert_eq!(model_label_for_transcript(&path), "—");
    }

    #[test]
    fn snapshot_porte_le_modele_du_coordinateur_et_du_sous_agent() {
        let dir = tmp_projects_dir("modele-snapshot");
        let escaped = dir.join("-w-proj");
        let coord_content = concat!(
            r#"{"type":"user","cwd":"/w/proj","message":{"content":"hello"}}"#,
            "\n",
            r#"{"type":"assistant","message":{"model":"claude-sonnet-4-5-20250929","content":[]}}"#,
        );
        write_with_mtime(&escaped.join("sid.jsonl"), coord_content, 0);
        let sub_content = r#"{"type":"assistant","message":{"model":"claude-haiku-4-5-20251001","content":[]}}"#;
        write_with_mtime(&escaped.join("sid/subagents/agent-1.jsonl"), sub_content, 0);
        write_meta(&escaped.join("sid/subagents/agent-1.meta.json"), "gimli", None, 1);

        let snap = snapshot_at(&dir, now_for_tests(), 90);
        let coord = &snap.sessions[0].coordinator;
        assert_eq!(coord.model, "Sonnet 4.5");
        assert_eq!(coord.children[0].model, "Haiku 4.5");
    }

    // ------------------- Colonne « Reste » (specs/instructions/temps-restant.md) -------------------

    #[test]
    fn resolve_estimate_view_sans_date_de_debut_est_nul() {
        let e = estimate::Estimate {
            lo_secs: None,
            hi_secs: 600,
        };
        assert_eq!(
            resolve_estimate_view(Some(&e), None),
            None,
            "date de debut indisponible -> pas de barre, meme avec une estimation valide"
        );
        assert_eq!(resolve_estimate_view(None, Some(120)), None);
        assert!(resolve_estimate_view(Some(&e), Some(120)).is_some());
    }

    #[test]
    fn arbre_estimation_depuis_la_ligne_de_consigne() {
        let dir = tmp_projects_dir("estimate-ligne");
        let escaped = dir.join("-w-proj");
        write_with_mtime(&escaped.join("sid.jsonl"), &session_line("/w/proj"), 0);
        let sub_content = r#"{"type":"user","message":{"content":"Durée estimée : ~10 min\nfais le travail"}}"#;
        write_with_mtime(&escaped.join("sid/subagents/agent-1.jsonl"), sub_content, 0);
        write_meta(&escaped.join("sid/subagents/agent-1.meta.json"), "gimli", None, 1);

        let snap = snapshot_at(&dir, now_for_tests(), 90);
        let child = &snap.sessions[0].coordinator.children[0];
        assert!(child.estimate.is_some(), "estimation attendue depuis la ligne de consigne");
        assert!(
            child.tooltip.contains("estimé ~10 min"),
            "infobulle DA-T4 attendue : {}",
            child.tooltip
        );
    }

    #[test]
    fn arbre_balises_de_commande_en_lignes_suivantes_ignorees_pour_la_consigne() {
        let dir = tmp_projects_dir("estimate-balises");
        let escaped = dir.join("-w-proj");
        write_with_mtime(&escaped.join("sid.jsonl"), &session_line("/w/proj"), 0);
        let sub_content = concat!(
            r#"{"type":"user","message":{"content":"Durée estimée : ~10 min\nfais le travail"}}"#,
            "\n",
            r#"{"type":"user","message":{"content":"<command-message>iakaframe-fabrication</command-message>"}}"#,
        );
        write_with_mtime(&escaped.join("sid/subagents/agent-1.jsonl"), sub_content, 0);
        write_meta(&escaped.join("sid/subagents/agent-1.meta.json"), "gimli", None, 1);

        let snap = snapshot_at(&dir, now_for_tests(), 90);
        let child = &snap.sessions[0].coordinator.children[0];
        assert!(
            child.estimate.is_some(),
            "la balise de commande en 2e ligne ne doit pas empecher de lire la 1re"
        );
    }

    #[test]
    fn arbre_suffixe_du_compagnon_utilise_quand_le_transcript_n_a_pas_de_ligne() {
        let dir = tmp_projects_dir("estimate-suffixe");
        let escaped = dir.join("-w-proj");
        write_with_mtime(&escaped.join("sid.jsonl"), &session_line("/w/proj"), 0);
        write_with_mtime(&escaped.join("sid/subagents/agent-1.jsonl"), "{}", 0);
        let meta_json = r#"{"agentType":"gimli","description":"Test barre (~10 min)","spawnDepth":1}"#;
        fs::write(escaped.join("sid/subagents/agent-1.meta.json"), meta_json).unwrap();

        let snap = snapshot_at(&dir, now_for_tests(), 90);
        let child = &snap.sessions[0].coordinator.children[0];
        assert!(child.estimate.is_some(), "estimation attendue depuis le suffixe du libelle");
        assert_eq!(child.task, "Test barre", "le suffixe doit etre retire de la tache affichee");
    }

    #[test]
    fn arbre_sans_ligne_ni_suffixe_estimation_nulle() {
        let dir = tmp_projects_dir("estimate-rien");
        let escaped = dir.join("-w-proj");
        write_with_mtime(&escaped.join("sid.jsonl"), &session_line("/w/proj"), 0);
        write_with_mtime(&escaped.join("sid/subagents/agent-1.jsonl"), "{}", 0);
        write_meta(&escaped.join("sid/subagents/agent-1.meta.json"), "gimli", None, 1);

        let snap = snapshot_at(&dir, now_for_tests(), 90);
        let child = &snap.sessions[0].coordinator.children[0];
        assert_eq!(child.estimate, None);
    }

    #[test]
    fn arbre_coordinateur_n_a_jamais_d_estimation() {
        let dir = tmp_projects_dir("estimate-coordinateur");
        let escaped = dir.join("-w-proj");
        let coord_content = r#"{"type":"user","cwd":"/w/proj","message":{"content":"Durée estimée : ~10 min\nbonjour"}}"#;
        write_with_mtime(&escaped.join("sid.jsonl"), coord_content, 0);

        let snap = snapshot_at(&dir, now_for_tests(), 90);
        assert_eq!(
            snap.sessions[0].coordinator.estimate, None,
            "DA-T6 : la session principale n'a jamais d'estimation, meme si sa 1re ligne y ressemble"
        );
    }

    // ------------------- Tâche de la session principale (amendement A3, point 11) -------------------

    #[test]
    fn truncate_task_sous_la_limite_est_inchange() {
        assert_eq!(truncate_task("corriger le bug d'affichage", 60), "corriger le bug d'affichage");
    }

    #[test]
    fn truncate_task_tronque_a_60_caracteres_avec_points_de_suspension() {
        let long = "a".repeat(80);
        let got = truncate_task(&long, 60);
        assert_eq!(got.chars().count(), 61); // 60 caracteres + le glyphe unique "…"
        assert!(got.ends_with('…'));
        assert_eq!(got.chars().filter(|&c| c == 'a').count(), 60);
    }

    #[test]
    fn truncate_task_compte_les_caracteres_pas_les_octets() {
        // Caracteres accentues multi-octets (UTF-8) : la troncature ne doit jamais couper au
        // milieu d'un caractere ni fausser le compte sur un octet.
        let text = "é".repeat(70);
        let got = truncate_task(&text, 60);
        assert_eq!(got.chars().count(), 61);
    }

    #[test]
    fn truncate_task_aplatit_les_retours_a_la_ligne() {
        assert_eq!(truncate_task("ligne un\nligne deux", 60), "ligne un ligne deux");
    }

    #[test]
    fn is_service_tag_detecte_les_balises_hors_prose_reelle() {
        assert!(is_service_tag("<command-message>iakaframe-fabrication</command-message>"));
        assert!(is_service_tag("  <system-reminder>contexte</system-reminder>"));
        assert!(!is_service_tag("corrige le bug d'affichage svp"));
    }

    #[test]
    fn first_user_message_snippet_saute_le_preambule_et_les_balises_de_commande() {
        let dir = tmp_projects_dir("tache-preambule");
        let path = dir.join("sid.jsonl");
        let content = concat!(
            r#"{"type":"mode","mode":"interactive"}"#,
            "\n",
            r#"{"type":"permission-mode","mode":"default"}"#,
            "\n",
            r#"{"type":"user","message":{"content":"<command-message>iakaframe-fabrication</command-message>"}}"#,
            "\n",
            r#"{"type":"assistant","message":{"content":[]}}"#,
            "\n",
            r#"{"type":"user","message":{"content":[{"type":"text","text":"Corrige le bug d'affichage de la popover, merci"}]}}"#,
        );
        fs::write(&path, content).unwrap();
        assert_eq!(
            first_user_message_snippet(&path, TRANSCRIPT_HEAD_CAP),
            Some("Corrige le bug d'affichage de la popover, merci".to_string())
        );
    }

    #[test]
    fn coordinator_task_label_repli_coordinateur_si_rien_d_exploitable() {
        let dir = tmp_projects_dir("tache-absente");
        let path = dir.join("sid.jsonl");
        fs::write(&path, r#"{"type":"mode","mode":"interactive"}"#).unwrap();
        assert_eq!(coordinator_task_label(Some(&path)), "coordinateur");
        assert_eq!(coordinator_task_label(None), "coordinateur");
    }

    #[test]
    fn snapshot_porte_la_tache_reelle_du_coordinateur_tronquee() {
        let dir = tmp_projects_dir("tache-snapshot");
        let escaped = dir.join("-w-proj");
        let long_task = "d".repeat(90);
        let content = format!(
            concat!(
                r#"{{"type":"file-history-snapshot","files":[]}}"#,
                "\n",
                r#"{{"type":"user","cwd":"/w/proj","message":{{"content":[{{"type":"text","text":"{}"}}]}}}}"#,
            ),
            long_task
        );
        write_with_mtime(&escaped.join("sid.jsonl"), &content, 0);
        let snap = snapshot_at(&dir, now_for_tests(), 90);
        let coord = &snap.sessions[0].coordinator;
        assert_eq!(coord.task.chars().count(), 61);
        assert!(coord.task.ends_with('…'));
    }

    // ---------------------------- Emission-si-changement ----------------------------

    #[test]
    fn deux_instantanes_identiques_sont_egaux() {
        let dir = tmp_projects_dir("stabilite");
        let escaped = dir.join("-w-proj");
        write_with_mtime(&escaped.join("sid.jsonl"), &session_line("/w/proj"), 0);
        let now = now_for_tests();
        let a = snapshot_at(&dir, now, 90);
        let b = snapshot_at(&dir, now, 90);
        assert_eq!(a, b, "meme entree, meme instantane -> PartialEq permet de ne pas re-emettre");
    }

    /// Horloge murale figee pour les tests d'arbre.
    fn now_for_tests() -> i64 {
        now_secs()
    }
}
