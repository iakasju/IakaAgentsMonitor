//! Vecteurs partages `contract-vectors/` (specs/contrat-bus-iaka.md § 6) : la part de l'app Rust.
//! Les trois pieces (demon Node, publieur, app) passent les MEMES cas ; c'est ce qui rend la
//! convention unique verifiable. Hors couverture DECLAREE cote Rust : le hachage du `<sid>`
//! (l'app compare des sid deja haches par le demon, elle n'en calcule aucun).

use std::collections::HashMap;

use iakaagentsmonitor_lib::contract::{
    agent_topic, agents_filter, normalize_node, parse_topic, Topic, ALL_OUT,
};
use iakaagentsmonitor_lib::card::AgentCard;
use iakaagentsmonitor_lib::fleet_model::{machine_alerts, machine_state, CodeMsg, MachineState};
use iakaagentsmonitor_lib::labels::Labels;
use iakaagentsmonitor_lib::roster::Roster;
use iakaagentsmonitor_lib::sources::local_sessions::{
    build_local_snapshot, normalize_session, LocalInput, LocalSession,
};
use serde_json::Value;

fn load(name: &str) -> Value {
    let path = format!("{}/../contract-vectors/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}")))
        .unwrap()
}

#[test]
fn vecteurs_normalisation_node() {
    let v = load("node-normalization.json");
    let ok = v["ok"].as_array().unwrap();
    let refused = v["refused"].as_array().unwrap();
    assert!(!ok.is_empty() && !refused.is_empty());
    for c in ok {
        let input = c["input"].as_str().unwrap();
        assert_eq!(normalize_node(input).as_deref(), Ok(c["expected"].as_str().unwrap()), "{input:?}");
    }
    for bad in refused {
        let input = bad.as_str().unwrap();
        assert!(normalize_node(input).is_err(), "{input:?} devrait etre refuse");
    }
}

#[test]
fn vecteurs_topics_construction() {
    let b = &load("topics.json")["build"];
    let agents = b["agent"].as_array().unwrap();
    assert!(!agents.is_empty());
    for c in agents {
        let got = agent_topic(
            c["node"].as_str().unwrap(),
            c["runner"].as_str().unwrap(),
            c["sid"].as_str().unwrap(),
            c["agentId"].as_str().unwrap(),
        );
        assert_eq!(got, c["expected"].as_str().unwrap());
    }
    for c in b["agentsFilter"].as_array().unwrap() {
        assert_eq!(agents_filter(c["node"].as_str().unwrap()), c["expected"].as_str().unwrap());
    }
    assert_eq!(ALL_OUT, b["allOut"].as_str().unwrap());
}

fn topic_json(t: Topic) -> Value {
    match t {
        Topic::Presence { node, code } => serde_json::json!({"node": node, "kind": "presence", "code": code}),
        Topic::Session { node, pid, code } => serde_json::json!({"node": node, "kind": "session", "pid": pid, "code": code}),
        Topic::Agent { node, runner, sid, agent_id } => serde_json::json!({"node": node, "kind": "agent", "runner": runner, "sid": sid, "agentId": agent_id}),
    }
}

#[test]
fn vecteurs_topics_decomposition() {
    let cases = load("topics.json")["parse"].as_array().unwrap().clone();
    assert!(!cases.is_empty());
    for c in cases {
        let topic = c["topic"].as_str().unwrap();
        let got = parse_topic(topic).map(topic_json).unwrap_or(Value::Null);
        assert_eq!(got, c["expected"], "{topic}");
    }
}

#[test]
fn vecteurs_etat_de_machine_et_alertes() {
    let cases = load("machine-state.json")["cases"].as_array().unwrap().clone();
    assert!(!cases.is_empty());
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let now = c["nowS"].as_i64().unwrap();
        let meta: HashMap<String, CodeMsg> = c["meta"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, m)| (k.clone(), CodeMsg { v: m["v"].clone(), t: m["t"].as_i64().unwrap() }))
            .collect();
        let exp = &c["expected"];
        let got: MachineState = machine_state(&meta, now);
        assert_eq!(got.state.as_str(), exp["state"].as_str().unwrap(), "{name}");
        assert_eq!(got.since, exp["since"].as_i64(), "{name}");
        assert_eq!(got.basis.map(|b| b.as_str()), exp["basis"].as_str(), "{name}");
        let alerts: Vec<String> = exp["alerts"].as_array().unwrap().iter().map(|a| a.as_str().unwrap().to_string()).collect();
        assert_eq!(machine_alerts(&meta), alerts, "{name}");
    }
}

// ---- local-sessions.json : « Ce poste » (ce-poste-vue-locale-lisibilite.md D2) -------------------

#[test]
fn vecteurs_local_normalisation_de_session() {
    let v = load("local-sessions.json");
    let cases = v["normalize"].as_array().unwrap();
    assert!(cases.len() >= 6);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let pid = c["fallbackPid"].as_u64().unwrap() as u32;
        // La vivacité est INJECTÉE par le vecteur : pid mort => la session est absente.
        if !c["alive"].as_bool().unwrap() {
            assert!(c["expected"].is_null(), "{name}");
            continue;
        }
        let s = normalize_session(&c["file"], pid);
        let e = &c["expected"];
        assert_eq!(s.pid as u64, e["pid"].as_u64().unwrap(), "{name} pid");
        assert_eq!(s.session_id.as_deref(), e["sessionId"].as_str(), "{name} sessionId");
        assert_eq!(s.status.as_deref(), e["status"].as_str(), "{name} status");
        assert_eq!(s.status_since, e["statusSince"].as_i64(), "{name} statusSince");
        assert_eq!(s.stuck_suspect, e["stuckSuspect"].as_bool().unwrap(), "{name} stuckSuspect");
        assert_eq!(s.name.as_deref(), e["name"].as_str(), "{name} name");
        let project = s.cwd.as_deref().and_then(|c| {
            c.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).find(|p| !p.is_empty()).map(str::to_string)
        });
        assert_eq!(project.as_deref(), e["project"].as_str(), "{name} project");
    }
}

fn cards_of(v: &Value) -> Vec<AgentCard> {
    v.as_array().unwrap().iter().map(|c| serde_json::from_value(c.clone()).unwrap()).collect()
}

#[test]
fn vecteurs_local_assemblage_regle_d2() {
    let v = load("local-sessions.json");
    let now = v["nowS"].as_i64().unwrap();
    let liveness = v["livenessS"].as_i64().unwrap();
    let cases = v["assemble"].as_array().unwrap();
    assert!(cases.len() >= 8);
    let labels = Labels::default();
    let roster = Roster::new();
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let sessions: Vec<LocalSession> = c["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["alive"].as_bool().unwrap())
            .map(|s| {
                let pid = s["file"]["pid"].as_u64().unwrap() as u32;
                normalize_session(&s["file"], pid)
            })
            .collect();
        let fb_map = c["transcriptFallback"].as_object().unwrap();
        let mut fallback = |s: &LocalSession| -> Vec<AgentCard> {
            fb_map.get(&s.pid.to_string()).map(cards_of).unwrap_or_default()
        };
        let out = build_local_snapshot(
            LocalInput { sessions: &sessions, disk_cards: cards_of(&c["diskCards"]), bus_cards: cards_of(&c["busCards"]) },
            &mut fallback,
            now,
            liveness,
            &labels,
            &roster,
        );
        let e = &c["expected"];
        let exp_sessions = e["sessions"].as_array().unwrap();
        assert_eq!(out.snapshot.sessions.len(), exp_sessions.len(), "{name} : nombre de sessions affichees");
        for (got, exp) in out.snapshot.sessions.iter().zip(exp_sessions) {
            assert_eq!(got.project, exp["project"].as_str().unwrap(), "{name} projet");
            assert_eq!(got.coordinator.task, exp["coordinator"]["task"].as_str().unwrap(), "{name} tache du coordinateur");
            assert_eq!(got.coordinator.state, exp["coordinator"]["state"].as_str().unwrap(), "{name} etat du coordinateur");
            let kids: Vec<&str> = got.coordinator.children.iter().map(|n| n.task.as_str()).collect();
            let exp_kids: Vec<&str> = exp["children"].as_array().unwrap().iter().map(|k| k.as_str().unwrap()).collect();
            assert_eq!(kids, exp_kids, "{name} sous-agents");
        }
        let unattached: Vec<&str> = out.snapshot.unattached.iter().map(|n| n.task.as_str()).collect();
        let exp_un: Vec<&str> = e["unattached"].as_array().unwrap().iter().map(|k| k.as_str().unwrap()).collect();
        assert_eq!(unattached, exp_un, "{name} non rattaches");
        let mut fb = out.fallback_for.clone();
        fb.sort_unstable();
        let exp_fb: Vec<u32> = e["fallbackFor"].as_array().unwrap().iter().map(|p| p.as_u64().unwrap() as u32).collect();
        assert_eq!(fb, exp_fb, "{name} repli par session (aucun autre fichier ouvert)");
        let _ = e["otherRunners"].as_array().unwrap(); // verifie via le nombre de sessions affichees (runner tiers = sa propre session)
    }
}
