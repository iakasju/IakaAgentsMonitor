//! fleet_config — configuration de la vue distante : `~/.iakafleet/.env` (fichier du démon,
//! instruction de fusion D4/D6), les variables d'environnement priment. Aucun secret en dur ;
//! le mot de passe n'apparaît JAMAIS dans un `Debug`, un journal ni un message d'erreur.
//!
//! **Rien ne change sans configuration** : sans identifiants, le central n'est pas configuré et
//! l'app ne tente AUCUNE connexion sortante (hors `127.0.0.1:<port local>` du bus local).

use std::collections::HashMap;

use crate::contract;

/// Broker central par défaut (contrat § 5) — utilisé seulement si des identifiants existent.
pub const DEFAULT_URL: &str = "mqtts://mqtt.naonedge.com:8883";

/// Central configuré : hôte, port, TLS obligatoire hors boucle locale, identifiants.
#[derive(Clone, PartialEq, Eq)]
pub struct RemoteConfig {
    pub host: String,
    pub port: u16,
    /// `true` pour `mqtts://` (certificat vérifié, non débrayable) ; `false` seulement pour la
    /// boucle locale (`mqtt://127.0.0.1` / `localhost`, utilisé par les tests).
    pub tls: bool,
    pub user: String,
    pub pass: String,
}

impl std::fmt::Debug for RemoteConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("tls", &self.tls)
            .field("user", &self.user)
            .field("pass", &"<masqué>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteSetting {
    /// Pas d'identifiants : le Monitor marche seul, aucune connexion vers un central.
    NotConfigured,
    /// Configuration refusée (URL en clair vers un hôte distant…) : DITE, jamais silencieuse.
    Refused(String),
    Configured(RemoteConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetConfig {
    /// `<node>` de cette machine ; `None` si refusé/indéterminable.
    pub node: Option<String>,
    pub remote: RemoteSetting,
}

/// Analyse un contenu `.env` : `KEY=VALUE`, commentaires `#`, guillemets retirés (même règle que
/// `fleet/src/shared/config.js`).
pub fn parse_env(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(eq) = line.find('=') else { continue };
        if eq < 1 {
            continue;
        }
        let key = line[..eq].trim().to_string();
        let mut val = line[eq + 1..].trim();
        if val.len() >= 2
            && ((val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\'')))
        {
            val = &val[1..val.len() - 1];
        }
        if !val.is_empty() {
            out.insert(key, val.to_string());
        }
    }
    out
}

/// `mqtts://` partout ; `mqtt://` en clair seulement vers `127.0.0.1` / `localhost`.
pub fn parse_broker_url(url: &str) -> Result<(bool, String, u16), String> {
    let (scheme, rest) = url.split_once("://").ok_or_else(|| format!("URL de broker invalide : \"{url}\""))?;
    let tls = match scheme {
        "mqtts" => true,
        "mqtt" => false,
        other => return Err(format!("protocole \"{other}:\" non accepté (mqtts:// attendu)")),
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (h, Some(p.parse::<u16>().map_err(|_| format!("port invalide dans \"{url}\""))?)),
        None => (authority, None),
    };
    if host.is_empty() {
        return Err(format!("URL de broker invalide : \"{url}\""));
    }
    if !tls && host != "127.0.0.1" && host != "localhost" {
        return Err(format!(
            "mqtt:// en clair refusé vers \"{host}\" : seul mqtts:// est accepté hors 127.0.0.1/localhost"
        ));
    }
    Ok((tls, host.to_string(), port.unwrap_or(if tls { 8883 } else { 1883 })))
}

/// Configuration à partir de sources injectées (pure, testable) : `env` = variables du processus,
/// `file` = contenu de `~/.iakafleet/.env` (ou `None`).
pub fn from_sources(env: &dyn Fn(&str) -> Option<String>, file: Option<&str>) -> FleetConfig {
    let file_env = file.map(parse_env).unwrap_or_default();
    let get = |k: &str| env(k).filter(|v| !v.is_empty()).or_else(|| file_env.get(k).cloned());

    // IAKA_NODE prioritaire ; IAKAFLEET_HOST conservé en alias (contrat § 1, D6).
    let node = contract::resolve_node(get("IAKA_NODE").or_else(|| get("IAKAFLEET_HOST")).as_deref());

    // Compte de la machine (lit toute la flotte, A-3) sinon compte lecteur `fleet-console`.
    let creds = match (get("IAKAFLEET_AGENT_USER"), get("IAKAFLEET_AGENT_PASS")) {
        (Some(u), Some(p)) => Some((u, p)),
        _ => match (get("IAKAFLEET_CONSOLE_USER"), get("IAKAFLEET_CONSOLE_PASS")) {
            (Some(u), Some(p)) => Some((u, p)),
            _ => None,
        },
    };
    let remote = match creds {
        None => RemoteSetting::NotConfigured,
        Some((user, pass)) => {
            let url = get("IAKAFLEET_MQTT_URL").unwrap_or_else(|| DEFAULT_URL.to_string());
            match parse_broker_url(&url) {
                Ok((tls, host, port)) => RemoteSetting::Configured(RemoteConfig { host, port, tls, user, pass }),
                Err(e) => RemoteSetting::Refused(e),
            }
        }
    };
    FleetConfig { node, remote }
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(Into::into)
}

/// Charge la configuration réelle : environnement du processus + `~/.iakafleet/.env` (absent = ok).
pub fn load() -> FleetConfig {
    let file = home_dir().and_then(|h| std::fs::read_to_string(h.join(".iakafleet").join(".env")).ok());
    from_sources(&|k| std::env::var(k).ok(), file.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(env: &[(&str, &str)], file: Option<&str>) -> FleetConfig {
        let env: HashMap<String, String> = env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        from_sources(&|k| env.get(k).cloned(), file)
    }

    #[test]
    fn sans_identifiants_le_central_n_est_pas_configure() {
        // CA-7 : rien ne change sans configuration — même si une URL est présente.
        assert_eq!(cfg(&[], None).remote, RemoteSetting::NotConfigured);
        assert_eq!(cfg(&[("IAKAFLEET_MQTT_URL", "mqtts://x:8883")], None).remote, RemoteSetting::NotConfigured);
        assert_eq!(cfg(&[("IAKAFLEET_AGENT_USER", "u")], None).remote, RemoteSetting::NotConfigured);
    }

    #[test]
    fn compte_machine_prioritaire_sur_console_defaut_tls_naonedge() {
        let c = cfg(
            &[("IAKAFLEET_AGENT_USER", "fleet-mac"), ("IAKAFLEET_AGENT_PASS", "p1"),
              ("IAKAFLEET_CONSOLE_USER", "fleet-console"), ("IAKAFLEET_CONSOLE_PASS", "p2")],
            None,
        );
        match c.remote {
            RemoteSetting::Configured(r) => {
                assert_eq!((r.user.as_str(), r.host.as_str(), r.port, r.tls), ("fleet-mac", "mqtt.naonedge.com", 8883, true));
            }
            other => panic!("attendu Configured, recu {other:?}"),
        }
        let c = cfg(&[("IAKAFLEET_CONSOLE_USER", "fleet-console"), ("IAKAFLEET_CONSOLE_PASS", "p2")], None);
        assert!(matches!(c.remote, RemoteSetting::Configured(r) if r.user == "fleet-console"));
    }

    #[test]
    fn fichier_env_lu_et_variables_du_processus_priment() {
        let file = "IAKA_NODE=depuis-fichier\nIAKAFLEET_CONSOLE_USER=u\nIAKAFLEET_CONSOLE_PASS=\"p\"\n";
        assert_eq!(cfg(&[], Some(file)).node.as_deref(), Some("depuis-fichier"));
        assert_eq!(cfg(&[("IAKA_NODE", "Env-Node")], Some(file)).node.as_deref(), Some("env-node"));
        assert!(matches!(cfg(&[], Some(file)).remote, RemoteSetting::Configured(_)));
        assert_eq!(cfg(&[("IAKAFLEET_HOST", "ancien")], None).node.as_deref(), Some("ancien"));
        assert_eq!(cfg(&[("IAKA_NODE", "a/b")], None).node, None, "node refusé => None, jamais inventé");
    }

    #[test]
    fn mqtt_en_clair_refuse_hors_boucle_locale() {
        let creds = [("IAKAFLEET_CONSOLE_USER", "u"), ("IAKAFLEET_CONSOLE_PASS", "p")];
        let mk = |url: &'static str| {
            let mut e = creds.to_vec();
            e.push(("IAKAFLEET_MQTT_URL", url));
            cfg(&e, None).remote
        };
        assert!(matches!(mk("mqtt://mqtt.naonedge.com:1883"), RemoteSetting::Refused(_)));
        assert!(matches!(mk("ws://localhost:9001"), RemoteSetting::Refused(_)));
        assert!(matches!(mk("pas une url"), RemoteSetting::Refused(_)));
        assert!(matches!(mk("mqtt://127.0.0.1:1883"), RemoteSetting::Configured(r) if !r.tls && r.port == 1883));
        assert!(matches!(mk("mqtts://broker.exemple.org"), RemoteSetting::Configured(r) if r.tls && r.port == 8883));
    }

    #[test]
    fn le_mot_de_passe_n_apparait_jamais_dans_debug() {
        let c = cfg(&[("IAKAFLEET_CONSOLE_USER", "u"), ("IAKAFLEET_CONSOLE_PASS", "SECRET-PASS")], None);
        assert!(!format!("{c:?}").contains("SECRET-PASS"));
    }

    #[test]
    fn parse_env_commentaires_guillemets_vides() {
        let o = parse_env("# c\nA=1\nB=\"deux\"\nC=\n  D = 'x y' \nnope\n");
        assert_eq!(o.get("A").map(String::as_str), Some("1"));
        assert_eq!(o.get("B").map(String::as_str), Some("deux"));
        assert_eq!(o.get("D").map(String::as_str), Some("x y"));
        assert!(!o.contains_key("C") && !o.contains_key("nope"));
    }
}
