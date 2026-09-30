//! Vecteurs partages `contract-vectors/` (specs/contrat-bus-iaka.md § 6) : la part de l'app Rust.
//! Les trois pieces (demon Node, publieur, app) passent les MEMES cas ; c'est ce qui rend la
//! convention unique verifiable. Hors couverture DECLAREE cote Rust : le hachage du `<sid>`
//! (l'app compare des sid deja haches par le demon, elle n'en calcule aucun).

use std::collections::HashMap;

use iakaagentsmonitor_lib::contract::{
    agent_topic, agents_filter, normalize_node, parse_topic, Topic, ALL_OUT,
};
use iakaagentsmonitor_lib::fleet_model::{machine_alerts, machine_state, CodeMsg, MachineState};
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
