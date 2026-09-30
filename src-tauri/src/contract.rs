//! contract — le contrat de bus `iaka/` v2 côté Rust (`specs/contrat-bus-iaka.md`) : normalisation
//! de `<node>`, construction et décomposition des topics. Module NEUTRE (aucun nom de runner).
//!
//! Même règle que `fleet/src/shared/contract.js` et `publishers/claude-code/iaka-agents-publish.mjs`,
//! prouvée par les vecteurs partagés `contract-vectors/` (`tests/vectors.rs`). Hors couverture
//! DÉCLARÉE : le hachage du `<sid>` — l'app compare des `<sid>` déjà hachés par le démon, elle n'en
//! calcule aucun.

/// Racine du portefeuille.
pub const ROOT: &str = "iaka";

/// Abonnement d'une machine qui lit toute la flotte (`iaka/+/out/#`) — jamais `in/`.
pub const ALL_OUT: &str = "iaka/+/out/#";

/// Refus nommé d'un `<node>` (vide, ou contenant `+ # /`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeError(pub String);

impl std::fmt::Display for NodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Un segment de topic valide : `[A-Za-z0-9._-]{1,128}`, jamais `+`, `#` ni `/`.
pub fn is_valid_segment(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// § 1 : forme normalisée du `<node>` — minuscules, suffixe `.local` retiré, tout caractère hors
/// `[a-z0-9-]` remplacé par `-` ; vide (ou que des tirets) ou contenant `+ # /` ⇒ refus.
pub fn normalize_node(raw: &str) -> Result<String, NodeError> {
    if raw.contains(['+', '#', '/']) {
        return Err(NodeError(format!("node invalide : \"{raw}\" contient +, # ou /")));
    }
    let lower = raw.trim().to_lowercase();
    let base = lower.strip_suffix(".local").unwrap_or(&lower);
    let node: String = base
        .chars()
        .map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' { c } else { '-' })
        .collect();
    if node.is_empty() || node.chars().all(|c| c == '-') {
        return Err(NodeError(format!("node invalide : \"{raw}\" donne une valeur vide")));
    }
    Ok(node)
}

/// `<node>` de CETTE machine : `IAKA_NODE` (alias `IAKAFLEET_HOST`) fourni par l'appelant, sinon le
/// nom d'hôte du système. `None` si la valeur est refusée : l'app n'invente pas d'identité.
pub fn resolve_node(configured: Option<&str>) -> Option<String> {
    let raw = match configured {
        Some(c) if !c.is_empty() => c.to_string(),
        _ => system_hostname()?,
    };
    normalize_node(&raw).ok()
}

/// Nom d'hôte du système sans nouvelle dépendance ni processus lancé : `gethostname(3)` (libc,
/// déjà liée par la bibliothèque standard) sous unix, variable `COMPUTERNAME` sinon.
#[cfg(unix)]
fn system_hostname() -> Option<String> {
    extern "C" {
        fn gethostname(name: *mut std::os::raw::c_char, len: usize) -> std::os::raw::c_int;
    }
    let mut buf = vec![0u8; 256];
    // SAFETY : `buf` est un tampon valide de 256 octets ; `gethostname` n'écrit pas au-delà de
    // `len` et termine la chaîne par NUL sauf troncature, que l'on borne ci-dessous.
    let rc = unsafe { gethostname(buf.as_mut_ptr().cast(), buf.len()) };
    if rc != 0 {
        return None;
    }
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8(buf[..end].to_vec()).ok().filter(|s| !s.is_empty())
}

#[cfg(not(unix))]
fn system_hostname() -> Option<String> {
    std::env::var("COMPUTERNAME").ok().filter(|s| !s.is_empty())
}

/// `iaka/<node>/out/agents/<runner>/<sid>/<agentId>`.
pub fn agent_topic(node: &str, runner: &str, sid: &str, agent_id: &str) -> String {
    format!("{ROOT}/{node}/out/agents/{runner}/{sid}/{agent_id}")
}

/// Filtre d'abonnement du bus LOCAL : `iaka/<node>/out/agents/+/+/+`.
pub fn agents_filter(node: &str) -> String {
    format!("{ROOT}/{node}/out/agents/+/+/+")
}

/// Un topic du contrat, décomposé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Topic {
    Presence { node: String, code: String },
    Session { node: String, pid: u32, code: String },
    Agent { node: String, runner: String, sid: String, agent_id: String },
}

/// Décompose un topic ; `None` s'il ne respecte pas le contrat (jamais de panique).
pub fn parse_topic(topic: &str) -> Option<Topic> {
    let p: Vec<&str> = topic.split('/').collect();
    if p.len() < 6 || p[0] != ROOT || p[2] != "out" || !is_valid_segment(p[1]) {
        return None;
    }
    let node = p[1].to_string();
    match (p[3], p.len()) {
        ("presence", 6) if p[5] == "current" => {
            Some(Topic::Presence { node, code: p[4].to_string() })
        }
        ("sessions", 7) if p[6] == "current" && !p[4].is_empty() && p[4].chars().all(|c| c.is_ascii_digit()) => {
            let pid = p[4].parse().ok()?;
            Some(Topic::Session { node, pid, code: p[5].to_string() })
        }
        ("agents", 7) if [p[4], p[5], p[6]].iter().all(|s| is_valid_segment(s)) => Some(Topic::Agent {
            node,
            runner: p[4].to_string(),
            sid: p[5].to_string(),
            agent_id: p[6].to_string(),
        }),
        _ => None,
    }
}
