//! Tests d'intégration contre un broker `rumqttd` ÉPHÉMÈRE (étape 11 de
//! specs/instructions/agnostique-mqtt.md), sur un PORT LIBRE — jamais 1883 (celui d'iakahub,
//! réutilisé mais jamais accaparé par les tests). Patron repris de `iakaTokenCounter/iakahub`
//! (`broker.rs`, `tests/retained_backlog_ceiling.rs`) : gabarit TOML embarqué avec un placeholder
//! de `listen`, `rumqttd::Broker::start()` bloquant dans un thread dédié, sonde de connexion TCP
//! avant de rendre la main.
//!
//! Limite assumée (documentée plutôt que contournée) : `rumqttd::Broker` n'expose pas d'arrêt
//! propre dans cette version — on ne peut donc pas éteindre le broker éphémère EN COURS DE TEST
//! pour observer la bascule "broker arrêté -> fichiers" sur la MÊME instance. Le dernier test
//! ci-dessous prouve la même mécanique par contraste : deux `BusSource` sur deux ports différents
//! (un avec broker, un sans) donnent deux verdicts `is_connected()` opposés — c'est exactement le
//! signal sur lequel `watcher::run_watcher` bascule (Q3-A).

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use rumqttd::{Broker, Config};

use iakaagentsmonitor_lib::sources::bus::BusSource;
use iakaagentsmonitor_lib::tree;

const CONFIG_TEMPLATE: &str = r#"
id = 0

[router]
max_connections = 100
max_outgoing_packet_count = 200
max_segment_size = 104857600
max_segment_count = 10

[v4.1]
name = "iaka-am-test-v4"
listen = "__LISTEN__"
next_connection_delay_ms = 1

    [v4.1.connections]
    connection_timeout_ms = 60000
    max_payload_size = 20480
    max_inflight_count = 100
    dynamic_filters = true
"#;

fn free_port() -> u16 {
    let l = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemere");
    l.local_addr().unwrap().port()
}

fn wait_until_listening(addr: SocketAddr, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
            return;
        }
        if Instant::now() >= deadline {
            panic!("le broker ephemere n'ecoute pas sur {addr} apres {timeout:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Démarre un broker `rumqttd` éphémère sur `127.0.0.1:<port>` dans un thread dédié (jamais
/// arrêté explicitement — cf. doc de module) et attend qu'il écoute.
fn start_ephemeral_broker(port: u16) {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let toml_str = CONFIG_TEMPLATE.replace("__LISTEN__", &addr.to_string());
    let config: Config = toml::from_str(&toml_str).expect("gabarit rumqttd invalide");
    std::thread::Builder::new()
        .name("iaka-am-test-broker".to_string())
        .spawn(move || {
            let mut broker = Broker::new(config);
            let _ = broker.start();
        })
        .expect("thread broker");
    wait_until_listening(addr, Duration::from_secs(5));
}

/// Publie une fiche (ou une suppression, `payload` vide) retenue en QoS 1, avec un client
/// `rumqttc` de courte durée — simule "un autre runner" qui publie AVANT que l'app ne s'abonne.
fn publish_retained(port: u16, topic: &str, payload: &[u8]) {
    let client_id = format!("test-pub-{}-{}", std::process::id(), fastrand_suffix());
    let opts = MqttOptions::new(client_id, "127.0.0.1", port);
    let (client, mut connection) = Client::new(opts, 10);
    client
        .publish(topic, QoS::AtLeastOnce, true, payload.to_vec())
        .expect("publish");
    // Draine l'event loop jusqu'au PubAck (ou un nombre borné d'itérations, défensif).
    for (i, notif) in connection.iter().enumerate() {
        match notif {
            Ok(Event::Incoming(Packet::PubAck(_))) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        if i > 20 {
            break;
        }
    }
}

fn fastrand_suffix() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() % 1_000_000) as u32
}

fn wait_until<F: Fn() -> bool>(pred: F, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if pred() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn card_json(runner: &str, session_id: &str, agent_id: &str, last_seen_at: i64) -> String {
    serde_json::json!({
        "v": 1,
        "runner": runner,
        "agentId": agent_id,
        "parentId": null,
        "sessionId": session_id,
        "startedAt": last_seen_at - 60,
        "lastSeenAt": last_seen_at,
        "state": "running"
    })
    .to_string()
}

fn now_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

#[test]
fn fiche_retenue_publiee_avant_abonnement_est_recue() {
    let port = free_port();
    start_ephemeral_broker(port);

    let topic = "iaka/testnode/out/agents/essai/sess-int-1/main";
    publish_retained(port, topic, card_json("essai", "sess-int-1", "main", now_secs()).as_bytes());
    std::thread::sleep(Duration::from_millis(300)); // laisse le retenu s'installer côté broker.

    let bus = BusSource::start_for_node(port, Some("testnode"));
    assert!(wait_until(|| bus.is_connected(), Duration::from_secs(5)), "bus jamais connecte");
    assert!(wait_until(|| !bus.cards().is_empty(), Duration::from_secs(5)), "fiche jamais recue");

    let cards = bus.cards();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].session_id, "sess-int-1");
    assert_eq!(cards[0].agent_id, "main");
}

#[test]
fn payload_vide_efface_la_fiche_retenue() {
    let port = free_port();
    start_ephemeral_broker(port);

    let topic = "iaka/testnode/out/agents/essai/sess-int-2/main";
    let bus = BusSource::start_for_node(port, Some("testnode"));
    assert!(wait_until(|| bus.is_connected(), Duration::from_secs(5)));

    publish_retained(port, topic, card_json("essai", "sess-int-2", "main", now_secs()).as_bytes());
    assert!(wait_until(|| !bus.cards().is_empty(), Duration::from_secs(5)));

    publish_retained(port, topic, b""); // suppression (contrat § Fin, plantage, expiration).
    assert!(wait_until(|| bus.cards().is_empty(), Duration::from_secs(5)), "fiche jamais effacee");
}

#[test]
fn fiche_perimee_est_masquee_par_le_coeur_neutre() {
    let port = free_port();
    start_ephemeral_broker(port);

    let now = now_secs();
    let topic = "iaka/testnode/out/agents/essai/sess-int-3/main";
    publish_retained(port, topic, card_json("essai", "sess-int-3", "main", now - 91).as_bytes());
    std::thread::sleep(Duration::from_millis(300));

    let bus = BusSource::start_for_node(port, Some("testnode"));
    assert!(wait_until(|| bus.is_connected(), Duration::from_secs(5)));
    assert!(wait_until(|| !bus.cards().is_empty(), Duration::from_secs(5)));

    // La fiche est bien REÇUE (le bus ne filtre rien) mais MASQUÉE par le cœur neutre : la
    // péremption (90 s) est une décision de `tree::build_snapshot`, jamais de la source.
    let labels = iakaagentsmonitor_lib::labels::Labels::default();
    let roster = iakaagentsmonitor_lib::roster::Roster::new();
    let snap = tree::build_snapshot(&bus.cards(), now, 90, &labels, &roster);
    assert!(snap.sessions.is_empty(), "fiche perimee (91 s) doit etre masquee");
}

#[test]
fn broker_absent_vs_present_donnent_des_verdicts_is_connected_opposes() {
    // Cf. doc de module : preuve par contraste, faute d'arret propre du broker ephemere en
    // cours de test. C'est exactement le signal binaire dont depend la bascule Q3-A.
    let port_with_broker = free_port();
    start_ephemeral_broker(port_with_broker);
    let bus_ok = BusSource::start_for_node(port_with_broker, Some("testnode"));
    assert!(wait_until(|| bus_ok.is_connected(), Duration::from_secs(5)));

    let port_without_broker = free_port(); // libéré juste après : rien n'y écoute.
    let bus_absent = BusSource::start_for_node(port_without_broker, Some("testnode"));
    std::thread::sleep(Duration::from_millis(300));
    assert!(!bus_absent.is_connected(), "aucun broker sur ce port : doit rester deconnecte");
}
