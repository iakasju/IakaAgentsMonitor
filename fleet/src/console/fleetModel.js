// Modele de la flotte (D9) : module PUR. Regle unique de l'etat d'une machine, prouvee par
// contract-vectors/machine-state.json (memes cas cote Rust : src-tauri/src/fleet_model.rs).
// L'absence ne se deguise jamais : une machine sans code `online` est "inconnue", pas "hors ligne".
import { parseTopic } from '../shared/contract.js';

export const DEFAULT_HEARTBEAT_S = 60;
export const STALE_FACTOR = 3;

/** Etat d'une machine a partir de ses codes meta (Map code -> {v,t}). */
export function machineState(meta, nowS) {
  const online = meta.get('online');
  if (!online) return { state: 'unknown', since: null, basis: null };
  if (online.v === false) {
    // Le `t` d'un testament est l'instant de CONNEXION du demon, pas de sa mort (contrat § 2) :
    // « depuis » = dernier signe de vie connu, max(online.t, last_tick_at.v). Jamais online.t seul.
    const tick = meta.get('last_tick_at')?.v;
    if (typeof tick === 'number') return { state: 'offline', since: Math.max(online.t, tick), basis: 'last_seen' };
    return { state: 'offline', since: online.t, basis: 'connected' };
  }
  if (online.v === true) {
    const hb = Number(meta.get('heartbeat_s')?.v) > 0 ? Number(meta.get('heartbeat_s').v) : DEFAULT_HEARTBEAT_S;
    const tick = meta.get('last_tick_at')?.v;
    if (typeof tick === 'number' && nowS - tick <= STALE_FACTOR * hb) return { state: 'online', since: online.t, basis: null };
    return { state: 'mute', since: typeof tick === 'number' ? tick : online.t, basis: null };
  }
  return { state: 'unknown', since: null, basis: null };
}

export const ALERT_SESSIONS_UNREADABLE = 'sessions illisibles';
export const ALERT_NO_CARDS_SOURCE = "fiches d'agents : aucune source sur cette machine";

/** Alertes d'une machine (R2 : « pas de hooks » ne se deguise jamais en « aucun agent »). */
export function machineAlerts(meta) {
  const alerts = [];
  if (meta.get('sessions_dir_ok')?.v === false) alerts.push(ALERT_SESSIONS_UNREADABLE);
  if (meta.get('cards_source_ok')?.v === false) alerts.push(ALERT_NO_CARDS_SOURCE);
  return alerts;
}

/** Fiche distante -> vue d'agent. `task` null = masquee cote machine (jamais une cellule vide). */
function agentView(sid, card, live) {
  const c = card && typeof card === 'object' ? card : {};
  const str = (v) => (typeof v === 'string' ? v : null);
  return {
    sid,
    agentId: str(c.agentId),
    parentId: str(c.parentId),
    persona: str(c.persona),
    model: str(c.model),
    project: str(c.project),
    task: str(c.task),
    state: str(c.state),
    startedAt: Number.isInteger(c.startedAt) ? c.startedAt : null,
    lastSeenAt: Number.isInteger(c.lastSeenAt) ? c.lastSeenAt : null,
    live,
  };
}

/**
 * store : Map topic -> valeur deja decodee ({v,t} pour presence/sessions ; fiche JSON pour agents ;
 * les codes effaces en sont retires). Retourne { machines: [...] } trie par node.
 * Agents rattaches a leur session quand `sid` (code de session) = <sid> de la fiche ; sinon
 * `unattachedAgents` (R3 : jamais perdus).
 */
export function buildFleet(store, nowS) {
  const nodes = new Map();
  const slot = (node) => {
    if (!nodes.has(node)) nodes.set(node, { meta: new Map(), sessions: new Map(), cards: [] });
    return nodes.get(node);
  };
  for (const [topic, msg] of store) {
    const p = parseTopic(topic);
    if (!p) continue;
    const h = slot(p.node);
    if (p.kind === 'presence') h.meta.set(p.code, msg);
    else if (p.kind === 'session') {
      if (!h.sessions.has(p.pid)) h.sessions.set(p.pid, new Map());
      h.sessions.get(p.pid).set(p.code, msg);
    } else h.cards.push({ sid: p.sid, card: msg });
  }
  const machines = [...nodes.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([host, { meta, sessions, cards }]) => {
      const { state, since, basis } = machineState(meta, nowS);
      const mv = (c) => (meta.has(c) ? meta.get(c).v : null);
      const alerts = machineAlerts(meta);
      const list = [...sessions.entries()]
        .sort(([a], [b]) => a - b)
        .map(([pid, codes]) => {
          const sv = (c) => (codes.has(c) ? codes.get(c).v : null);
          const lastT = Math.max(...[...codes.values()].map((m) => m.t));
          return {
            pid,
            status: sv('status'),
            stuckSuspect: sv('stuck_suspect') === true,
            statusSince: sv('status_since'),
            name: sv('name'),
            kind: sv('kind'),
            entrypoint: sv('entrypoint'),
            claudeVersion: sv('claude_version'),
            cwd: sv('cwd'),
            repo: sv('repo'),
            branch: sv('branch'),
            rc: sv('rc') === true,
            sid: sv('sid'),
            startedAt: sv('started_at'),
            lastKnownT: lastT,
            live: state === 'online', // hors "en ligne" : jamais presentee comme tournante
            agents: [],
          };
        });
      const bySid = new Map(list.filter((s) => typeof s.sid === 'string' && s.sid !== '').map((s) => [s.sid, s]));
      const unattachedAgents = [];
      for (const { sid, card } of cards.sort((a, b) => `${a.sid}/${a.card?.agentId}`.localeCompare(`${b.sid}/${b.card?.agentId}`))) {
        const view = agentView(sid, card, state === 'online');
        const owner = bySid.get(sid);
        if (owner) owner.agents.push(view);
        else unattachedAgents.push(view);
      }
      return {
        host,
        state,
        since,
        basis,
        os: mv('os'),
        osRelease: mv('os_release'),
        hostname: mv('hostname'),
        agentVersion: mv('agent_version'),
        nodeVersion: mv('node_version'),
        iakaframeVersion: mv('iakaframe_version'),
        sessionsCount: mv('sessions_count'),
        lastTickAt: mv('last_tick_at'),
        alerts,
        cardsSourceOk: meta.has('cards_source_ok') ? meta.get('cards_source_ok').v : null,
        sessions: list,
        unattachedAgents,
      };
    });
  return { machines };
}
