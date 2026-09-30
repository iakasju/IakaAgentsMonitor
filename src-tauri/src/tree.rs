//! tree — cœur NEUTRE de l'arbre de délégation, construit à partir de fiches (`AgentCard`) plutôt
//! que de fichiers (specs/instructions/agnostique-mqtt.md, étape 4). Aucun nom de runner ni de
//! fournisseur de modèle ici : les libellés viennent de `labels::Labels`, les personas de
//! `roster::Roster` — ce module ne fait que combiner ce que les sources lui apportent.
//!
//! Reprend les règles d'affichage de l'ancien `agents.rs` (transposées sur des fiches, mêmes
//! bornes) : remontée d'ancêtre mort jusqu'au premier ancêtre VIVANT, bornes 6 sessions / 8
//! enfants avec marqueur `+N`, tri par activité décroissante entre sessions et par date de début
//! croissante entre enfants.

use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::card::AgentCard;
use crate::estimate;
use crate::labels::Labels;
use crate::roster::{expand_home, PersonaInfo, Roster};

/// Bornes d'affichage : au-delà, un marqueur « +N » (identiques à liste-des-agents.md).
pub const MAX_SESSIONS: usize = 6;
pub const MAX_CHILDREN: usize = 8;

const FALLBACK_BG: &str = "#6F6F78";
const FALLBACK_FG: &str = "#FFFFFF";

/// Nombre de teintes de la palette de dépôt (amendement A7, point 3 de liste-des-agents.md).
pub const GROUP_COLOR_COUNT: u8 = 6;

/// Sprite résolu d'un agent : lettre + couleurs (fond/texte), déjà prêtes à peindre. Les couleurs
/// viennent du fichier de configuration (`labels::Labels`), plus de table figée dans le code.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sprite {
    pub letter: char,
    pub bg: String,
    pub fg: String,
}

fn fallback_sprite(letter: char) -> Sprite {
    Sprite {
        letter,
        bg: FALLBACK_BG.to_string(),
        fg: FALLBACK_FG.to_string(),
    }
}

fn titlecase(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn first_letter_upper(s: &str) -> char {
    s.chars()
        .next()
        .map(|c| c.to_ascii_uppercase())
        .unwrap_or('?')
}

/// Persona résolu, prêt pour l'affichage : sprite + nom + clé (pour l'infobulle, en minuscule).
struct ResolvedPersona {
    sprite: Sprite,
    display_name: String,
    key: String,
}

/// Résout un persona (`agentType`) contre le roster de l'équipe active + les couleurs du fichier
/// de configuration. Persona absent -> « Inconnu » gris. Persona hors équipe (pas dans le
/// roster) -> gris + sa lettre (imposée ou première lettre), jamais un faux persona de la frame.
fn resolve_persona(
    persona_id: Option<&str>,
    roster_map: &HashMap<String, PersonaInfo>,
    labels: &Labels,
) -> ResolvedPersona {
    let Some(id) = persona_id.filter(|s| !s.is_empty()) else {
        return ResolvedPersona {
            sprite: fallback_sprite('?'),
            display_name: "Inconnu".to_string(),
            key: "inconnu".to_string(),
        };
    };
    let lower = id.to_ascii_lowercase();
    if let Some(info) = roster_map.get(&lower) {
        let letter = labels
            .forced_letter(&lower)
            .unwrap_or_else(|| first_letter_upper(&info.name));
        let (bg, fg) = info
            .pastille
            .as_deref()
            .and_then(|p| labels.pastille_color(p))
            .map(|c| (c.bg.clone(), c.fg.clone()))
            .unwrap_or_else(|| (FALLBACK_BG.to_string(), FALLBACK_FG.to_string()));
        ResolvedPersona {
            sprite: Sprite { letter, bg, fg },
            display_name: info.name.clone(),
            key: lower,
        }
    } else {
        let letter = labels
            .forced_letter(&lower)
            .unwrap_or_else(|| first_letter_upper(&lower));
        ResolvedPersona {
            sprite: fallback_sprite(letter),
            display_name: titlecase(&lower),
            key: lower,
        }
    }
}

// ============================ Palette de dépôt ============================

/// Hash FNV-1a 32 bits — déterministe et stable d'une exécution à l'autre.
fn fnv1a(s: &str) -> u32 {
    const OFFSET_BASIS: u32 = 0x811c_9dc5;
    const PRIME: u32 = 0x0100_0193;
    let mut hash = OFFSET_BASIS;
    for byte in s.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// Index de couleur stable pour un nom de dépôt : même nom -> même index, toujours.
pub fn group_color_index(project: &str) -> u8 {
    (fnv1a(project) % u32::from(GROUP_COLOR_COUNT)) as u8
}

/// Dernier segment non vide d'un `cwd` (déduction du projet quand la fiche ne le porte pas,
/// contrat § « Fiche », champ `project`). Duplication volontaire d'une fonction équivalente déjà
/// écrite pour un adaptateur runner-spécifique : ce module ne doit importer AUCUN symbole d'un
/// module réservé à un runner (grille de neutralité de l'étape 12).
fn project_from_cwd(cwd: &str) -> Option<String> {
    cwd.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .find(|s| !s.is_empty())
        .map(str::to_string)
}

// ============================ Nœuds d'affichage ============================

/// Nœud d'agent déjà résolu pour l'affichage (miroir de l'ancien `agents::AgentNode`, + `runner`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentNode {
    pub sprite: Sprite,
    pub persona: String,
    pub task: String,
    pub active_ago: String,
    pub active_secs: i64,
    pub started_ago: String,
    pub started_secs: Option<i64>,
    pub model: String,
    /// Libellé du runner (résolu via `labels::Labels::runner_label`), amendement de l'étape 8.
    pub runner: String,
    /// État d'exécution de l'agent (`running` | `idle`, contrat § Fiche champ `state`) ; vide pour
    /// le marqueur de débordement. Sert à la phrase d'état de « Ce poste » (agents actifs).
    pub state: String,
    pub estimate: Option<estimate::EstimateView>,
    pub tooltip: String,
    pub children: Vec<AgentNode>,
    pub overflow: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionNode {
    pub project: String,
    pub coordinator: AgentNode,
    pub group_color: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentsSnapshot {
    pub count: u32,
    pub sessions: Vec<SessionNode>,
    pub overflow_sessions: Option<u32>,
    /// Agents dont la session n'est pas (ou plus) vivante mais dont la fiche est fraîche (D2.5 de
    /// `ce-poste-vue-locale-lisibilite.md`) : « agents non rattachés », jamais perdus.
    pub unattached: Vec<AgentNode>,
}

pub(crate) fn format_duration_secs(secs: i64) -> String {
    let secs = secs.max(0);
    if secs < 60 {
        return format!("{secs} s");
    }
    let minutes = secs / 60;
    if minutes < 60 {
        return format!("{minutes} min");
    }
    let hours = minutes / 60;
    let rem_min = minutes % 60;
    if rem_min == 0 {
        format!("{hours} h")
    } else {
        format!("{hours} h {rem_min} min")
    }
}

fn card_alive(card: &AgentCard, now: i64, liveness_secs: i64) -> bool {
    now - card.last_seen_at <= liveness_secs
}

/// Parent d'affichage NORMALISÉ d'une fiche : `None` = coordinateur (`parentId` absent ou égal à
/// `"main"`, contrat § Fiche champ `parentId`), sinon l'`agentId` du parent.
fn normalized_parent(card: &AgentCard) -> Option<String> {
    match card.parent_id.as_deref() {
        None | Some("main") => None,
        Some(pid) => Some(pid.to_string()),
    }
}

/// Construit l'instantané complet à partir de TOUTES les fiches actuellement connues (peu
/// importe leur source, bus ou fichiers — ce module ne le sait pas). `now`/`liveness_secs`
/// explicites -> testable sans horloge murale.
pub fn build_snapshot(
    cards: &[AgentCard],
    now: i64,
    liveness_secs: i64,
    labels: &Labels,
    roster: &Roster,
) -> AgentsSnapshot {
    build_snapshot_ext(cards, &HashSet::new(), &[], now, liveness_secs, labels, roster)
}

/// Comme [`build_snapshot`], avec deux apports de « Ce poste » (ce-poste-vue-locale-lisibilite.md
/// D2) :
/// - `pinned` : sessions `(runner, sessionId)` dont on SAIT qu'elles vivent (pid vivant) — elles
///   s'affichent TOUJOURS, même sans écriture récente (plus de péremption à 90 s pour une
///   session ; celle des sous-agents reste inchangée) ;
/// - `orphans` : fiches fraîches dont la session n'est pas vivante, rendues à plat sous
///   `unattached` (« agents non rattachés », jamais perdus).
pub fn build_snapshot_ext(
    cards: &[AgentCard],
    pinned: &HashSet<(String, String)>,
    orphans: &[AgentCard],
    now: i64,
    liveness_secs: i64,
    labels: &Labels,
    roster: &Roster,
) -> AgentsSnapshot {
    let iakaframe_root = expand_home(&labels.iakaframe_root);

    // Regroupement par session (runner, sessionId) — une session peut porter un coordinateur
    // (agentId "main") et des sous-agents.
    let mut sessions: HashMap<(String, String), Vec<&AgentCard>> = HashMap::new();
    for card in cards {
        sessions
            .entry((card.runner.clone(), card.session_id.clone()))
            .or_default()
            .push(card);
    }

    let mut with_activity: Vec<(i64, SessionNode)> = Vec::new();
    let mut count: u32 = 0;

    for (key, group) in &sessions {
        let Some((node, activity)) = build_session_node(
            group,
            pinned.contains(key),
            now,
            liveness_secs,
            labels,
            roster,
            &iakaframe_root,
        ) else {
            continue;
        };
        count += count_live_agents(&node.coordinator);
        with_activity.push((activity, node));
    }

    with_activity.sort_by_key(|(activity, node)| (std::cmp::Reverse(*activity), node.project.clone()));

    let overflow = with_activity.len().saturating_sub(MAX_SESSIONS);
    let sessions: Vec<SessionNode> = with_activity
        .into_iter()
        .take(MAX_SESSIONS)
        .map(|(_, n)| n)
        .collect();

    let unattached = build_unattached(orphans, now, labels, roster, &iakaframe_root);
    count += unattached.iter().map(|n| n.overflow.unwrap_or(1)).sum::<u32>();

    AgentsSnapshot {
        count,
        sessions,
        overflow_sessions: if overflow > 0 {
            Some(overflow as u32)
        } else {
            None
        },
        unattached,
    }
}

/// Rend les fiches orphelines à plat (aucune hiérarchie : leur parent n'est plus là), du plus
/// ancien démarrage au plus récent, avec les bornes d'affichage habituelles. L'identité d'une
/// fiche orpheline est `<sessionId>/<agentId>` (deux sessions mortes peuvent porter chacune un
/// `main`) ; un `main` orphelin reprend le persona du coordinateur de la configuration.
fn build_unattached(
    orphans: &[AgentCard],
    now: i64,
    labels: &Labels,
    roster: &Roster,
    iakaframe_root: &std::path::Path,
) -> Vec<AgentNode> {
    if orphans.is_empty() {
        return Vec::new();
    }
    let flat: Vec<AgentCard> = orphans
        .iter()
        .map(|c| {
            let mut c = c.clone();
            if c.agent_id == "main" && c.persona.is_none() {
                c.persona = Some(labels.main_persona.clone());
            }
            c.agent_id = format!("{}/{}", c.session_id, c.agent_id);
            c.parent_id = None;
            c
        })
        .collect();
    let by_id: HashMap<&str, &AgentCard> = flat.iter().map(|c| (c.agent_id.as_str(), c)).collect();
    let display_parent: HashMap<&str, Option<String>> = flat.iter().map(|c| (c.agent_id.as_str(), None)).collect();
    let mut children_of: HashMap<Option<String>, Vec<&str>> = HashMap::new();
    children_of.insert(None, flat.iter().map(|c| c.agent_id.as_str()).collect());
    let reference_cwd = flat.iter().find_map(|c| c.cwd.as_deref());
    let roster_map = roster.resolve(reference_cwd, iakaframe_root);
    let runner_label = flat.first().map(|c| labels.runner_label(&c.runner)).unwrap_or_default();
    build_children(None, &children_of, &by_id, &display_parent, now, labels, &roster_map, &runner_label)
}

fn count_live_agents(node: &AgentNode) -> u32 {
    let mut total = 1;
    for child in &node.children {
        total += child.overflow.unwrap_or_else(|| count_live_agents(child));
    }
    total
}

fn build_session_node(
    group: &[&AgentCard],
    pinned: bool,
    now: i64,
    liveness_secs: i64,
    labels: &Labels,
    roster: &Roster,
    iakaframe_root: &std::path::Path,
) -> Option<(SessionNode, i64)> {
    let coordinator_card = group.iter().find(|c| c.agent_id == "main").copied();
    let subagents: Vec<&AgentCard> = group.iter().filter(|c| c.agent_id != "main").copied().collect();

    let coordinator_alive = coordinator_card.is_some_and(|c| card_alive(c, now, liveness_secs));
    let any_descendant_alive = subagents.iter().any(|c| card_alive(c, now, liveness_secs));
    if !coordinator_alive && !any_descendant_alive && !pinned {
        return None;
    }

    let by_id: HashMap<&str, &AgentCard> = subagents.iter().map(|c| (c.agent_id.as_str(), *c)).collect();

    let mut display_parent: HashMap<&str, Option<String>> = HashMap::new();
    for card in subagents.iter().filter(|c| card_alive(c, now, liveness_secs)) {
        let mut current = normalized_parent(card);
        let mut seen: HashSet<String> = HashSet::new();
        let mut found: Option<String> = None;
        while let Some(pid) = current {
            if !seen.insert(pid.clone()) {
                break; // cycle défensif -> rattaché au coordinateur.
            }
            match by_id.get(pid.as_str()) {
                Some(p) if card_alive(p, now, liveness_secs) => {
                    found = Some(pid);
                    break;
                }
                Some(p) => current = normalized_parent(p),
                None => break,
            }
        }
        display_parent.insert(card.agent_id.as_str(), found);
    }

    let mut children_of: HashMap<Option<String>, Vec<&str>> = HashMap::new();
    for card in subagents.iter().filter(|c| card_alive(c, now, liveness_secs)) {
        let parent = display_parent.get(card.agent_id.as_str()).cloned().flatten();
        children_of.entry(parent).or_default().push(card.agent_id.as_str());
    }

    // cwd de référence pour le projet et l'équipe active : coordinateur en priorité, sinon le
    // premier sous-agent qui en porte un.
    let reference_cwd = coordinator_card
        .and_then(|c| c.cwd.as_deref())
        .or_else(|| subagents.iter().find_map(|c| c.cwd.as_deref()));
    let roster_map = roster.resolve(reference_cwd, iakaframe_root);

    let runner_label = coordinator_card
        .map(|c| c.runner.as_str())
        .or_else(|| subagents.first().map(|c| c.runner.as_str()))
        .map(|r| labels.runner_label(r))
        .unwrap_or_default();

    let coordinator_children = build_children(None, &children_of, &by_id, &display_parent, now, labels, &roster_map, &runner_label);

    let (coordinator, coordinator_activity) = match coordinator_card {
        Some(card) => {
            let elapsed = (now - card.last_seen_at).max(0);
            let started_secs = (now - card.started_at).max(0);
            let (task_display, _) = estimate::resolve(&estimate::DelegationText {
                consigne: None,
                short_label: Some(card.task.clone().unwrap_or_default()),
            });
            let task_display = task_display.unwrap_or_default();
            let task = if task_display.trim().is_empty() {
                "coordinateur".to_string()
            } else {
                task_display
            };
            let resolved = resolve_persona(Some(&labels.main_persona), &roster_map, labels);
            // État du coordinateur (contrat § Fiche, champ `state`) : "idle" entre deux tours,
            // signalé dans l'infobulle — les boutons de contrôle (pause/reprise/arrêt) restent un
            // sous-lot séparé (amendement A1, non livré ici), cette mention est purement
            // informative.
            let state = card.state_or_default();
            let state_suffix = if state == "idle" { " (en attente)" } else { "" };
            (
                AgentNode {
                    sprite: resolved.sprite,
                    persona: resolved.display_name,
                    task,
                    active_ago: format_duration_secs(elapsed),
                    active_secs: elapsed,
                    started_ago: format_duration_secs(started_secs),
                    started_secs: Some(started_secs),
                    model: card.model.as_deref().map(|m| labels.model_label(m)).unwrap_or_else(|| "—".to_string()),
                    runner: runner_label.clone(),
                    state: state.to_string(),
                    estimate: None, // DA-T6 : la session principale n'a jamais d'estimation.
                    tooltip: format!(
                        "{} (coordinateur){state_suffix} — actif il y a {elapsed} s",
                        resolved.key
                    ),
                    children: coordinator_children,
                    overflow: None,
                },
                card.last_seen_at,
            )
        }
        None => {
            // Coordinateur mort mais descendants vivants (remontée) : racine synthétique, comme
            // l'ancien comportement où le coordinateur froid restait affiché tant qu'un
            // descendant l'était.
            let resolved = resolve_persona(Some(&labels.main_persona), &roster_map, labels);
            let activity = subagents
                .iter()
                .map(|c| c.last_seen_at)
                .max()
                .unwrap_or(i64::MIN);
            (
                AgentNode {
                    sprite: resolved.sprite,
                    persona: resolved.display_name,
                    task: "coordinateur".to_string(),
                    active_ago: String::new(),
                    active_secs: 0,
                    started_ago: String::new(),
                    started_secs: None,
                    model: "—".to_string(),
                    runner: runner_label.clone(),
                    state: "idle".to_string(),
                    estimate: None,
                    tooltip: format!("{} (coordinateur, terminé)", resolved.key),
                    children: coordinator_children,
                    overflow: None,
                },
                activity,
            )
        }
    };

    // Priorité (contrat § Fiche, champ `project`) : champ explicite d'abord, déduction du `cwd`
    // en repli.
    let project = coordinator_card
        .and_then(|c| c.project.clone())
        .or_else(|| subagents.first().and_then(|c| c.project.clone()))
        .or_else(|| reference_cwd.and_then(project_from_cwd))
        .unwrap_or_else(|| "—".to_string());
    let group_color = group_color_index(&project);

    Some((
        SessionNode {
            project,
            coordinator,
            group_color,
        },
        coordinator_activity,
    ))
}

#[allow(clippy::too_many_arguments)]
fn build_children(
    parent: Option<String>,
    children_of: &HashMap<Option<String>, Vec<&str>>,
    by_id: &HashMap<&str, &AgentCard>,
    display_parent: &HashMap<&str, Option<String>>,
    now: i64,
    labels: &Labels,
    roster_map: &HashMap<String, PersonaInfo>,
    runner_label: &str,
) -> Vec<AgentNode> {
    let Some(ids) = children_of.get(&parent) else {
        return Vec::new();
    };
    let mut kids: Vec<&AgentCard> = ids.iter().filter_map(|id| by_id.get(*id).copied()).collect();
    kids.sort_by(|a, b| a.started_at.cmp(&b.started_at).then_with(|| a.agent_id.cmp(&b.agent_id)));

    let overflow = kids.len().saturating_sub(MAX_CHILDREN);
    let mut nodes: Vec<AgentNode> = kids
        .into_iter()
        .take(MAX_CHILDREN)
        .map(|card| {
            let resolved = resolve_persona(card.persona.as_deref(), roster_map, labels);
            let elapsed = (now - card.last_seen_at).max(0);
            let started_secs = (now - card.started_at).max(0);
            let (task_display, _) = estimate::resolve(&estimate::DelegationText {
                consigne: None,
                short_label: Some(card.task.clone().unwrap_or_default()),
            });
            let task_display = task_display.unwrap_or_default();

            let mut tooltip = if task_display.is_empty() {
                format!("{} — actif il y a {elapsed} s", resolved.key)
            } else {
                format!("{} — {task_display} — actif il y a {elapsed} s", resolved.key)
            };

            let display_p = display_parent.get(card.agent_id.as_str()).cloned().flatten();
            if normalized_parent(card) != display_p {
                let real = normalized_parent(card)
                    .as_deref()
                    .and_then(|pid| by_id.get(pid))
                    .map(|p| p.persona.clone().unwrap_or_else(|| "inconnu".to_string()))
                    .unwrap_or_else(|| "coordinateur".to_string());
                tooltip.push_str(&format!(" (parent réel : {real}, terminé)"));
            }

            let estimate_view = card
                .estimate
                .as_deref()
                .and_then(estimate::parse_value)
                .map(|est| {
                    let view = estimate::estimate_view(&est, started_secs);
                    let announced = estimate::format_estimate_announced(&est);
                    if view.remaining_secs < 0 {
                        tooltip.push_str(&format!(" — estimé {announced}, dépassement {}", view.short_label));
                    } else {
                        tooltip.push_str(&format!(" — estimé {announced}, reste {}", view.short_label));
                    }
                    view
                });

            let children = build_children(
                Some(card.agent_id.clone()),
                children_of,
                by_id,
                display_parent,
                now,
                labels,
                roster_map,
                runner_label,
            );

            AgentNode {
                sprite: resolved.sprite,
                persona: resolved.display_name,
                task: task_display,
                active_ago: format_duration_secs(elapsed),
                active_secs: elapsed,
                started_ago: format_duration_secs(started_secs),
                started_secs: Some(started_secs),
                model: card.model.as_deref().map(|m| labels.model_label(m)).unwrap_or_else(|| "—".to_string()),
                runner: runner_label.to_string(),
                state: card.state_or_default().to_string(),
                estimate: estimate_view,
                tooltip,
                children,
                overflow: None,
            }
        })
        .collect();

    if overflow > 0 {
        nodes.push(AgentNode {
            sprite: fallback_sprite('+'),
            persona: "+".to_string(),
            task: String::new(),
            active_ago: String::new(),
            active_secs: 0,
            started_ago: String::new(),
            started_secs: None,
            model: String::new(),
            runner: String::new(),
            state: String::new(),
            estimate: None,
            tooltip: format!("+{overflow} agent(s) supplémentaire(s)"),
            children: Vec::new(),
            overflow: Some(overflow as u32),
        });
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("iaka-am-tree-{name}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_reservoir(root: &std::path::Path) {
        fs::create_dir_all(root.join("frames")).unwrap();
        fs::create_dir_all(root.join("teams")).unwrap();
        fs::create_dir_all(root.join("library/personas")).unwrap();
        fs::write(root.join("frames/iakaframe.md"), "teamId: t\n").unwrap();
        fs::write(root.join("teams/t.md"), "personas: [odin, gimli]\n").unwrap();
        fs::write(
            root.join("library/personas/odin.md"),
            "---\nname: Odin\npastille: \"🟡\"\n---\n",
        )
        .unwrap();
        fs::write(
            root.join("library/personas/gimli.md"),
            "---\nname: Gimli\npastille: \"🔴\"\n---\n",
        )
        .unwrap();
    }

    fn labels_with_root(root: &std::path::Path) -> Labels {
        let toml = format!(
            r##"
main_persona = "odin"
iakaframe_root = "{}"
[runners]
"essai" = "Essai"
[[models]]
prefix = "acme-x"
label = "X"
[pastilles."🔴"]
bg = "#FF3B30"
fg = "#000000"
[pastilles."🟡"]
bg = "#FFD60A"
fg = "#000000"
[letters]
gimli = "I"
"##,
            root.to_str().unwrap()
        );
        crate::labels::parse_labels_toml(&toml).unwrap()
    }

    fn card(agent_id: &str, parent_id: Option<&str>, persona: Option<&str>, last_seen: i64) -> AgentCard {
        AgentCard {
            v: 1,
            runner: "essai".to_string(),
            runner_version: None,
            model: Some("acme-x-5-5".to_string()),
            persona: persona.map(str::to_string),
            agent_id: agent_id.to_string(),
            parent_id: parent_id.map(str::to_string),
            session_id: "sess-1".to_string(),
            project: None,
            cwd: Some("/Users/x/work/demo".to_string()),
            task: Some("Une tache".to_string()),
            estimate: None,
            started_at: 1_000,
            last_seen_at: last_seen,
            state: Some("running".to_string()),
        }
    }

    #[test]
    fn session_absente_si_rien_de_vivant() {
        let root = tmp_root("absente");
        make_reservoir(&root);
        let labels = labels_with_root(&root);
        let roster = Roster::new();
        let cards = vec![card("main", None, None, 0)]; // last_seen très ancien.
        let snap = build_snapshot(&cards, 1_000_000, 90, &labels, &roster);
        assert!(snap.sessions.is_empty());
        assert_eq!(snap.count, 0);
    }

    #[test]
    fn liveness_bornes_0_89_90_91() {
        let root = tmp_root("liveness");
        make_reservoir(&root);
        let labels = labels_with_root(&root);
        let roster = Roster::new();
        let now = 1_000_100;
        for (delta, expect_alive) in [(0i64, true), (89, true), (90, true), (91, false)] {
            let cards = vec![card("main", None, None, now - delta)];
            let snap = build_snapshot(&cards, now, 90, &labels, &roster);
            assert_eq!(!snap.sessions.is_empty(), expect_alive, "delta={delta}");
        }
    }

    #[test]
    fn remontee_d_ancetre_mort() {
        let root = tmp_root("remontee");
        make_reservoir(&root);
        let labels = labels_with_root(&root);
        let roster = Roster::new();
        let now = 1_000;
        // main vivant, gandalf mort, gimli (enfant de gandalf) vivant -> rattaché au coordinateur.
        let cards = vec![
            card("main", None, None, now),
            card("g1", Some("main"), Some("gandalf"), now - 1000), // mort
            card("g2", Some("g1"), Some("gimli"), now),
        ];
        let snap = build_snapshot(&cards, now, 90, &labels, &roster);
        let coord = &snap.sessions[0].coordinator;
        assert_eq!(coord.children.len(), 1);
        assert_eq!(coord.children[0].persona, "Gimli");
        assert!(coord.children[0].tooltip.contains("parent réel"));
    }

    #[test]
    fn bornes_8_enfants_avec_marqueur_overflow() {
        let root = tmp_root("bornes");
        make_reservoir(&root);
        let labels = labels_with_root(&root);
        let roster = Roster::new();
        let now = 1_000;
        let mut cards = vec![card("main", None, None, now)];
        for i in 0..10 {
            let mut c = card(&format!("c{i}"), Some("main"), Some("gimli"), now);
            c.started_at = 1_000 + i as i64;
            cards.push(c);
        }
        let snap = build_snapshot(&cards, now, 90, &labels, &roster);
        let coord = &snap.sessions[0].coordinator;
        assert_eq!(coord.children.len(), 9); // 8 + marqueur
        assert_eq!(coord.children[8].overflow, Some(2));
    }

    #[test]
    fn bornes_6_sessions_avec_marqueur_overflow() {
        let root = tmp_root("sessions");
        make_reservoir(&root);
        let labels = labels_with_root(&root);
        let roster = Roster::new();
        let now = 1_000;
        let mut cards = Vec::new();
        for i in 0..8 {
            let mut c = card("main", None, None, now);
            c.session_id = format!("sess-{i}");
            c.cwd = Some(format!("/Users/x/work/proj{i}"));
            c.last_seen_at = now - i as i64; // activité décroissante distincte.
            cards.push(c);
        }
        let snap = build_snapshot(&cards, now, 90, &labels, &roster);
        assert_eq!(snap.sessions.len(), 6);
        assert_eq!(snap.overflow_sessions, Some(2));
    }

    #[test]
    fn deux_instantanes_identiques_restent_egaux() {
        let root = tmp_root("egalite");
        make_reservoir(&root);
        let labels = labels_with_root(&root);
        let roster = Roster::new();
        let now = 1_000;
        let cards = vec![card("main", None, None, now), card("c1", Some("main"), Some("gimli"), now)];
        let a = build_snapshot(&cards, now, 90, &labels, &roster);
        let b = build_snapshot(&cards, now, 90, &labels, &roster);
        assert_eq!(a, b);
    }

    #[test]
    fn persona_hors_equipe_gris_avec_sa_lettre() {
        let root = tmp_root("horsequipe");
        make_reservoir(&root);
        let labels = labels_with_root(&root);
        let roster = Roster::new();
        let now = 1_000;
        let cards = vec![
            card("main", None, None, now),
            card("c1", Some("main"), Some("general-purpose"), now),
        ];
        let snap = build_snapshot(&cards, now, 90, &labels, &roster);
        let child = &snap.sessions[0].coordinator.children[0];
        assert_eq!(child.sprite.letter, 'G');
        assert_eq!(child.sprite.bg, FALLBACK_BG);
    }

    #[test]
    fn estimation_valide_donne_une_vue() {
        let root = tmp_root("estimation");
        make_reservoir(&root);
        let labels = labels_with_root(&root);
        let roster = Roster::new();
        let now = 1_300;
        let mut c1 = card("c1", Some("main"), Some("gimli"), now);
        c1.estimate = Some("~10 min".to_string());
        c1.started_at = 1_000;
        let cards = vec![card("main", None, None, now), c1];
        let snap = build_snapshot(&cards, now, 90, &labels, &roster);
        let child = &snap.sessions[0].coordinator.children[0];
        assert!(child.estimate.is_some());
    }

    #[test]
    fn group_color_index_est_deterministe_et_borne() {
        assert_eq!(group_color_index("iakaos"), group_color_index("iakaos"));
        assert!(group_color_index("iakaos") < GROUP_COLOR_COUNT);
    }

    #[test]
    fn reservoir_illisible_donne_gris_partout() {
        let root = tmp_root("reservoir-absent").join("n-existe-pas");
        let labels = labels_with_root(&root);
        let roster = Roster::new();
        let now = 1_000;
        let cards = vec![
            card("main", None, None, now),
            card("c1", Some("main"), Some("gimli"), now),
        ];
        let snap = build_snapshot(&cards, now, 90, &labels, &roster);
        let child = &snap.sessions[0].coordinator.children[0];
        assert_eq!(child.sprite.bg, FALLBACK_BG, "reservoir illisible -> gris");
    }
}
