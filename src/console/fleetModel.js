// Modele de la flotte (D9) : module PUR. Regle unique de l'etat d'une machine.
// L'absence ne se deguise jamais : une machine sans code `online` est "inconnue", pas "hors ligne".
import { parseTopic } from '../shared/contract.js';

export const DEFAULT_HEARTBEAT_S = 60;
export const STALE_FACTOR = 3;

/** Etat d'une machine a partir de ses codes meta (Map code -> {v,t}). */
export function machineState(meta, nowS) {
  const online = meta.get('online');
  if (!online) return { state: 'unknown', since: null };
  if (online.v === false) return { state: 'offline', since: online.t };
  if (online.v === true) {
    const hb = Number(meta.get('heartbeat_s')?.v) > 0 ? Number(meta.get('heartbeat_s').v) : DEFAULT_HEARTBEAT_S;
    const tick = meta.get('last_tick_at')?.v;
    if (typeof tick === 'number' && nowS - tick <= STALE_FACTOR * hb) return { state: 'online', since: online.t };
    return { state: 'mute', since: typeof tick === 'number' ? tick : online.t };
  }
  return { state: 'unknown', since: null };
}

/**
 * store : Map topic -> {v,t} (deja decode, les codes effaces en sont retires).
 * Retourne { machines: [...] } trie par host.
 */
export function buildFleet(store, nowS) {
  const hosts = new Map();
  const slot = (host) => {
    if (!hosts.has(host)) hosts.set(host, { meta: new Map(), sessions: new Map() });
    return hosts.get(host);
  };
  for (const [topic, msg] of store) {
    const p = parseTopic(topic);
    if (!p) continue;
    const h = slot(p.host);
    if (p.kind === 'meta') h.meta.set(p.code, msg);
    else {
      if (!h.sessions.has(p.pid)) h.sessions.set(p.pid, new Map());
      h.sessions.get(p.pid).set(p.code, msg);
    }
  }
  const machines = [...hosts.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([host, { meta, sessions }]) => {
      const { state, since } = machineState(meta, nowS);
      const mv = (c) => (meta.has(c) ? meta.get(c).v : null);
      const alerts = [];
      if (mv('sessions_dir_ok') === false) alerts.push('sessions illisibles');
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
            startedAt: sv('started_at'),
            lastKnownT: lastT,
            live: state === 'online', // hors "en ligne" : jamais presentee comme tournante
          };
        });
      return {
        host,
        state,
        since,
        os: mv('os'),
        osRelease: mv('os_release'),
        hostname: mv('hostname'),
        agentVersion: mv('agent_version'),
        nodeVersion: mv('node_version'),
        iakaframeVersion: mv('iakaframe_version'),
        sessionsCount: mv('sessions_count'),
        lastTickAt: mv('last_tick_at'),
        alerts,
        sessions: list,
      };
    });
  return { machines };
}
