//! fleet_model — modèle de la flotte distante : module PUR (aucune E/S). Règle unique de l'état
//! d'une machine, **portée de `fleet/src/console/fleetModel.js`** et prouvée par les MÊMES vecteurs
//! (`contract-vectors/machine-state.json`, `tests/vectors.rs`).
//!
//! L'absence ne se déguise jamais : une machine sans code `online` est `Unknown`, pas `Offline` ;
//! une machine non en ligne n'a pas de sessions « tournantes » (`live = false`, affichées
//! « ? dernier connu »).

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use serde_json::Value;

use crate::card::AgentCard;
use crate::contract::{parse_topic, Topic};
use crate::tree::format_duration_secs;

/// Battement par défaut (s) quand `heartbeat_s` est absent ou invalide.
pub const DEFAULT_HEARTBEAT_S: f64 = 60.0;
/// Une machine est « muette » au-delà de `STALE_FACTOR` battements sans nouvelle.
pub const STALE_FACTOR: f64 = 3.0;

pub const ALERT_SESSIONS_UNREADABLE: &str = "sessions illisibles";
pub const ALERT_NO_CARDS_SOURCE: &str = "fiches d'agents : aucune source sur cette machine";

/// Un message code/valeur du contrat : `{"v":<scalaire|null>,"t":<epoch s>}`.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeMsg {
    pub v: Value,
    pub t: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Online,
    Mute,
    Offline,
    Unknown,
}

impl State {
    pub fn as_str(&self) -> &'static str {
        match self {
            State::Online => "online",
            State::Mute => "mute",
            State::Offline => "offline",
            State::Unknown => "unknown",
        }
    }
}

/// Sur quoi repose le « depuis » d'une machine hors ligne (contrat § 2, règle D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// `max(online.t, last_tick_at.v)` : dernier signe de vie connu (ou heure d'un arrêt propre).
    LastSeen,
    /// Repli sur `online.t` : on ne connaît que l'heure de la dernière connexion.
    Connected,
}

impl Basis {
    pub fn as_str(&self) -> &'static str {
        match self {
            Basis::LastSeen => "last_seen",
            Basis::Connected => "connected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineState {
    pub state: State,
    pub since: Option<i64>,
    /// `Some` seulement pour `Offline`.
    pub basis: Option<Basis>,
}

/// Payload hors du contrat `{v,t}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotInContract;

/// Décode un payload `{v,t}` strict : exactement deux clés, `v` scalaire ou null, `t` entier.
/// `Ok(None)` = payload vide (code effacé) ; `Err(NotInContract)` = hors contrat (ignoré par
/// l'appelant).
pub fn decode_code_msg(payload: &[u8]) -> Result<Option<CodeMsg>, NotInContract> {
    if payload.is_empty() {
        return Ok(None);
    }
    let o: Value = serde_json::from_slice(payload).map_err(|_| NotInContract)?;
    let obj = o.as_object().ok_or(NotInContract)?;
    if obj.len() != 2 || !obj.contains_key("v") || !obj.contains_key("t") {
        return Err(NotInContract);
    }
    let v = obj["v"].clone();
    if v.is_object() || v.is_array() {
        return Err(NotInContract);
    }
    let t = obj["t"].as_i64().ok_or(NotInContract)?;
    Ok(Some(CodeMsg { v, t }))
}

/// État d'une machine à partir de ses codes de `presence/` (règle D9 du MVP Fleet).
pub fn machine_state(meta: &HashMap<String, CodeMsg>, now_s: i64) -> MachineState {
    let Some(online) = meta.get("online") else {
        return MachineState { state: State::Unknown, since: None, basis: None };
    };
    match &online.v {
        // Le `t` d'un testament est l'instant de CONNEXION du démon, pas de sa mort (contrat § 2) :
        // « depuis » = dernier signe de vie, max(online.t, last_tick_at.v). Jamais online.t seul.
        Value::Bool(false) => match meta.get("last_tick_at").and_then(|m| m.v.as_f64()) {
            Some(tick) => MachineState {
                state: State::Offline,
                since: Some(online.t.max(tick as i64)),
                basis: Some(Basis::LastSeen),
            },
            None => MachineState { state: State::Offline, since: Some(online.t), basis: Some(Basis::Connected) },
        },
        Value::Bool(true) => {
            let hb = meta
                .get("heartbeat_s")
                .and_then(|m| m.v.as_f64())
                .filter(|h| *h > 0.0)
                .unwrap_or(DEFAULT_HEARTBEAT_S);
            let tick = meta.get("last_tick_at").and_then(|m| m.v.as_f64());
            match tick {
                Some(t) if (now_s as f64) - t <= STALE_FACTOR * hb => {
                    MachineState { state: State::Online, since: Some(online.t), basis: None }
                }
                Some(t) => MachineState { state: State::Mute, since: Some(t as i64), basis: None },
                None => MachineState { state: State::Mute, since: Some(online.t), basis: None },
            }
        }
        _ => MachineState { state: State::Unknown, since: None, basis: None },
    }
}

/// Alertes d'une machine, dans l'ordre fixe du contrat (R2 : « pas de hooks » ne se déguise
/// jamais en « aucun agent »).
pub fn machine_alerts(meta: &HashMap<String, CodeMsg>) -> Vec<String> {
    let mut alerts = Vec::new();
    if meta.get("sessions_dir_ok").map(|m| &m.v) == Some(&Value::Bool(false)) {
        alerts.push(ALERT_SESSIONS_UNREADABLE.to_string());
    }
    if meta.get("cards_source_ok").map(|m| &m.v) == Some(&Value::Bool(false)) {
        alerts.push(ALERT_NO_CARDS_SOURCE.to_string());
    }
    alerts
}

/// Contenu retenu d'un broker central (une entrée par topic), alimenté par la source distante.
#[derive(Debug, Default, Clone)]
pub struct RemoteStore {
    /// topic -> code/valeur (`presence/` et `sessions/`).
    pub codes: HashMap<String, CodeMsg>,
    /// topic -> fiche d'agent.
    pub cards: HashMap<String, AgentCard>,
}

/// Vue d'un agent distant. `task_masked` : la machine ne publie pas le texte (`task = null`) —
/// l'app écrit « tâche masquée », jamais une cellule vide.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    pub sid: String,
    pub agent_id: String,
    pub parent_id: Option<String>,
    pub persona: Option<String>,
    pub model: Option<String>,
    pub project: Option<String>,
    pub task: Option<String>,
    pub task_masked: bool,
    pub state: String,
    pub started_at: i64,
    pub last_seen_at: i64,
    pub live: bool,
    /// Libellés déjà formatés (D9 : la webview ne calcule rien).
    pub state_label: String,
    pub active_ago: String,
    pub started_ago: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub pid: u32,
    pub status: Option<String>,
    pub stuck_suspect: bool,
    pub status_since: Option<i64>,
    pub name: Option<String>,
    pub kind: Option<String>,
    pub entrypoint: Option<String>,
    pub claude_version: Option<String>,
    pub cwd: Option<String>,
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub rc: bool,
    pub sid: Option<String>,
    pub started_at: Option<i64>,
    pub last_known_t: i64,
    /// `false` dès que la machine n'est pas « en ligne » : jamais présentée comme tournante.
    pub live: bool,
    /// « busy » · « busy ? » (suspecte) · « ? (dernier connu : busy, il y a 2 h) » (machine non en ligne).
    pub status_label: String,
    pub agents: Vec<AgentView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineView {
    pub node: String,
    pub state: State,
    pub since: Option<i64>,
    /// Base du « depuis » hors ligne (`last_seen` | `connected`) ; `None` sinon.
    pub basis: Option<Basis>,
    pub os: Option<String>,
    pub os_release: Option<String>,
    pub hostname: Option<String>,
    pub agent_version: Option<String>,
    pub iakaframe_version: Option<String>,
    pub last_tick_at: Option<i64>,
    pub alerts: Vec<String>,
    /// « en ligne » · « muette (dernier battement il y a 8 min) » · « hors ligne (dernier signe de vie il y a 2 h) » · « inconnue ».
    pub state_label: String,
    pub cards_source_ok: Option<bool>,
    pub sessions: Vec<SessionView>,
    pub unattached_agents: Vec<AgentView>,
}

/// Libellé d'état d'une machine (mêmes mots que la CLI `iakafleet`).
pub fn machine_state_label(ms: &MachineState, now_s: i64) -> String {
    match (ms.state, ms.since) {
        (State::Online, _) => "en ligne".to_string(),
        (State::Mute, Some(t)) => {
            format!("muette (dernier battement il y a {})", format_duration_secs(now_s - t))
        }
        (State::Mute, None) => "muette".to_string(),
        (State::Offline, Some(t)) => {
            let d = format_duration_secs(now_s - t);
            match ms.basis {
                Some(Basis::LastSeen) => format!("hors ligne (dernier signe de vie il y a {d})"),
                _ => format!("hors ligne (heure d'arrêt inconnue ; connecté il y a {d})"),
            }
        }
        (State::Offline, None) => "hors ligne".to_string(),
        (State::Unknown, _) => "inconnue".to_string(),
    }
}

/// Statut d'une session : « ? (dernier connu…) » si la machine n'est pas en ligne, « busy ? » si
/// suspecte — jamais présentée comme tournante à tort.
fn session_status_label(s: &SessionView, now_s: i64) -> String {
    let st = s.status.as_deref().unwrap_or("inconnu");
    if !s.live {
        return format!("? (dernier connu : {st}, il y a {})", format_duration_secs(now_s - s.last_known_t));
    }
    if s.stuck_suspect && st == "busy" {
        return "busy ?".to_string();
    }
    st.to_string()
}

fn s(m: &HashMap<String, CodeMsg>, code: &str) -> Option<String> {
    m.get(code).and_then(|c| c.v.as_str()).map(str::to_string)
}
fn i(m: &HashMap<String, CodeMsg>, code: &str) -> Option<i64> {
    m.get(code).and_then(|c| c.v.as_i64())
}
fn b(m: &HashMap<String, CodeMsg>, code: &str) -> Option<bool> {
    m.get(code).and_then(|c| c.v.as_bool())
}

/// Construit la vue de la flotte. `own_node` : les messages de CE `<node>` sont ignorés (pas de
/// doublon avec la vue « Ce poste »). Machines triées par node ; agents rattachés à leur session
/// quand le code de session `sid` = `<sid>` de la fiche, sinon « non rattachés » (R3 : jamais
/// perdus).
pub fn build_fleet(store: &RemoteStore, own_node: Option<&str>, now_s: i64) -> Vec<MachineView> {
    #[derive(Default)]
    struct Slot {
        meta: HashMap<String, CodeMsg>,
        sessions: BTreeMap<u32, HashMap<String, CodeMsg>>,
        cards: Vec<(String, AgentCard)>,
    }
    let mut nodes: BTreeMap<String, Slot> = BTreeMap::new();
    for (topic, msg) in &store.codes {
        match parse_topic(topic) {
            Some(Topic::Presence { node, code }) if Some(node.as_str()) != own_node => {
                nodes.entry(node).or_default().meta.insert(code, msg.clone());
            }
            Some(Topic::Session { node, pid, code }) if Some(node.as_str()) != own_node => {
                nodes.entry(node).or_default().sessions.entry(pid).or_default().insert(code, msg.clone());
            }
            _ => {}
        }
    }
    for (topic, card) in &store.cards {
        if let Some(Topic::Agent { node, sid, .. }) = parse_topic(topic) {
            if Some(node.as_str()) != own_node {
                nodes.entry(node).or_default().cards.push((sid, card.clone()));
            }
        }
    }

    nodes
        .into_iter()
        .map(|(node, mut slot)| {
            let ms = machine_state(&slot.meta, now_s);
            let live = ms.state == State::Online;
            let mut sessions: Vec<SessionView> = slot
                .sessions
                .iter()
                .map(|(pid, codes)| SessionView {
                    pid: *pid,
                    status: s(codes, "status"),
                    stuck_suspect: b(codes, "stuck_suspect") == Some(true),
                    status_since: i(codes, "status_since"),
                    name: s(codes, "name"),
                    kind: s(codes, "kind"),
                    entrypoint: s(codes, "entrypoint"),
                    claude_version: s(codes, "claude_version"),
                    cwd: s(codes, "cwd"),
                    repo: s(codes, "repo"),
                    branch: s(codes, "branch"),
                    rc: b(codes, "rc") == Some(true),
                    sid: s(codes, "sid").filter(|x| !x.is_empty()),
                    started_at: i(codes, "started_at"),
                    last_known_t: codes.values().map(|c| c.t).max().unwrap_or(0),
                    live,
                    status_label: String::new(),
                    agents: Vec::new(),
                })
                .collect();
            for sess in &mut sessions {
                sess.status_label = session_status_label(sess, now_s);
            }
            slot.cards.sort_by(|a, b| {
                (a.0.as_str(), a.1.agent_id.as_str()).cmp(&(b.0.as_str(), b.1.agent_id.as_str()))
            });
            let mut unattached = Vec::new();
            for (sid, card) in slot.cards {
                let view = AgentView {
                    sid: sid.clone(),
                    agent_id: card.agent_id.clone(),
                    parent_id: card.parent_id.clone(),
                    persona: card.persona.clone(),
                    model: card.model.clone(),
                    project: card.project.clone(),
                    task_masked: card.task.is_none(),
                    task: card.task.clone(),
                    state: card.state_or_default().to_string(),
                    started_at: card.started_at,
                    last_seen_at: card.last_seen_at,
                    live,
                    state_label: if live {
                        card.state_or_default().to_string()
                    } else {
                        format!("? (dernier connu : {})", card.state_or_default())
                    },
                    active_ago: format_duration_secs(now_s - card.last_seen_at),
                    started_ago: format_duration_secs(now_s - card.started_at),
                };
                match sessions.iter_mut().find(|x| x.sid.as_deref() == Some(sid.as_str())) {
                    Some(owner) => owner.agents.push(view),
                    None => unattached.push(view),
                }
            }
            MachineView {
                node,
                state: ms.state,
                since: ms.since,
                basis: ms.basis,
                os: s(&slot.meta, "os"),
                os_release: s(&slot.meta, "os_release"),
                hostname: s(&slot.meta, "hostname"),
                agent_version: s(&slot.meta, "agent_version"),
                iakaframe_version: s(&slot.meta, "iakaframe_version"),
                last_tick_at: i(&slot.meta, "last_tick_at"),
                alerts: machine_alerts(&slot.meta),
                state_label: machine_state_label(&ms, now_s),
                cards_source_ok: b(&slot.meta, "cards_source_ok"),
                sessions,
                unattached_agents: unattached,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn put(store: &mut RemoteStore, topic: &str, v: Value, t: i64) {
        store.codes.insert(topic.to_string(), CodeMsg { v, t });
    }

    #[test]
    fn libelle_hors_ligne_dernier_signe_de_vie_apres_testament() {
        let now = 10_000;
        let mut st = RemoteStore::default();
        // testament : t = démarrage du démon (très ancien) ; dernier battement il y a 50 s
        put(&mut st, "iaka/pc/out/presence/online/current", json!(false), 1_000);
        put(&mut st, "iaka/pc/out/presence/last_tick_at/current", json!(now - 50), now - 50);
        let pc = &build_fleet(&st, None, now)[0];
        assert_eq!(pc.since, Some(now - 50));
        assert_eq!(pc.basis, Some(Basis::LastSeen));
        assert_eq!(pc.state_label, "hors ligne (dernier signe de vie il y a 50 s)");
    }

    #[test]
    fn libelles_machine_et_session_non_en_ligne_dernier_connu() {
        let now = 10_000;
        let mut st = RemoteStore::default();
        put(&mut st, "iaka/pc/out/presence/online/current", json!(false), now - 7200);
        put(&mut st, "iaka/pc/out/sessions/9/status/current", json!("busy"), now - 7300);
        put(&mut st, "iaka/mac2/out/presence/online/current", json!(true), now - 9000);
        put(&mut st, "iaka/mac2/out/presence/last_tick_at/current", json!(now - 500), now - 500);
        put(&mut st, "iaka/mac2/out/sessions/1/status/current", json!("idle"), now - 500);
        let fleet = build_fleet(&st, None, now);
        let mac2 = fleet.iter().find(|m| m.node == "mac2").unwrap();
        let pc = fleet.iter().find(|m| m.node == "pc").unwrap();
        assert_eq!(pc.state_label, "hors ligne (heure d'arrêt inconnue ; connecté il y a 2 h)");
        assert_eq!(pc.basis, Some(Basis::Connected));
        assert_eq!(pc.sessions[0].status_label, "? (dernier connu : busy, il y a 2 h 1 min)");
        assert!(!pc.sessions[0].live);
        assert_eq!(mac2.state, State::Mute);
        assert!(mac2.state_label.starts_with("muette (dernier battement il y a 8 min"));
        assert_eq!(mac2.sessions[0].status_label, "? (dernier connu : idle, il y a 8 min)");
    }

    #[test]
    fn session_suspecte_busy_point_d_interrogation_et_own_node_exclu() {
        let now = 10_000;
        let mut st = RemoteStore::default();
        for n in ["a", "own"] {
            put(&mut st, &format!("iaka/{n}/out/presence/online/current"), json!(true), now - 5);
            put(&mut st, &format!("iaka/{n}/out/presence/last_tick_at/current"), json!(now - 5), now - 5);
            put(&mut st, &format!("iaka/{n}/out/sessions/1/status/current"), json!("busy"), now - 5);
            put(&mut st, &format!("iaka/{n}/out/sessions/1/stuck_suspect/current"), json!(true), now - 5);
        }
        let fleet = build_fleet(&st, Some("own"), now);
        assert_eq!(fleet.len(), 1);
        assert_eq!(fleet[0].sessions[0].status_label, "busy ?");
    }

    #[test]
    fn decode_code_msg_strict() {
        assert_eq!(decode_code_msg(b""), Ok(None));
        assert!(matches!(decode_code_msg(br#"{"v":1,"t":2}"#), Ok(Some(_))));
        for bad in [&b"nope"[..], b"[]", br#"{"v":1}"#, br#"{"v":{},"t":1}"#, br#"{"v":1,"t":1,"x":2}"#, br#"{"v":1,"t":1.5}"#] {
            assert!(decode_code_msg(bad).is_err(), "{}", String::from_utf8_lossy(bad));
        }
    }
}
