//! local_view — en-tête de la vue « Ce poste » (ce-poste-vue-locale-lisibilite.md D4) : titre,
//! état du démon et phrase d'état. Module PUR : tout est composé ICI, en Rust (D9 : la webview ne
//! calcule rien, elle peint), à partir de données déjà calculées — l'instantané des agents, le
//! `<node>` et le libellé de l'état du démon (MÊME fonction que pour les autres machines,
//! `fleet_model::machine_state_label`). Aucune donnée nouvelle n'est introduite pour la phrase.

use serde::Serialize;

use crate::tree::{AgentNode, AgentsSnapshot};

/// Titre et libellés de « Ce poste », déjà composés.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalHeader {
    /// `Ce poste — <node> · <N> session(s) · <M> agent(s)` (ou `(nom de machine indéterminé)`).
    pub title: String,
    /// `démon : <libellé>` — mêmes mots que la vue distante, ou `démon : non observé (…)`.
    pub daemon: String,
    /// Phrase d'état (reprise de la direction C) : « 2 agents travaillent · 1 session en cours
    /// sur 4 ».
    pub phrase: String,
    /// Avertissement quand la source locale est dégradée (répertoire de sessions illisible…) :
    /// l'absence ne se déguise jamais en « aucune session ».
    pub notice: Option<String>,
    pub sessions: u32,
    pub agents: u32,
    pub active_agents: u32,
    pub busy_sessions: u32,
}

fn active_in(node: &AgentNode) -> u32 {
    if node.overflow.is_some() {
        return 0;
    }
    u32::from(node.state == "running") + node.children.iter().map(active_in).sum::<u32>()
}

/// Nombre d'agents en activité (`running`) parmi ceux affichés, non rattachés compris.
pub fn active_agents(snap: &AgentsSnapshot) -> u32 {
    snap.sessions.iter().map(|s| active_in(&s.coordinator)).sum::<u32>()
        + snap.unattached.iter().map(active_in).sum::<u32>()
}

fn plural(n: u32, one: &str, many: &str) -> String {
    format!("{n} {}", if n > 1 { many } else { one })
}

/// Phrase d'état : agents actifs, sessions en cours sur le total. Le démon n'y figure pas (il a
/// sa propre ligne dans le titre) mais reste dans la donnée `daemon`.
pub fn state_phrase(active: u32, busy_sessions: u32, sessions: u32) -> String {
    let lead = match active {
        0 => "Aucun agent en cours".to_string(),
        1 => "1 agent travaille".to_string(),
        n => format!("{n} agents travaillent"),
    };
    if sessions == 0 {
        return format!("{lead} · aucune session ouverte");
    }
    format!("{lead} · {} sur {sessions}", plural(busy_sessions, "session en cours", "sessions en cours"))
}

/// Compose l'en-tête. `daemon_label` = libellé d'état du démon SANS le préfixe « démon : ».
pub fn build_header(node: Option<&str>, snap: &AgentsSnapshot, daemon_label: &str, notice: Option<String>) -> LocalHeader {
    let sessions = snap.sessions.len() as u32 + snap.overflow_sessions.unwrap_or(0);
    let busy_sessions = snap.sessions.iter().filter(|s| s.coordinator.state == "running").count() as u32;
    let active = active_agents(snap);
    let who = node.map(str::to_string).unwrap_or_else(|| "(nom de machine indéterminé)".to_string());
    LocalHeader {
        title: format!("Ce poste — {who} · {sessions} session(s) · {} agent(s)", snap.count),
        daemon: format!("démon : {daemon_label}"),
        phrase: state_phrase(active, busy_sessions, sessions),
        notice,
        sessions,
        agents: snap.count,
        active_agents: active,
        busy_sessions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{AgentNode, SessionNode, Sprite};

    fn node(state: &str, children: Vec<AgentNode>) -> AgentNode {
        AgentNode {
            sprite: Sprite { letter: 'M', bg: String::new(), fg: String::new() },
            persona: "Main".into(),
            task: String::new(),
            active_ago: String::new(),
            active_secs: 0,
            started_ago: String::new(),
            started_secs: None,
            model: String::new(),
            runner: String::new(),
            state: state.into(),
            estimate: None,
            tooltip: String::new(),
            children,
            overflow: None,
        }
    }

    fn snap(sessions: Vec<AgentNode>, overflow: Option<u32>) -> AgentsSnapshot {
        let count = sessions.iter().map(|c| 1 + c.children.len() as u32).sum();
        AgentsSnapshot {
            count,
            sessions: sessions
                .into_iter()
                .map(|coordinator| SessionNode { project: "p".into(), coordinator, group_color: 0 })
                .collect(),
            overflow_sessions: overflow,
            unattached: vec![],
            header: None,
        }
    }

    #[test]
    fn titre_exact_node_sessions_agents() {
        let s = snap(vec![node("idle", vec![node("running", vec![])]), node("running", vec![])], None);
        let h = build_header(Some("mbp"), &s, "en ligne", None);
        assert_eq!(h.title, "Ce poste — mbp · 2 session(s) · 3 agent(s)");
        assert_eq!(h.daemon, "démon : en ligne");
        assert_eq!((h.sessions, h.agents, h.active_agents, h.busy_sessions), (2, 3, 2, 1));
        assert_eq!(h.phrase, "2 agents travaillent · 1 session en cours sur 2");
    }

    #[test]
    fn node_indetermine_le_dit() {
        let h = build_header(None, &snap(vec![], None), "non observé (pas de broker central)", None);
        assert_eq!(h.title, "Ce poste — (nom de machine indéterminé) · 0 session(s) · 0 agent(s)");
        assert_eq!(h.daemon, "démon : non observé (pas de broker central)");
        assert_eq!(h.phrase, "Aucun agent en cours · aucune session ouverte");
    }

    #[test]
    fn sessions_au_dela_de_la_troncature_sont_comptees() {
        let h = build_header(Some("m"), &snap(vec![node("idle", vec![])], Some(3)), "en ligne", None);
        assert_eq!(h.sessions, 4);
        assert_eq!(h.phrase, "Aucun agent en cours · 0 session en cours sur 4");
    }

    #[test]
    fn un_seul_agent_actif_singulier() {
        assert_eq!(state_phrase(1, 1, 1), "1 agent travaille · 1 session en cours sur 1");
        assert_eq!(state_phrase(3, 2, 5), "3 agents travaillent · 2 sessions en cours sur 5");
    }
}
