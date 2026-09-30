//! sources::local_sessions — « Ce poste » lu SUR LE DISQUE (ce-poste-vue-locale-lisibilite.md D1/D2/D3).
//!
//! L'app lit elle-même les sources du démon, sans broker ni démon : les fichiers de session de
//! Claude Code (`~/.claude/sessions/<pid>.json`, vivacité par `kill(pid, 0)`) et les fiches du
//! publieur de hooks (`~/.cache/iaka-agents/<sessionId>/<agentId>.json`). Portage Rust de
//! `fleet/src/agent/sessions.js` et `cards.js`, même règle, prouvée par les vecteurs partagés
//! `contract-vectors/local-sessions.json`. Lecture SEULE, parsing tolérant (un fichier illisible
//! est ignoré, jamais fatal), coût O(sessions vivantes) : aucun parcours de `~/.claude/projects`.
//!
//! Règle `D2` : la session fait foi, les fiches la peuplent.
//! 1. une session au pid vivant s'affiche TOUJOURS (plus de péremption à 90 s pour les sessions) ;
//! 2. son coordinateur est la fiche `main` si elle existe, sinon une ligne synthétique tirée du
//!    fichier de session (tâche = nom de session, état = status, projet = dépôt du `cwd`) ;
//! 3. ses sous-agents = fiches du disque UNIES à celles du bus local, dédoublonnées par
//!    (runner, sessionId, agentId), la plus récente (`lastSeenAt`) l'emporte ;
//! 4. session SANS aucune fiche (hooks absents) : repli par transcripts pour CETTE session seule ;
//! 5. fiche orpheline fraîche (sessionId sans session vivante, p. ex. après `/clear`) : « non
//!    rattachée », jamais perdue ; périmée (> péremption) : absente.
//!
//! Les fiches d'un runner autre que Claude Code (bus local) restent leurs propres sessions.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::card::{AgentCard, SUPPORTED_VERSION};
use crate::claude_paths::project_of;
use crate::labels::Labels;
use crate::roster::Roster;
use crate::sources::claude_code_files::RUNNER_ID;
use crate::tree::{build_snapshot_ext, AgentsSnapshot};

/// Variable d'environnement du répertoire de sessions (même nom que le démon `fleet/`).
const SESSIONS_DIR_ENV: &str = "IAKAFLEET_SESSIONS_DIR";

/// Session Claude Code vivante, normalisée depuis `~/.claude/sessions/<pid>.json`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LocalSession {
    pub pid: u32,
    pub session_id: Option<String>,
    /// `idle` · `busy` · `shell`… tel que Claude Code l'écrit.
    pub status: Option<String>,
    /// Instant du dernier changement de statut (epoch s).
    pub status_since: Option<i64>,
    /// Dernière écriture du fichier de session (epoch s) : « actif il y a… » d'un coordinateur
    /// synthétique.
    pub updated_at: Option<i64>,
    /// Session `busy` dont un job est parqué et plus récent que le dernier statut (`sessions.js`).
    pub stuck_suspect: bool,
    pub name: Option<String>,
    pub cwd: Option<String>,
    pub started_at: Option<i64>,
}

fn str_of(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

/// Millisecondes -> secondes entières (`Math.floor(ms / 1000)` de `sessions.js`).
fn secs_of(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(Value::as_f64).filter(|m| m.is_finite()).map(|m| (m / 1000.0).floor() as i64)
}

/// Normalise un fichier de session brut (champs absents => `None`) — même règle que
/// `normalizeSession` de `fleet/src/agent/sessions.js`.
pub fn normalize_session(raw: &Value, fallback_pid: u32) -> LocalSession {
    let pid = raw
        .get("pid")
        .and_then(Value::as_u64)
        .and_then(|p| u32::try_from(p).ok())
        .unwrap_or(fallback_pid);
    let parked = raw.get("parkedJobId").is_some_and(|p| !p.is_null());
    let updated = raw.get("updatedAt").and_then(Value::as_f64);
    let status_updated = raw.get("statusUpdatedAt").and_then(Value::as_f64);
    let stuck_suspect = parked && matches!((updated, status_updated), (Some(u), Some(s)) if u > s);
    LocalSession {
        pid,
        session_id: str_of(raw, "sessionId"),
        status: str_of(raw, "status"),
        status_since: secs_of(raw, "statusUpdatedAt"),
        updated_at: secs_of(raw, "updatedAt"),
        stuck_suspect,
        name: str_of(raw, "name"),
        cwd: str_of(raw, "cwd"),
        started_at: secs_of(raw, "startedAt"),
    }
}

/// Vivacité d'un pid : `kill(pid, 0)` n'envoie aucun signal ; `EPERM` = vivant (le processus
/// existe, il appartient à un autre utilisateur) ; `ESRCH` = mort. Unix seulement (Mac, Linux) :
/// hors périmètre ailleurs (Windows : aucune session n'est alors lue, cf. `CLAUDE.md`).
#[cfg(unix)]
pub fn pid_alive(pid: u32) -> bool {
    // pid 0 signalerait le groupe de processus ; un pid > i32::MAX n'existe pas.
    let Ok(p) = libc::pid_t::try_from(pid) else { return false };
    if p < 1 {
        return false;
    }
    // SAFETY: `kill` avec le signal 0 ne fait que tester l'existence du processus, sans effet.
    if unsafe { libc::kill(p, 0) } == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(not(unix))]
pub fn pid_alive(_pid: u32) -> bool {
    false
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(Into::into)
}

/// `~/.claude/sessions` (ou `IAKAFLEET_SESSIONS_DIR`, comme le démon).
pub fn sessions_dir() -> Option<PathBuf> {
    match std::env::var_os(SESSIONS_DIR_ENV).filter(|v| !v.is_empty()) {
        Some(d) => Some(PathBuf::from(d)),
        None => home_dir().map(|h| h.join(".claude").join("sessions")),
    }
}

/// `~/.cache/iaka-agents` : dossier d'état du publieur de hooks (une fiche par agent).
pub fn cards_dir() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".cache").join("iaka-agents"))
}

/// Lit le répertoire de sessions : uniquement `^\d+\.json$` (les `<pid>.<hash>.key` ne sont jamais
/// lus), fichier illisible ignoré, pid mort écarté. `None` = répertoire illisible (jamais
/// sous-entendu). Triées par pid.
pub fn read_sessions(dir: &Path, alive: &dyn Fn(u32) -> bool) -> Option<Vec<LocalSession>> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(stem) = name.strip_suffix(".json") else { continue };
        if stem.is_empty() || !stem.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let Ok(pid_from_name) = stem.parse::<u32>() else { continue };
        let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
        let Ok(raw) = serde_json::from_str::<Value>(&text) else { continue };
        if !raw.is_object() {
            continue;
        }
        let session = normalize_session(&raw, pid_from_name);
        if alive(session.pid) {
            out.push(session);
        }
    }
    out.sort_by_key(|s| s.pid);
    Some(out)
}

/// Lit les fiches du publieur (`<dossier>/<sessionId>/<agentId>.json` = `{card, …}`). Lecture
/// tolérante : fiche illisible, hors contrat ou sans `startedAt`/`lastSeenAt` ignorée. Le
/// `sessionId` absent d'une fiche est repris du nom de dossier (comme `cards.js`).
pub fn read_disk_cards(dir: &Path) -> Vec<AgentCard> {
    let mut out = Vec::new();
    let Ok(sessions) = std::fs::read_dir(dir) else { return out };
    for sdir in sessions.flatten() {
        let path = sdir.path();
        if !path.is_dir() {
            continue;
        }
        let dir_name = sdir.file_name().to_string_lossy().into_owned();
        let Ok(files) = std::fs::read_dir(&path) else { continue };
        for file in files.flatten() {
            let fp = file.path();
            if fp.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&fp) else { continue };
            let Ok(raw) = serde_json::from_str::<Value>(&text) else { continue };
            let Some(mut card) = raw.get("card").cloned().filter(Value::is_object) else { continue };
            if card.get("sessionId").and_then(Value::as_str).is_none() {
                card["sessionId"] = Value::String(dir_name.clone());
            }
            let Ok(card) = serde_json::from_value::<AgentCard>(card) else { continue };
            if card.v == SUPPORTED_VERSION && !card.runner.is_empty() && !card.agent_id.is_empty() && !card.session_id.is_empty() {
                out.push(card);
            }
        }
    }
    out
}

/// Entrée de l'assemblage : sessions vivantes + fiches du disque + fiches du bus local.
pub struct LocalInput<'a> {
    pub sessions: &'a [LocalSession],
    pub disk_cards: Vec<AgentCard>,
    pub bus_cards: Vec<AgentCard>,
}

/// Résultat : instantané affiché + pids pour lesquels le repli par transcripts a été appelé
/// (preuve que rien d'autre n'a été ouvert).
pub struct LocalOutcome {
    pub snapshot: AgentsSnapshot,
    pub fallback_for: Vec<u32>,
}

/// État d'exécution d'une fiche déduit du statut d'une session (`idle` reste `idle`, tout le reste
/// — `busy`, `shell`… — est de l'activité).
fn state_of_status(status: Option<&str>) -> &'static str {
    if status == Some("idle") {
        "idle"
    } else {
        "running"
    }
}

/// Fiche synthétique du coordinateur d'une session sans fiche `main` (D2.2). Le persona est laissé
/// vide : le cœur (`tree`) applique `main_persona` de la configuration (règle DA-M6).
fn synthetic_coordinator(s: &LocalSession, session_id: &str, now: i64) -> AgentCard {
    let last_seen = s.updated_at.or(s.status_since).or(s.started_at).unwrap_or(now);
    AgentCard {
        v: SUPPORTED_VERSION,
        runner: RUNNER_ID.to_string(),
        runner_version: None,
        model: None,
        persona: None,
        agent_id: "main".to_string(),
        parent_id: None,
        session_id: session_id.to_string(),
        project: s.cwd.as_deref().and_then(project_of),
        cwd: s.cwd.clone(),
        task: s.name.clone().filter(|n| !n.trim().is_empty()),
        estimate: None,
        started_at: s.started_at.unwrap_or(last_seen),
        last_seen_at: last_seen,
        state: Some(state_of_status(s.status.as_deref()).to_string()),
    }
}

/// Union disque + bus, dédoublonnée par (runner, sessionId, agentId) : la plus récente
/// (`lastSeenAt`) l'emporte ; à égalité, le bus (source temps réel).
fn union_cards(disk: Vec<AgentCard>, bus: Vec<AgentCard>) -> Vec<AgentCard> {
    let mut by_key: HashMap<(String, String, String), AgentCard> = HashMap::new();
    let mut order: Vec<(String, String, String)> = Vec::new();
    for c in disk.into_iter().chain(bus) {
        let key = (c.runner.clone(), c.session_id.clone(), c.agent_id.clone());
        match by_key.get(&key) {
            Some(prev) if prev.last_seen_at > c.last_seen_at => {}
            Some(_) => {
                by_key.insert(key, c);
            }
            None => {
                order.push(key.clone());
                by_key.insert(key, c);
            }
        }
    }
    order.into_iter().filter_map(|k| by_key.remove(&k)).collect()
}

/// Règle `D2` de bout en bout, jusqu'à l'instantané affiché. `fallback` = repli par transcripts
/// (D3), appelé pour UNE session sans aucune fiche et jamais ailleurs.
pub fn build_local_snapshot(
    input: LocalInput<'_>,
    fallback: &mut dyn FnMut(&LocalSession) -> Vec<AgentCard>,
    now: i64,
    liveness_secs: i64,
    labels: &Labels,
    roster: &Roster,
) -> LocalOutcome {
    let all = union_cards(input.disk_cards, input.bus_cards);
    let live_sids: HashSet<&str> = input.sessions.iter().filter_map(|s| s.session_id.as_deref()).collect();

    // Fiches d'un runner tiers : leurs propres sessions, sous la péremption habituelle.
    // Fiches Claude Code d'une session vivante : regroupées par sessionId. Les autres : orphelines.
    let mut by_session: HashMap<&str, Vec<&AgentCard>> = HashMap::new();
    let mut cards: Vec<AgentCard> = Vec::new();
    let mut orphans: Vec<AgentCard> = Vec::new();
    for c in &all {
        if c.runner != RUNNER_ID {
            cards.push(c.clone());
        } else if live_sids.contains(c.session_id.as_str()) {
            by_session.entry(c.session_id.as_str()).or_default().push(c);
        } else if now - c.last_seen_at <= liveness_secs {
            orphans.push(c.clone());
        }
    }

    let mut pinned: HashSet<(String, String)> = HashSet::new();
    let mut fallback_for: Vec<u32> = Vec::new();
    for s in input.sessions {
        let sid = s.session_id.clone().unwrap_or_else(|| format!("pid-{}", s.pid));
        let mut session_cards: Vec<AgentCard> =
            by_session.get(sid.as_str()).map(|v| v.iter().map(|c| (*c).clone()).collect()).unwrap_or_default();
        if session_cards.is_empty() {
            // D2.4 : mode dégradé sans hooks — repli par transcripts, pour CETTE session seule.
            fallback_for.push(s.pid);
            session_cards = fallback(s)
                .into_iter()
                .map(|mut c| {
                    c.session_id = sid.clone();
                    c
                })
                .collect();
        }
        if !session_cards.iter().any(|c| c.agent_id == "main") {
            session_cards.push(synthetic_coordinator(s, &sid, now));
        }
        pinned.insert((RUNNER_ID.to_string(), sid));
        cards.extend(session_cards);
    }

    let snapshot = build_snapshot_ext(&cards, &pinned, &orphans, now, liveness_secs, labels, roster);
    LocalOutcome { snapshot, fallback_for }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("iaka-am-local-{name}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn pid_alive_soi_meme_vivant_pid_invalide_ou_inexistant_mort() {
        assert!(pid_alive(std::process::id()));
        assert!(!pid_alive(0));
        assert!(!pid_alive(u32::MAX));
        // Un pid très élevé mais valide pour kill(2) : quasi certainement libre.
        assert!(!pid_alive(i32::MAX as u32));
    }

    #[test]
    fn read_sessions_ne_lit_que_les_fichiers_pid_json_et_ecarte_les_morts() {
        let dir = tmp_dir("sessions");
        fs::write(dir.join("1001.json"), r#"{"sessionId":"a","status":"idle","name":"n"}"#).unwrap();
        fs::write(dir.join("1002.json"), r#"{"sessionId":"b"}"#).unwrap(); // pid mort (injecté)
        fs::write(dir.join("1003.json"), "pas du json").unwrap(); // illisible : ignoré
        fs::write(dir.join("1004.json"), "[1,2]").unwrap(); // pas un objet : ignoré
        fs::write(dir.join("1001.abcdef.key"), "secret").unwrap(); // jamais lu
        fs::write(dir.join("notes.json"), "{}").unwrap();
        let got = read_sessions(&dir, &|pid| pid != 1002).unwrap();
        assert_eq!(got.iter().map(|s| s.pid).collect::<Vec<_>>(), vec![1001]);
        assert_eq!(got[0].name.as_deref(), Some("n"));
        assert!(read_sessions(&dir.join("absent"), &|_| true).is_none(), "dossier illisible = None, pas un vide");
    }

    #[test]
    fn read_disk_cards_format_du_publieur_et_tolerance() {
        let dir = tmp_dir("cards");
        let s1 = dir.join("sid-1");
        fs::create_dir_all(&s1).unwrap();
        fs::write(
            s1.join("main.json"),
            r#"{"card":{"v":1,"runner":"claude-code","agentId":"main","parentId":null,"sessionId":"sid-1","startedAt":10,"lastSeenAt":20,"state":"idle"},"lastPublishedAt":20}"#,
        )
        .unwrap();
        // sessionId absent de la fiche : repris du dossier ; fiche hors contrat (v=2) ignorée.
        fs::write(s1.join("a1.json"), r#"{"card":{"v":1,"runner":"claude-code","agentId":"a1","parentId":"main","startedAt":11,"lastSeenAt":21}}"#).unwrap();
        fs::write(s1.join("v2.json"), r#"{"card":{"v":2,"runner":"x","agentId":"v2","sessionId":"sid-1","startedAt":1,"lastSeenAt":1}}"#).unwrap();
        fs::write(s1.join("bad.json"), "{").unwrap();
        fs::write(s1.join("readme.txt"), "x").unwrap();
        let mut cards = read_disk_cards(&dir);
        cards.sort_by(|a, b| a.agent_id.cmp(&b.agent_id));
        assert_eq!(cards.iter().map(|c| c.agent_id.as_str()).collect::<Vec<_>>(), vec!["a1", "main"]);
        assert_eq!(cards[0].session_id, "sid-1");
        assert!(read_disk_cards(&dir.join("absent")).is_empty());
    }

    #[test]
    fn repli_par_session_n_ouvre_aucun_transcript_hors_des_sessions_vivantes() {
        // CA-5 : répertoire témoin contenant des transcripts MORTS ; la session vivante n'a pas de
        // fiche ; seul SON transcript est lu, les autres ne sont jamais ouverts.
        let projects = tmp_dir("projects");
        let cwd = "/Users/x/work/vivant";
        let escaped = projects.join("-Users-x-work-vivant");
        fs::create_dir_all(&escaped).unwrap();
        fs::write(
            escaped.join("sid-live.jsonl"),
            format!("{{\"cwd\":\"{cwd}\"}}\n{{\"type\":\"user\",\"message\":{{\"content\":\"tache vivante\"}}}}\n"),
        )
        .unwrap();
        let dead = projects.join("-Users-x-work-mort");
        fs::create_dir_all(&dead).unwrap();
        // Un transcript de session morte : il existe, mais le repli ne doit jamais y toucher (la
        // sonde `opened` ci-dessous ne doit voir QUE la session vivante sans fiche).
        fs::write(dead.join("sid-dead.jsonl"), "{\"cwd\":\"/Users/x/work/mort\"}\n").unwrap();

        let sessions = vec![LocalSession {
            pid: 4242,
            session_id: Some("sid-live".into()),
            cwd: Some(cwd.into()),
            status: Some("busy".into()),
            ..Default::default()
        }];
        let mut opened: Vec<String> = Vec::new();
        let mut fb = |s: &LocalSession| {
            opened.push(s.session_id.clone().unwrap());
            crate::sources::claude_code_files::cards_for_session(&projects, s.cwd.as_deref().unwrap(), s.session_id.as_deref().unwrap())
        };
        let labels = Labels::default();
        let out = build_local_snapshot(
            LocalInput { sessions: &sessions, disk_cards: vec![], bus_cards: vec![] },
            &mut fb,
            now_secs(),
            90,
            &labels,
            &Roster::new(),
        );
        assert_eq!(opened, vec!["sid-live".to_string()], "le repli n'est appelé que pour la session vivante sans fiche");
        assert_eq!(out.snapshot.sessions.len(), 1);
        assert_eq!(out.snapshot.sessions[0].coordinator.task, "tache vivante", "tâche tirée du transcript de CETTE session");
    }

    fn now_secs() -> i64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
    }
}
