//! card — la fiche d'agent normalisée du contrat MQTT v1
//! (specs/instructions/agnostique-mqtt.md § « Contrat sur le fil »).
//!
//! Module NEUTRE : aucun nom de runner ni de fournisseur de modèle. Une fiche est reçue soit
//! depuis le bus (source `sources::bus`), soit produite par la source fichiers, qui joue le rôle
//! de l'adaptateur d'un runner historique (voir `sources/`). Les deux sources rendent le MÊME
//! type ; le cœur (`tree`) ne connaît qu'elles.

use serde::{Deserialize, Serialize};

/// Taille maximale d'un payload de fiche (contrat § « Fiche »).
pub const MAX_PAYLOAD_BYTES: usize = 4096;

/// Version majeure du contrat acceptée par cette app. Toute autre valeur -> fiche ignorée.
pub const SUPPORTED_VERSION: u32 = 1;

/// Longueur maximale d'un segment de topic (`<runner>`, `<sessionId>`, `<agentId>`).
const MAX_SEGMENT_LEN: usize = 128;

/// Une fiche d'agent normalisée (contrat v1). Champs en camelCase sur le fil (`#[serde]`).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    pub v: u32,
    pub runner: String,
    #[serde(default)]
    pub runner_version: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub persona: Option<String>,
    pub agent_id: String,
    pub parent_id: Option<String>,
    pub session_id: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub estimate: Option<String>,
    pub started_at: i64,
    pub last_seen_at: i64,
    #[serde(default)]
    pub state: Option<String>,
}

impl AgentCard {
    /// `state` normalisé : valeur inconnue ou absente -> `"running"` (contrat § Fiche, champ
    /// `state`).
    pub fn state_or_default(&self) -> &str {
        match self.state.as_deref() {
            Some("running") | Some("idle") => self.state.as_deref().unwrap(),
            _ => "running",
        }
    }
}

/// Raison de rejet d'une fiche — jamais un plantage, toujours une valeur ignorée côté appelant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardError {
    TooLarge,
    InvalidJson,
    UnsupportedVersion,
    MissingField(&'static str),
    InvalidTopic,
    TopicMismatch,
}

/// Un segment de topic valide : `[A-Za-z0-9._-]{1,128}`, jamais `+`, `#` ni `/`.
fn is_valid_segment(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_SEGMENT_LEN
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Segments `(runner, sessionId, agentId)` extraits d'un topic
/// `iaka/agents/out/<runner>/<sessionId>/<agentId>`. `None` si la forme ne correspond pas.
fn parse_topic(topic: &str) -> Option<(&str, &str, &str)> {
    let rest = topic.strip_prefix("iaka/agents/out/")?;
    let mut parts = rest.split('/');
    let runner = parts.next()?;
    let session_id = parts.next()?;
    let agent_id = parts.next()?;
    if parts.next().is_some() {
        return None; // segment supplémentaire -> topic incohérent.
    }
    if !is_valid_segment(runner) || !is_valid_segment(session_id) || !is_valid_segment(agent_id) {
        return None;
    }
    Some((runner, session_id, agent_id))
}

/// Parse une fiche reçue sur `topic` avec le `payload` donné.
///
/// - Payload vide (0 octet) -> `Ok(None)` : c'est la suppression (contrat § « Fin, plantage,
///   expiration »), jamais une erreur.
/// - Payload > [`MAX_PAYLOAD_BYTES`] -> `Err(TooLarge)`.
/// - JSON invalide -> `Err(InvalidJson)`.
/// - `v != 1` -> `Err(UnsupportedVersion)`.
/// - Incohérence topic <-> (`runner`, `sessionId`, `agentId`) -> `Err(TopicMismatch)`.
///
/// Jamais de panique : toute fiche rejetée doit être ignorée par l'appelant, jamais bloquante.
pub fn parse_card(topic: &str, payload: &[u8]) -> Result<Option<AgentCard>, CardError> {
    if payload.is_empty() {
        return Ok(None);
    }
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(CardError::TooLarge);
    }
    let (t_runner, t_session, t_agent) = parse_topic(topic).ok_or(CardError::InvalidTopic)?;

    let card: AgentCard =
        serde_json::from_slice(payload).map_err(|_| CardError::InvalidJson)?;

    if card.v != SUPPORTED_VERSION {
        return Err(CardError::UnsupportedVersion);
    }
    if card.runner.is_empty() {
        return Err(CardError::MissingField("runner"));
    }
    if card.agent_id.is_empty() {
        return Err(CardError::MissingField("agentId"));
    }
    if card.session_id.is_empty() {
        return Err(CardError::MissingField("sessionId"));
    }
    if card.runner != t_runner || card.session_id != t_session || card.agent_id != t_agent {
        return Err(CardError::TopicMismatch);
    }

    Ok(Some(card))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOPIC: &str = "iaka/agents/out/essai/sess-1/main";

    fn valid_payload() -> String {
        serde_json::json!({
            "v": 1,
            "runner": "essai",
            "agentId": "main",
            "parentId": null,
            "sessionId": "sess-1",
            "startedAt": 1_790_000_000,
            "lastSeenAt": 1_790_000_010,
            "state": "running"
        })
        .to_string()
    }

    #[test]
    fn payload_vide_est_une_suppression() {
        assert_eq!(parse_card(TOPIC, b""), Ok(None));
    }

    #[test]
    fn fiche_valide_est_acceptee() {
        let payload = valid_payload();
        let card = parse_card(TOPIC, payload.as_bytes()).unwrap().unwrap();
        assert_eq!(card.runner, "essai");
        assert_eq!(card.agent_id, "main");
        assert_eq!(card.session_id, "sess-1");
        assert_eq!(card.state_or_default(), "running");
    }

    #[test]
    fn payload_trop_gros_est_rejete() {
        let big = "x".repeat(MAX_PAYLOAD_BYTES + 1);
        assert_eq!(parse_card(TOPIC, big.as_bytes()), Err(CardError::TooLarge));
    }

    #[test]
    fn json_invalide_est_rejete() {
        assert_eq!(
            parse_card(TOPIC, b"{ pas du json"),
            Err(CardError::InvalidJson)
        );
    }

    #[test]
    fn version_non_1_est_rejetee() {
        let mut v: serde_json::Value = serde_json::from_str(&valid_payload()).unwrap();
        v["v"] = serde_json::json!(2);
        assert_eq!(
            parse_card(TOPIC, v.to_string().as_bytes()),
            Err(CardError::UnsupportedVersion)
        );
    }

    #[test]
    fn topic_incoherent_avec_la_fiche_est_rejete() {
        let payload = valid_payload(); // sessionId = "sess-1"
        let other_topic = "iaka/agents/out/essai/sess-2/main";
        assert_eq!(
            parse_card(other_topic, payload.as_bytes()),
            Err(CardError::TopicMismatch)
        );
    }

    #[test]
    fn topic_mal_forme_est_rejete() {
        assert_eq!(
            parse_card("iaka/agents/out/essai/sess-1", valid_payload().as_bytes()),
            Err(CardError::InvalidTopic)
        );
        assert_eq!(
            parse_card("autre/arbre/x/y/z", valid_payload().as_bytes()),
            Err(CardError::InvalidTopic)
        );
    }

    #[test]
    fn segment_avec_caractere_interdit_est_rejete() {
        assert_eq!(
            parse_card("iaka/agents/out/cla+ude/sess-1/main", valid_payload().as_bytes()),
            Err(CardError::InvalidTopic)
        );
    }

    #[test]
    fn champs_inconnus_sont_ignores() {
        let mut v: serde_json::Value = serde_json::from_str(&valid_payload()).unwrap();
        v["champInconnu"] = serde_json::json!("valeur");
        let card = parse_card(TOPIC, v.to_string().as_bytes()).unwrap().unwrap();
        assert_eq!(card.runner, "essai");
    }

    #[test]
    fn state_inconnu_est_traite_comme_running() {
        let mut v: serde_json::Value = serde_json::from_str(&valid_payload()).unwrap();
        v["state"] = serde_json::json!("va-savoir");
        let card = parse_card(TOPIC, v.to_string().as_bytes()).unwrap().unwrap();
        assert_eq!(card.state_or_default(), "running");
    }

    #[test]
    fn parent_id_main_pour_un_enfant_direct() {
        let mut v: serde_json::Value = serde_json::from_str(&valid_payload()).unwrap();
        v["parentId"] = serde_json::json!("main");
        v["agentId"] = serde_json::json!("child-1");
        let topic = "iaka/agents/out/essai/sess-1/child-1";
        let card = parse_card(topic, v.to_string().as_bytes()).unwrap().unwrap();
        assert_eq!(card.parent_id, Some("main".to_string()));
    }
}
