//! sources::claude_code_files — l'ADAPTATEUR Claude Code (specs/instructions/agnostique-mqtt.md,
//! étape 5) : lecture en cascade des transcripts `~/.claude/projects/` et de leurs compagnons
//! `agent-<id>.meta.json`, produisant des [`crate::card::AgentCard`] — la même monnaie que la
//! source bus (`sources::bus`). Ce module est le SEUL, avec `publishers/claude-code/`, où le nom
//! d'un runner a le droit d'être écrit en dur (grille de neutralité de l'étape 12).
//!
//! ┌────────────────────────────────────────────────────────────────────────────────────────┐
//! │ PROVENANCE : succède à `agents.rs` (lui-même recopié depuis iakaTokenCounter, branche     │
//! │ `worktree-agent-a73ae73650ed5e328`, commit `1c9fd0e`). Toute la construction d'arbre       │
//! │ (Sprite, ROSTER, AgentNode, build_children…) a migré vers le cœur neutre `tree.rs` ; ce    │
//! │ module ne fait plus que DÉCOUVRIR et LIRE — il rend des fiches brutes, jamais un arbre.    │
//! └────────────────────────────────────────────────────────────────────────────────────────┘

use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::card::AgentCard;
use crate::claude_paths::{claude_transcript_files, project_of};
pub use crate::claude_paths::claude_projects_dir;
use crate::estimate;

/// Identifiant de runner de cet adaptateur (contrat, champ `runner`) — SEUL endroit, avec
/// `publishers/claude-code/`, où cette chaîne est écrite.
pub(crate) const RUNNER_ID: &str = "claude-code";

/// Plafond de lecture en tête d'un transcript (voir doc historique dans l'ancien `agents.rs`) :
/// sert au `cwd`, au modèle et à la consigne, jamais à lire le fichier entier.
const TRANSCRIPT_HEAD_CAP: usize = 1024 * 1024; // 1 MiB

/// Nombre maximal de caractères conservés dans le libellé de tâche du coordinateur.
const TASK_SNIPPET_MAX_CHARS: usize = 60;

// ========================= Lecture du sidecar meta.json =========================

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct AgentMeta {
    agent_type: String,
    description: Option<String>,
    parent_agent_id: Option<String>,
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
}

fn read_agent_meta(path: &Path) -> Option<AgentMeta> {
    let content = std::fs::read_to_string(path).ok()?;
    let raw: RawAgentMeta = serde_json::from_str(&content).ok()?;
    Some(AgentMeta {
        agent_type: raw.agent_type,
        description: raw.description,
        parent_agent_id: raw.parent_agent_id,
    })
}

// ============================ Horodatages ============================

fn epoch_of(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn mtime_of(path: &Path) -> Option<i64> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok().map(epoch_of)
}

fn created_of(path: &Path) -> Option<i64> {
    std::fs::metadata(path).and_then(|m| m.created()).ok().map(epoch_of)
}

// ===================== Lecture bornée en tête de transcript =====================

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
        if let Some(model) = v.get("message").and_then(|m| m.get("model")).and_then(|m| m.as_str()) {
            return Some(model.to_string());
        }
    }
    None
}

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

fn is_service_tag(text: &str) -> bool {
    text.trim_start().starts_with('<')
}

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

fn coordinator_task_label(path: Option<&Path>) -> String {
    path.and_then(|p| read_head_capped(p, TRANSCRIPT_HEAD_CAP))
        .and_then(|head| raw_user_snippet_from_head(&head))
        .map(|text| truncate_task(&text, TASK_SNIPPET_MAX_CHARS))
        .unwrap_or_else(|| "coordinateur".to_string())
}

// ===================== Découverte des fichiers de session =====================

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

fn group_sessions(projects_dir: &Path) -> HashMap<SessionKey, RawSession> {
    let mut sessions: HashMap<SessionKey, RawSession> = HashMap::new();
    for path in claude_transcript_files(projects_dir) {
        if let Some(agent_id) = subagent_id(&path) {
            let Some(subagents_dir) = path.parent() else { continue };
            let Some(sid_dir) = subagents_dir.parent() else { continue };
            let Some(escaped_dir) = sid_dir.parent() else { continue };
            let Some(sid) = sid_dir.file_name().and_then(|s| s.to_str()) else { continue };
            let key = (escaped_dir.to_path_buf(), sid.to_string());
            sessions.entry(key).or_default().subagents.push(RawSubagent {
                agent_id,
                transcript_path: path,
            });
        } else {
            let Some(escaped_dir) = path.parent() else { continue };
            let Some(sid) = path.file_stem().and_then(|s| s.to_str()) else { continue };
            let key = (escaped_dir.to_path_buf(), sid.to_string());
            sessions.entry(key).or_default().coordinator_transcript = Some(path);
        }
    }
    sessions
}

// ============================ Label de projet ============================

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

// ================= Production des fiches =================

/// Produit une fiche par agent (coordinateur + sous-agents) trouvé sous `projects_dir`, VIVANT
/// ou non (la péremption est une décision du cœur neutre, `tree::build_snapshot`, pas de cette
/// source). Fichier illisible (mtime absent) -> agent sauté, jamais de panique.
///
/// ⚠️ Parcourt le répertoire ENTIER (`F4` de ce-poste-vue-locale-lisibilite.md : ~1 s par passe sur
/// 660 transcripts) : le watcher ne l'appelle plus (`D3`), il n'existe plus que pour le micro-banc
/// `examples/tick_bench.rs` et les tests. Pour UNE session, voir [`cards_for_session`].
pub fn cards_from_dir(projects_dir: &Path) -> Vec<AgentCard> {
    let raw_sessions = group_sessions(projects_dir);
    let mut out = Vec::new();
    for (key, raw) in &raw_sessions {
        out.extend(cards_of_session(&key.0, &key.1, raw));
    }
    out
}

/// Nom de dossier de projet de Claude Code pour un `cwd` : tout caractère non alphanumérique
/// devient `-` (`/Users/x/work/a.b` -> `-Users-x-work-a-b`).
fn escaped_project_dir_name(cwd: &str) -> String {
    cwd.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect()
}

/// REPLI PAR SESSION (`D3`) : fiches d'UNE seule session (son transcript
/// `<projets>/<cwd échappé>/<sessionId>.jsonl` et ses sous-agents `<sessionId>/subagents/*.jsonl`),
/// sans jamais parcourir le répertoire des projets. Aucun fichier hors de cette session n'est
/// ouvert ; transcript absent -> liste vide.
pub fn cards_for_session(projects_dir: &Path, cwd: &str, session_id: &str) -> Vec<AgentCard> {
    if cwd.is_empty() || session_id.is_empty() || session_id.contains(['/', '\\']) {
        return Vec::new();
    }
    let escaped_dir = projects_dir.join(escaped_project_dir_name(cwd));
    let transcript = escaped_dir.join(format!("{session_id}.jsonl"));
    let mut raw = RawSession::default();
    if transcript.is_file() {
        raw.coordinator_transcript = Some(transcript);
    }
    if let Ok(entries) = std::fs::read_dir(escaped_dir.join(session_id).join("subagents")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if let Some(agent_id) = subagent_id(&path) {
                raw.subagents.push(RawSubagent { agent_id, transcript_path: path });
            }
        }
    }
    cards_of_session(&escaped_dir, session_id, &raw)
}

/// Fiches (coordinateur + sous-agents) d'une session déjà découverte.
fn cards_of_session(escaped_dir: &Path, sid: &str, raw: &RawSession) -> Vec<AgentCard> {
    let mut out = Vec::new();
    let project = project_label_for_session(escaped_dir, raw.coordinator_transcript.as_deref());
    let cwd = raw
        .coordinator_transcript
        .as_deref()
        .and_then(|p| first_cwd_in_transcript(p, TRANSCRIPT_HEAD_CAP));

    if let Some(path) = &raw.coordinator_transcript {
        if let Some(mtime) = mtime_of(path) {
            let started_at = created_of(path).unwrap_or(mtime);
            out.push(AgentCard {
                v: 1,
                runner: RUNNER_ID.to_string(),
                runner_version: None,
                model: read_head_capped(path, TRANSCRIPT_HEAD_CAP)
                    .as_deref()
                    .and_then(model_from_head),
                persona: None,
                agent_id: "main".to_string(),
                parent_id: None,
                session_id: sid.to_string(),
                project: Some(project.clone()),
                cwd: cwd.clone(),
                task: Some(coordinator_task_label(Some(path))),
                estimate: None,
                started_at,
                last_seen_at: mtime,
                state: Some("running".to_string()),
            });
        }
    }

    for sub in &raw.subagents {
        let path = &sub.transcript_path;
        let Some(mtime) = mtime_of(path) else { continue };
        let started_at = created_of(path).unwrap_or(mtime);
        let sidecar = path.with_extension("meta.json");
        let meta = read_agent_meta(&sidecar).unwrap_or_default();
        let head = read_head_capped(path, TRANSCRIPT_HEAD_CAP);
        let model = head.as_deref().and_then(model_from_head);
        let raw_consigne = head.as_deref().and_then(raw_user_snippet_from_head);
        let estimate_raw = raw_consigne
            .as_deref()
            .and_then(estimate::find_consigne_estimate_raw)
            .or_else(|| meta.description.as_deref().and_then(estimate::suffix_estimate_raw));
        let persona = if meta.agent_type.is_empty() {
            None
        } else {
            Some(meta.agent_type.clone())
        };
        let parent_id = meta.parent_agent_id.clone().or_else(|| Some("main".to_string()));

        out.push(AgentCard {
            v: 1,
            runner: RUNNER_ID.to_string(),
            runner_version: None,
            model,
            persona,
            agent_id: sub.agent_id.clone(),
            parent_id,
            session_id: sid.to_string(),
            project: Some(project.clone()),
            cwd: cwd.clone(),
            task: meta.description.clone(),
            estimate: estimate_raw,
            started_at,
            last_seen_at: mtime,
            state: Some("running".to_string()),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("iaka-am-ccf-{name}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Vérifie la correspondance RÉELLE du fichier de configuration embarqué pour ce runner —
    /// `labels.rs` reste neutre (ses propres tests n'utilisent que des identifiants d'exemple) ;
    /// cette vérification vit ici, seul endroit avec `publishers/claude-code/` où le nom d'un
    /// runner a le droit d'être écrit en dur (grille de neutralité de l'étape 12).
    #[test]
    fn le_fichier_de_labels_par_defaut_connait_ce_runner_et_ses_modeles() {
        let labels = crate::labels::Labels::default();
        assert_eq!(labels.runner_label(RUNNER_ID), "Claude Code");
        assert_eq!(labels.model_label("claude-opus-5-5-20260115"), "Opus");
        assert_eq!(labels.model_label("claude-sonnet-5"), "Sonnet");
    }

    fn write_transcript_line(path: &Path, cwd: &str, model: &str, user_text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let lines = format!(
            "{{\"cwd\":\"{cwd}\"}}\n{{\"type\":\"user\",\"message\":{{\"content\":\"{user_text}\"}}}}\n{{\"type\":\"assistant\",\"message\":{{\"model\":\"{model}\"}}}}\n"
        );
        fs::write(path, lines).unwrap();
    }

    #[test]
    fn produit_une_fiche_coordinateur_avec_agentid_main() {
        let dir = tmp_dir("coord");
        let escaped = dir.join("-w-proj");
        fs::create_dir_all(&escaped).unwrap();
        let sid_path = escaped.join("sid.jsonl");
        write_transcript_line(&sid_path, "/Users/x/work/proj", "claude-opus-5-5", "Bonjour");

        let cards = cards_from_dir(&dir);
        assert_eq!(cards.len(), 1);
        let card = &cards[0];
        assert_eq!(card.runner, "claude-code");
        assert_eq!(card.agent_id, "main");
        assert_eq!(card.parent_id, None);
        assert_eq!(card.persona, None);
        assert_eq!(card.project, Some("proj".to_string()));
        assert_eq!(card.model, Some("claude-opus-5-5".to_string()));
        assert_eq!(card.task, Some("Bonjour".to_string()));
    }

    #[test]
    fn produit_une_fiche_sous_agent_avec_parent_main_par_defaut() {
        let dir = tmp_dir("sub");
        let escaped = dir.join("-w-proj");
        let sid_dir = escaped.join("sid");
        fs::create_dir_all(sid_dir.join("subagents")).unwrap();
        write_transcript_line(&escaped.join("sid.jsonl"), "/Users/x/work/proj", "claude-opus", "Bonjour");
        let sub_path = sid_dir.join("subagents/agent-abc.jsonl");
        write_transcript_line(
            &sub_path,
            "/Users/x/work/proj",
            "claude-sonnet",
            "Durée estimée : ~10 min\\nfais le travail",
        );
        fs::write(
            sid_dir.join("subagents/agent-abc.meta.json"),
            r#"{"agentType":"gimli","description":"Fais le travail"}"#,
        )
        .unwrap();

        let cards = cards_from_dir(&dir);
        let sub_card = cards.iter().find(|c| c.agent_id == "abc").unwrap();
        assert_eq!(sub_card.parent_id, Some("main".to_string()));
        assert_eq!(sub_card.persona, Some("gimli".to_string()));
        assert_eq!(sub_card.task, Some("Fais le travail".to_string()));
        assert_eq!(sub_card.estimate, Some("~10 min".to_string()));
    }

    #[test]
    fn fichier_illisible_est_saute_sans_panique() {
        let dir = tmp_dir("absent");
        let cards = cards_from_dir(&dir.join("n-existe-pas"));
        assert!(cards.is_empty());
    }

    #[test]
    fn project_repli_sur_le_nom_de_dossier_echappe_si_cwd_absent() {
        let dir = tmp_dir("repli-projet");
        let escaped = dir.join("-w-hors-projet-connu");
        fs::create_dir_all(&escaped).unwrap();
        fs::write(escaped.join("sid.jsonl"), "{}\n").unwrap();
        let cards = cards_from_dir(&dir);
        assert_eq!(cards[0].project, Some("-w-hors-projet-connu".to_string()));
    }

    #[test]
    fn repli_par_session_ne_lit_que_la_session_demandee() {
        let dir = tmp_dir("repli-session");
        // La session demandée.
        let cwd = "/Users/x/work/mon.projet";
        let escaped = dir.join(escaped_project_dir_name(cwd));
        assert_eq!(escaped.file_name().unwrap(), "-Users-x-work-mon-projet");
        write_transcript_line(&escaped.join("sid-a.jsonl"), cwd, "claude-opus", "Ma tache");
        let sub = escaped.join("sid-a/subagents/agent-s1.jsonl");
        write_transcript_line(&sub, cwd, "claude-sonnet", "Consigne");
        fs::write(escaped.join("sid-a/subagents/agent-s1.meta.json"), r#"{"agentType":"gimli","description":"Sous tache"}"#).unwrap();
        // Une autre session, morte : ses transcripts existent mais ne doivent PAS être ouverts.
        let other = dir.join("-Users-x-work-autre");
        write_transcript_line(&other.join("sid-b.jsonl"), "/Users/x/work/autre", "claude-opus", "Morte");

        let cards = cards_for_session(&dir, cwd, "sid-a");
        let mut ids: Vec<&str> = cards.iter().map(|c| c.agent_id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec!["main", "s1"]);
        assert!(cards.iter().all(|c| c.session_id == "sid-a"));
        assert_eq!(cards.iter().find(|c| c.agent_id == "main").unwrap().task.as_deref(), Some("Ma tache"));

        // Transcript absent, cwd vide, identifiant piégé : liste vide, jamais un parcours.
        assert!(cards_for_session(&dir, cwd, "inconnue").is_empty());
        assert!(cards_for_session(&dir, "", "sid-a").is_empty());
        assert!(cards_for_session(&dir, cwd, "../sid-b").is_empty());
    }
}
