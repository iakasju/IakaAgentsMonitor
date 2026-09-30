//! sources::bus — la source BUS (specs/instructions/agnostique-mqtt.md, étape 6) : fil dédié
//! `rumqttc`, table `topic -> fiche` mise à jour à chaque message, suppression sur payload vide,
//! drapeau `connected`. Module NEUTRE : aucun nom de runner (les runners parlent au bus, cette
//! source ne fait qu'écouter).
//!
//! DA-M2 : hôte FORCÉ à `127.0.0.1` (aucune variable pour le changer), port `IAKAAM_MQTT_PORT`
//! (défaut 1883), identifiant client `iakaagentsmonitor-<pid>`, QoS 1, reconnexion avec attente
//! croissante bornée à 30 s. DA-M1 : `rumqttc` 0.24.

use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use crate::card::{parse_card, AgentCard};
use crate::contract;

/// Filtre d'abonnement du bus LOCAL (contrat v2 § 1) : `iaka/<node>/out/agents/+/+/+` — tout
/// runner, toute session, tout agent de CETTE machine. Si le `<node>` ne peut pas être déterminé
/// (nom d'hôte refusé), repli sur `+` : le bus local ne contient que les publieurs de la machine,
/// mieux vaut lire trop large que ne rien voir.
pub fn topic_filter(node: Option<&str>) -> String {
    contract::agents_filter(node.unwrap_or("+"))
}

const CLIENT_ID_PREFIX: &str = "iakaagentsmonitor";
const DEFAULT_PORT: u16 = 1883;
const PORT_ENV: &str = "IAKAAM_MQTT_PORT";
const RECONNECT_MIN_BACKOFF: Duration = Duration::from_secs(1);
const RECONNECT_MAX_BACKOFF: Duration = Duration::from_secs(30);

/// Avertissement journalisé à partir de 80 fiches reçues (contrat § « Fin, plantage,
/// expiration », plafond de rattrapage à 100 par filtre côté broker).
const WARN_THRESHOLD: usize = 80;

/// Port MQTT lu depuis [`PORT_ENV`], défaut [`DEFAULT_PORT`].
pub fn port_from_env() -> u16 {
    std::env::var(PORT_ENV)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

/// Applique un message PUBLISH reçu à la table de fiches : fiche valide -> insérée/mise à jour ;
/// payload vide -> suppression ; fiche invalide -> ignorée (jamais de panique). Fonction PURE
/// (aucune E/S réseau), donc testable sans broker réel.
pub fn apply_publish(cards: &mut HashMap<String, AgentCard>, topic: &str, payload: &[u8]) {
    match parse_card(topic, payload) {
        Ok(Some(card)) => {
            cards.insert(topic.to_string(), card);
        }
        Ok(None) => {
            cards.remove(topic);
        }
        Err(_) => {
            // Fiche rejetée (JSON invalide, version non supportée, incohérence topic…) : ignorée,
            // l'app reste intacte (critère d'acceptation « fiche invalide -> ignorée »).
        }
    }
}

/// Source bus vivante : fil dédié en tâche de fond, table de fiches partagée, drapeau de
/// connexion. `port` figé à la création (repris tel quel par les tests d'intégration contre un
/// broker éphémère, étape 11).
pub struct BusSource {
    cards: Arc<Mutex<HashMap<String, AgentCard>>>,
    connected: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

impl BusSource {
    /// Démarre le fil de connexion (hôte forcé à `127.0.0.1`, DA-M2). `port` explicite pour les
    /// tests d'intégration ; en production, [`port_from_env`].
    pub fn start(port: u16) -> Self {
        let node = crate::fleet_config::load().node;
        Self::start_for_node(port, node.as_deref())
    }

    /// Comme [`BusSource::start`], `<node>` explicite (tests d'intégration, indépendants du nom
    /// d'hôte de la machine qui les lance).
    pub fn start_for_node(port: u16, node: Option<&str>) -> Self {
        let filter = topic_filter(node);
        let cards: Arc<Mutex<HashMap<String, AgentCard>>> = Arc::new(Mutex::new(HashMap::new()));
        let connected = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));

        let cards_clone = cards.clone();
        let connected_clone = connected.clone();
        let stop_clone = stop.clone();
        std::thread::spawn(move || run_loop(port, filter, cards_clone, connected_clone, stop_clone));

        BusSource { cards, connected, stop }
    }

    /// `true` si un `ConnAck` a été reçu récemment (le fil de connexion est vivant).
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Toutes les fiches actuellement retenues côté abonné (une par topic).
    pub fn cards(&self) -> Vec<AgentCard> {
        self.cards.lock().unwrap().values().cloned().collect()
    }
}

impl Drop for BusSource {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn client_id() -> String {
    format!("{CLIENT_ID_PREFIX}-{}", std::process::id())
}

fn run_loop(
    port: u16,
    filter: String,
    cards: Arc<Mutex<HashMap<String, AgentCard>>>,
    connected: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
) {
    let mut backoff = RECONNECT_MIN_BACKOFF;
    while !stop.load(Ordering::Relaxed) {
        let mut opts = MqttOptions::new(client_id(), "127.0.0.1", port);
        opts.set_keep_alive(Duration::from_secs(15));

        let (client, mut connection) = Client::new(opts, 64);
        let mut warned = false;

        for notification in connection.iter() {
            if stop.load(Ordering::Relaxed) {
                let _ = client.disconnect();
                break;
            }
            match notification {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    connected.store(true, Ordering::Relaxed);
                    backoff = RECONNECT_MIN_BACKOFF;
                    let _ = client.subscribe(filter.as_str(), QoS::AtLeastOnce);
                }
                Ok(Event::Incoming(Packet::Publish(p))) => {
                    apply_publish(&mut cards.lock().unwrap(), &p.topic, &p.payload);
                    if !warned {
                        let count = cards.lock().unwrap().len();
                        if count >= WARN_THRESHOLD {
                            eprintln!(
                                "iakaagentsmonitor: {count} fiches retenues reçues (plafond broker 100 par filtre)"
                            );
                            warned = true;
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => break, // boucle for se termine -> reconnexion après backoff.
            }
        }

        connected.store(false, Ordering::Relaxed);
        if stop.load(Ordering::Relaxed) {
            break;
        }
        std::thread::sleep(backoff);
        backoff = (backoff * 2).min(RECONNECT_MAX_BACKOFF);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload_of(session_id: &str, agent_id: &str) -> String {
        serde_json::json!({
            "v": 1,
            "runner": "essai",
            "agentId": agent_id,
            "parentId": null,
            "sessionId": session_id,
            "startedAt": 1000,
            "lastSeenAt": 1010,
            "state": "running"
        })
        .to_string()
    }

    #[test]
    fn apply_publish_ajoute_une_fiche_valide() {
        let mut cards = HashMap::new();
        let topic = "iaka/mac/out/agents/essai/s1/main";
        apply_publish(&mut cards, topic, payload_of("s1", "main").as_bytes());
        assert_eq!(cards.len(), 1);
        assert!(cards.contains_key(topic));
    }

    #[test]
    fn apply_publish_payload_vide_supprime_la_fiche() {
        let mut cards = HashMap::new();
        let topic = "iaka/mac/out/agents/essai/s1/main";
        apply_publish(&mut cards, topic, payload_of("s1", "main").as_bytes());
        apply_publish(&mut cards, topic, b"");
        assert!(cards.is_empty());
    }

    #[test]
    fn apply_publish_fiche_invalide_est_ignoree() {
        let mut cards = HashMap::new();
        let topic = "iaka/mac/out/agents/essai/s1/main";
        apply_publish(&mut cards, topic, b"{ pas du json");
        assert!(cards.is_empty());
    }

    #[test]
    fn filtre_local_par_node_et_repli_large() {
        assert_eq!(topic_filter(Some("mac")), "iaka/mac/out/agents/+/+/+");
        assert_eq!(topic_filter(None), "iaka/+/out/agents/+/+/+");
    }

    #[test]
    fn topic_v1_est_ignore() {
        let mut cards = HashMap::new();
        apply_publish(&mut cards, "iaka/agents/out/essai/s1/main", payload_of("s1", "main").as_bytes());
        assert!(cards.is_empty());
    }

    #[test]
    fn port_from_env_defaut_1883() {
        std::env::remove_var(PORT_ENV);
        assert_eq!(port_from_env(), 1883);
    }

    #[test]
    fn bus_source_demarre_et_s_arrete_sans_panique_broker_absent() {
        // Port improbable, aucun broker en écoute : la source doit se comporter proprement (pas
        // de connexion, `is_connected() == false`), sans jamais planter.
        let bus = BusSource::start(18_831);
        std::thread::sleep(Duration::from_millis(50));
        assert!(!bus.is_connected());
        assert!(bus.cards().is_empty());
    }
}
