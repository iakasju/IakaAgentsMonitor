//! sources::remote — la source « Autres machines » (instruction de fusion D4) : `rumqttc` en TLS
//! vers le broker CENTRAL, abonnement `iaka/+/out/#` (jamais `in/`), table retenue des machines,
//! sessions et fiches d'agents des AUTRES machines. **Lecture seule** : l'app ne publie rien sur le
//! central et n'ouvre aucun port.
//!
//! Le transport est **injectable** ([`Feed`]) : la production utilise [`RumqttFeed`], les tests un
//! faux flux d'événements (ou un broker `rumqttd` éphémère sur la boucle locale) — jamais une
//! connexion à `mqtt.naonedge.com`. Sans configuration ([`RemoteSetting::NotConfigured`]), aucun
//! fil n'est lancé et aucune connexion n'est tentée : le Monitor marche seul (CA-7).
//!
//! L'absence ne se déguise jamais en vide : central injoignable/refusé ⇒ statut `unreachable` +
//! motif ; l'UI écrit « broker injoignable : <motif> ».

use rumqttc::{Client, Event, MqttOptions, Packet, QoS, SubscribeReasonCode, Transport};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use crate::card::parse_card;
use crate::contract::{self, parse_topic, Topic};
use crate::fleet_config::{RemoteConfig, RemoteSetting};
use crate::fleet_model::{build_fleet, decode_code_msg, MachineView, RemoteStore};

const RECONNECT_MIN_BACKOFF: Duration = Duration::from_secs(1);
const RECONNECT_MAX_BACKOFF: Duration = Duration::from_secs(30);

/// Ce qu'un flux de broker peut dire à la source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedEvent {
    /// Connexion acceptée par le broker (le magasin est repartie de zéro : les retenus arrivent).
    Connected,
    /// Message reçu (retenu ou non).
    Publish { topic: String, payload: Vec<u8> },
    /// Broker joignable mais abonnement refusé (ACL).
    Denied(String),
    /// Connexion perdue ou impossible, avec le MOTIF.
    Disconnected(String),
}

/// Flux d'événements d'un broker : bloque, émet des [`FeedEvent`] dans `sink`, et rend la main
/// quand `sink` répond `false` (arrêt demandé).
pub trait Feed: Send + 'static {
    fn run(&mut self, sink: &mut dyn FnMut(FeedEvent) -> bool);
}

/// État du lien avec le central, tel que l'UI le dit.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Link {
    NotConfigured,
    Connecting,
    Connected,
    Unreachable(String),
}

/// Vue sérialisée pour la webview : statut (`not_configured` | `connecting` | `connected` |
/// `unreachable`), motif éventuel, machines des AUTRES nodes.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteView {
    pub status: &'static str,
    pub reason: Option<String>,
    pub machines: Vec<MachineView>,
    pub own_node: Option<String>,
}

struct Inner {
    store: RemoteStore,
    link: Link,
    own_node: Option<String>,
}

impl Inner {
    fn is_own(&self, node: &str) -> bool {
        self.own_node.as_deref() == Some(node)
    }

    /// Applique un événement. Pur (aucune E/S) : testable sans réseau.
    fn apply(&mut self, ev: FeedEvent) {
        match ev {
            FeedEvent::Connected => {
                // Resynchronisation : les retenus effacés pendant une coupure ne doivent pas
                // survivre dans le magasin.
                self.store = RemoteStore::default();
                self.link = Link::Connected;
            }
            FeedEvent::Denied(reason) | FeedEvent::Disconnected(reason) => {
                self.link = Link::Unreachable(reason);
            }
            FeedEvent::Publish { topic, payload } => self.apply_publish(&topic, &payload),
        }
    }

    fn apply_publish(&mut self, topic: &str, payload: &[u8]) {
        match parse_topic(topic) {
            // Messages de son propre <node> : ignorés (pas de doublon avec « Ce poste »).
            Some(Topic::Presence { node, .. })
            | Some(Topic::Session { node, .. })
            | Some(Topic::Agent { node, .. })
                if self.is_own(&node) => {}
            Some(Topic::Presence { .. }) | Some(Topic::Session { .. }) => match decode_code_msg(payload) {
                Ok(Some(m)) => {
                    self.store.codes.insert(topic.to_string(), m);
                }
                Ok(None) => {
                    self.store.codes.remove(topic);
                }
                Err(_) => {} // hors contrat : ignoré, jamais bloquant
            },
            Some(Topic::Agent { .. }) => match parse_card(topic, payload) {
                Ok(Some(c)) => {
                    self.store.cards.insert(topic.to_string(), c);
                }
                Ok(None) => {
                    self.store.cards.remove(topic);
                }
                Err(_) => {}
            },
            None => {}
        }
    }
}

/// La source distante : un fil de fond (ou aucun), un magasin partagé.
pub struct RemoteSource {
    inner: Arc<Mutex<Inner>>,
    stop: Arc<AtomicBool>,
}

impl RemoteSource {
    /// Démarre selon la configuration : rien sans identifiants ; configuration refusée ⇒ dite ;
    /// sinon fil `rumqttc` (TLS obligatoire hors boucle locale).
    pub fn start(setting: &RemoteSetting, own_node: Option<String>) -> Self {
        match setting {
            RemoteSetting::NotConfigured => Self::inert(Link::NotConfigured, own_node),
            RemoteSetting::Refused(reason) => Self::inert(Link::Unreachable(reason.clone()), own_node),
            RemoteSetting::Configured(cfg) => {
                Self::start_with_feed(Box::new(RumqttFeed::new(cfg.clone())), own_node)
            }
        }
    }

    fn inert(link: Link, own_node: Option<String>) -> Self {
        RemoteSource {
            inner: Arc::new(Mutex::new(Inner { store: RemoteStore::default(), link, own_node })),
            stop: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Démarre avec un flux injecté (tests : faux broker).
    pub fn start_with_feed(mut feed: Box<dyn Feed>, own_node: Option<String>) -> Self {
        let inner = Arc::new(Mutex::new(Inner {
            store: RemoteStore::default(),
            link: Link::Connecting,
            own_node,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (inner_c, stop_c) = (inner.clone(), stop.clone());
        std::thread::spawn(move || {
            feed.run(&mut |ev| {
                inner_c.lock().unwrap().apply(ev);
                !stop_c.load(Ordering::Relaxed)
            });
        });
        RemoteSource { inner, stop }
    }

    /// Instantané pour l'UI. `machines` reste la dernière vue connue quand le lien est tombé
    /// (les états dérivent alors seuls vers « muette »), le statut dit pourquoi.
    pub fn view(&self, now_s: i64) -> RemoteView {
        let g = self.inner.lock().unwrap();
        let (status, reason) = match &g.link {
            Link::NotConfigured => ("not_configured", None),
            Link::Connecting => ("connecting", None),
            Link::Connected => ("connected", None),
            Link::Unreachable(r) => ("unreachable", Some(r.clone())),
        };
        RemoteView {
            status,
            reason,
            machines: build_fleet(&g.store, g.own_node.as_deref(), now_s),
            own_node: g.own_node.clone(),
        }
    }
}

impl Drop for RemoteSource {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Flux de production : `rumqttc`, TLS (racines du système, certificat vérifié) hors boucle locale.
pub struct RumqttFeed {
    cfg: RemoteConfig,
}

impl RumqttFeed {
    pub fn new(cfg: RemoteConfig) -> Self {
        RumqttFeed { cfg }
    }
}

/// Motif lisible d'une erreur de connexion — jamais d'identifiants.
fn describe(e: &rumqttc::ConnectionError) -> String {
    e.to_string()
}

impl Feed for RumqttFeed {
    fn run(&mut self, sink: &mut dyn FnMut(FeedEvent) -> bool) {
        let mut backoff = RECONNECT_MIN_BACKOFF;
        loop {
            let mut opts = MqttOptions::new(
                format!("iakaagentsmonitor-remote-{}", std::process::id()),
                self.cfg.host.clone(),
                self.cfg.port,
            );
            opts.set_keep_alive(Duration::from_secs(15));
            opts.set_credentials(self.cfg.user.clone(), self.cfg.pass.clone());
            if self.cfg.tls {
                // `tls_with_default_config` charge les racines du système et PANIQUE si elles sont
                // illisibles : on le convertit en motif dit, jamais en mort silencieuse du fil.
                match std::panic::catch_unwind(Transport::tls_with_default_config) {
                    Ok(t) => {
                        opts.set_transport(t);
                    }
                    Err(_) => {
                        if !sink(FeedEvent::Disconnected(
                            "certificats racine du système illisibles (TLS impossible)".to_string(),
                        )) {
                            return;
                        }
                        std::thread::sleep(backoff);
                        backoff = (backoff * 2).min(RECONNECT_MAX_BACKOFF);
                        continue;
                    }
                }
            }
            let (client, mut connection) = Client::new(opts, 64);
            for notification in connection.iter() {
                match notification {
                    Ok(Event::Incoming(Packet::ConnAck(_))) => {
                        backoff = RECONNECT_MIN_BACKOFF;
                        if !sink(FeedEvent::Connected) {
                            let _ = client.disconnect();
                            return;
                        }
                        let _ = client.subscribe(contract::ALL_OUT, QoS::AtLeastOnce);
                    }
                    Ok(Event::Incoming(Packet::SubAck(ack))) => {
                        if ack.return_codes.iter().any(|c| matches!(c, SubscribeReasonCode::Failure))
                            && !sink(FeedEvent::Denied(format!(
                                "abonnement {} refusé par le broker (ACL)",
                                contract::ALL_OUT
                            )))
                        {
                            let _ = client.disconnect();
                            return;
                        }
                    }
                    Ok(Event::Incoming(Packet::Publish(p))) => {
                        if !sink(FeedEvent::Publish { topic: p.topic.clone(), payload: p.payload.to_vec() }) {
                            let _ = client.disconnect();
                            return;
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        if !sink(FeedEvent::Disconnected(describe(&e))) {
                            return;
                        }
                        break; // reconnexion après attente croissante bornée
                    }
                }
            }
            // Attente par tranches de 200 ms : un arrêt demandé n'attend pas 30 s.
            let mut waited = Duration::ZERO;
            while waited < backoff {
                std::thread::sleep(Duration::from_millis(200));
                waited += Duration::from_millis(200);
            }
            backoff = (backoff * 2).min(RECONNECT_MAX_BACKOFF);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::time::Instant;

    /// Faux broker : les événements arrivent par un canal, depuis le test.
    struct ChannelFeed(Receiver<FeedEvent>);

    impl Feed for ChannelFeed {
        fn run(&mut self, sink: &mut dyn FnMut(FeedEvent) -> bool) {
            while let Ok(ev) = self.0.recv() {
                if !sink(ev) {
                    return;
                }
            }
        }
    }

    fn source(own: Option<&str>) -> (RemoteSource, Sender<FeedEvent>) {
        let (tx, rx) = channel();
        (RemoteSource::start_with_feed(Box::new(ChannelFeed(rx)), own.map(str::to_string)), tx)
    }

    fn publish(tx: &Sender<FeedEvent>, topic: &str, payload: &str) {
        tx.send(FeedEvent::Publish { topic: topic.to_string(), payload: payload.as_bytes().to_vec() }).unwrap();
    }

    fn wait_for(src: &RemoteSource, pred: impl Fn(&RemoteView) -> bool) -> RemoteView {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let v = src.view(10_000);
            if pred(&v) {
                return v;
            }
            assert!(Instant::now() < deadline, "condition jamais atteinte : {v:?}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn card(sid: &str, agent: &str, task: Option<&str>) -> String {
        serde_json::json!({"v":1,"runner":"claude-code","agentId":agent,"parentId":null,"persona":"Gimli",
            "sessionId":sid,"task":task,"startedAt":9000,"lastSeenAt":9990,"state":"running"}).to_string()
    }

    #[test]
    fn demarre_en_connexion_puis_connecte() {
        let (src, tx) = source(None);
        assert_eq!(src.view(0).status, "connecting");
        tx.send(FeedEvent::Connected).unwrap();
        wait_for(&src, |v| v.status == "connected");
    }

    #[test]
    fn broker_injoignable_dit_le_motif_jamais_une_liste_vide_muette() {
        // CA-8 : le statut porte le motif ; l'UI écrit « broker injoignable : <motif> ».
        let (src, tx) = source(None);
        tx.send(FeedEvent::Disconnected("Connection refused (os error 61)".into())).unwrap();
        let v = wait_for(&src, |v| v.status == "unreachable");
        assert_eq!(v.reason.as_deref(), Some("Connection refused (os error 61)"));
    }

    #[test]
    fn abonnement_refuse_par_acl_est_dit() {
        let (src, tx) = source(None);
        tx.send(FeedEvent::Connected).unwrap();
        tx.send(FeedEvent::Denied("abonnement iaka/+/out/# refusé par le broker (ACL)".into())).unwrap();
        let v = wait_for(&src, |v| v.status == "unreachable");
        assert!(v.reason.unwrap().contains("ACL"));
    }

    #[test]
    fn son_propre_node_est_ignore_les_autres_apparaissent() {
        let (src, tx) = source(Some("mac"));
        tx.send(FeedEvent::Connected).unwrap();
        publish(&tx, "iaka/mac/out/presence/online/current", r#"{"v":true,"t":9990}"#);
        publish(&tx, "iaka/pc/out/presence/online/current", r#"{"v":true,"t":9990}"#);
        publish(&tx, "iaka/pc/out/presence/last_tick_at/current", r#"{"v":9990,"t":9990}"#);
        publish(&tx, "iaka/mac/out/agents/claude-code/aaaaaaaaaaaa/main", &card("aaaaaaaaaaaa", "main", None));
        let v = wait_for(&src, |v| v.machines.iter().any(|m| m.node == "pc"));
        assert_eq!(v.machines.len(), 1, "le node local ne doit pas apparaitre : {v:?}");
        assert_eq!(v.machines[0].node, "pc");
        assert_eq!(v.machines[0].state.as_str(), "online");
    }

    #[test]
    fn agents_rattaches_par_sid_task_masquee_dite_sinon_non_rattaches() {
        let (src, tx) = source(Some("mac"));
        tx.send(FeedEvent::Connected).unwrap();
        publish(&tx, "iaka/pc/out/presence/online/current", r#"{"v":true,"t":9990}"#);
        publish(&tx, "iaka/pc/out/presence/last_tick_at/current", r#"{"v":9990,"t":9990}"#);
        publish(&tx, "iaka/pc/out/sessions/7/status/current", r#"{"v":"busy","t":9990}"#);
        publish(&tx, "iaka/pc/out/sessions/7/sid/current", r#"{"v":"aaaaaaaaaaaa","t":9990}"#);
        publish(&tx, "iaka/pc/out/agents/claude-code/aaaaaaaaaaaa/main", &card("aaaaaaaaaaaa", "main", None));
        publish(&tx, "iaka/pc/out/agents/claude-code/bbbbbbbbbbbb/main", &card("bbbbbbbbbbbb", "main", Some("relire")));
        let v = wait_for(&src, |v| v.machines.first().is_some_and(|m| !m.unattached_agents.is_empty()));
        let m = &v.machines[0];
        assert_eq!(m.sessions[0].agents.len(), 1);
        assert!(m.sessions[0].agents[0].task_masked, "task null => tâche masquée");
        assert_eq!(m.unattached_agents[0].task.as_deref(), Some("relire"));
        assert!(!m.unattached_agents[0].task_masked);
    }

    #[test]
    fn payload_vide_efface_hors_contrat_ignore() {
        let (src, tx) = source(None);
        tx.send(FeedEvent::Connected).unwrap();
        publish(&tx, "iaka/pc/out/presence/online/current", r#"{"v":true,"t":9990}"#);
        publish(&tx, "iaka/pc/out/presence/os/current", "pas du json");
        publish(&tx, "iaka/pc/out/presence/os/current", r#"{"v":"linux","t":1,"x":2}"#);
        wait_for(&src, |v| v.machines.len() == 1);
        assert_eq!(src.view(10_000).machines[0].os, None);
        publish(&tx, "iaka/pc/out/presence/online/current", "");
        wait_for(&src, |v| v.machines.is_empty());
    }

    #[test]
    fn reconnexion_repart_de_zero_les_retenus_effaces_pendant_la_coupure_disparaissent() {
        let (src, tx) = source(None);
        tx.send(FeedEvent::Connected).unwrap();
        publish(&tx, "iaka/pc/out/presence/online/current", r#"{"v":true,"t":9990}"#);
        wait_for(&src, |v| v.machines.len() == 1);
        tx.send(FeedEvent::Disconnected("réseau coupé".into())).unwrap();
        wait_for(&src, |v| v.status == "unreachable");
        assert_eq!(src.view(10_000).machines.len(), 1, "dernier connu conservé pendant la coupure");
        tx.send(FeedEvent::Connected).unwrap();
        wait_for(&src, |v| v.status == "connected" && v.machines.is_empty());
    }

    #[test]
    fn sans_configuration_aucun_fil_aucune_connexion() {
        let src = RemoteSource::start(&RemoteSetting::NotConfigured, Some("mac".into()));
        let v = src.view(0);
        assert_eq!(v.status, "not_configured");
        assert!(v.machines.is_empty() && v.reason.is_none());
    }

    #[test]
    fn configuration_refusee_est_dite() {
        let src = RemoteSource::start(&RemoteSetting::Refused("mqtt:// en clair refusé".into()), None);
        let v = src.view(0);
        assert_eq!(v.status, "unreachable");
        assert!(v.reason.unwrap().contains("en clair refusé"));
    }
}
