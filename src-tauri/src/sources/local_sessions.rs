//! sources::local_sessions — « Ce poste » lu SUR LE DISQUE (ce-poste-vue-locale-lisibilite.md D1/D2).
//!
//! SQUELETTE (étape 1 : vecteurs d'abord, vus rouges) — le corps arrive à l'étape 2.

use serde_json::Value;

use crate::card::AgentCard;
use crate::labels::Labels;
use crate::roster::Roster;
use crate::tree::AgentsSnapshot;

/// Session Claude Code vivante, normalisée depuis `~/.claude/sessions/<pid>.json`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LocalSession {
    pub pid: u32,
    pub session_id: Option<String>,
    pub status: Option<String>,
    pub status_since: Option<i64>,
    pub updated_at: Option<i64>,
    pub stuck_suspect: bool,
    pub name: Option<String>,
    pub cwd: Option<String>,
    pub started_at: Option<i64>,
}

/// Normalise un fichier de session brut (champs absents => `None`).
pub fn normalize_session(_raw: &Value, fallback_pid: u32) -> LocalSession {
    LocalSession { pid: fallback_pid, ..Default::default() }
}

/// Entrée de l'assemblage : sessions vivantes + fiches du disque + fiches du bus local.
pub struct LocalInput<'a> {
    pub sessions: &'a [LocalSession],
    pub disk_cards: Vec<AgentCard>,
    pub bus_cards: Vec<AgentCard>,
}

/// Résultat : instantané affiché + pids pour lesquels le repli par transcripts a été appelé.
pub struct LocalOutcome {
    pub snapshot: AgentsSnapshot,
    pub fallback_for: Vec<u32>,
}

/// Règle `D2` de bout en bout, jusqu'à l'instantané affiché.
pub fn build_local_snapshot(
    _input: LocalInput<'_>,
    _fallback: &mut dyn FnMut(&LocalSession) -> Vec<AgentCard>,
    _now: i64,
    _liveness_secs: i64,
    _labels: &Labels,
    _roster: &Roster,
) -> LocalOutcome {
    LocalOutcome { snapshot: AgentsSnapshot::default(), fallback_for: Vec::new() }
}
