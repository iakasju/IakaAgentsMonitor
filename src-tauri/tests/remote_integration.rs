//! Source distante contre un broker `rumqttd` ÉPHÉMÈRE sur la boucle locale (`mqtt://127.0.0.1`,
//! port libre) : prouve le chemin `RumqttFeed` de bout en bout (connexion, abonnement
//! `iaka/+/out/#`, retenus, resynchronisation) SANS jamais toucher `mqtt.naonedge.com`. Le TLS
//! n'est pas exercé ici (pas de certificat de test) : sa vérification contre le VPS est la mesure
//! `M4`, non mesurée dans ce lot (dépend du Lot 0).

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use rumqttd::{Broker, Config};

use iakaagentsmonitor_lib::fleet_config::{RemoteConfig, RemoteSetting};
use iakaagentsmonitor_lib::sources::remote::{RemoteSource, RemoteView};

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

fn cfg(port: u16) -> RemoteSetting {
    RemoteSetting::Configured(RemoteConfig {
        host: "127.0.0.1".into(),
        port,
        tls: false,
        user: "fleet-test".into(),
        pass: "pas-un-vrai-secret".into(),
    })
}

fn view_when(src: &RemoteSource, pred: impl Fn(&RemoteView) -> bool, what: &str) -> RemoteView {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let v = src.view(now_secs());
        if pred(&v) {
            return v;
        }
        assert!(Instant::now() < deadline, "{what} : {v:?}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn now_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

#[test]
fn machines_retenues_recues_son_node_ignore_efface_repercute() {
    let port = free_port();
    start_ephemeral_broker(port);
    let now = now_secs();
    let msg = |v: &str| format!(r#"{{"v":{v},"t":{now}}}"#);
    publish_retained(port, "iaka/pc/out/presence/online/current", msg("true").as_bytes());
    publish_retained(port, "iaka/pc/out/presence/last_tick_at/current", msg(&now.to_string()).as_bytes());
    publish_retained(port, "iaka/pc/out/sessions/9/status/current", msg("\"busy\"").as_bytes());
    publish_retained(port, "iaka/mac/out/presence/online/current", msg("true").as_bytes());
    std::thread::sleep(Duration::from_millis(300));

    let src = RemoteSource::start(&cfg(port), Some("mac".into()));
    let v = view_when(&src, |v| v.status == "connected" && !v.machines.is_empty(), "machine distante jamais recue");
    assert_eq!(v.machines.len(), 1, "le node local (mac) ne doit pas apparaitre : {v:?}");
    assert_eq!(v.machines[0].node, "pc");
    assert_eq!(v.machines[0].state.as_str(), "online");
    assert_eq!(v.machines[0].sessions[0].pid, 9);

    publish_retained(port, "iaka/pc/out/sessions/9/status/current", b"");
    view_when(&src, |v| v.machines.first().is_some_and(|m| m.sessions.is_empty()), "effacement jamais repercute");
}

#[test]
fn broker_injoignable_statut_unreachable_avec_motif() {
    let port = free_port(); // libéré juste après : rien n'y écoute.
    let src = RemoteSource::start(&cfg(port), Some("mac".into()));
    let v = view_when(&src, |v| v.status == "unreachable", "jamais unreachable");
    assert!(v.reason.as_deref().is_some_and(|r| !r.is_empty()), "le motif ne doit jamais etre vide : {v:?}");
    assert!(!v.reason.unwrap().contains("pas-un-vrai-secret"), "le mot de passe ne fuit jamais");
}
