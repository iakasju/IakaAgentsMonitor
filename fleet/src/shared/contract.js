// Contrat sur le fil v1 (D4) : partage par l'agent ET la console, source unique.
// Topic = code ; payload = {"v":<scalaire|null>,"t":<epoch s>} ; retained, QoS 1.

export const ROOT = 'iakafleet';

export const META_CODES = [
  'online', 'os', 'os_release', 'hostname', 'agent_version', 'node_version',
  'iakaframe_version', 'sessions_dir_ok', 'sessions_count', 'heartbeat_s', 'last_tick_at',
];

export const SESSION_CODES = [
  'status', 'status_since', 'stuck_suspect', 'name', 'kind', 'entrypoint',
  'claude_version', 'cwd', 'repo', 'branch', 'rc', 'started_at',
];

export class ContractError extends Error {
  constructor(message) {
    super(message);
    this.name = 'ContractError';
  }
}

/** D5 : forme normalisee du <host>. Refus nomme si vide ou contenant + # /. */
export function normalizeHost(raw) {
  if (typeof raw !== 'string') throw new ContractError('host invalide : chaine attendue');
  if (/[+#/]/.test(raw)) throw new ContractError(`host invalide : "${raw}" contient +, # ou /`);
  const host = raw
    .trim()
    .toLowerCase()
    .replace(/\.local$/, '')
    .replace(/[^a-z0-9-]/g, '-');
  if (host === '' || /^-+$/.test(host)) throw new ContractError(`host invalide : "${raw}" donne une valeur vide`);
  return host;
}

export const metaTopic = (host, code) => `${ROOT}/${host}/meta/${code}/current`;
export const sessionTopic = (host, pid, code) => `${ROOT}/${host}/sessions/${pid}/${code}/current`;
export const sessionsWildcard = (host) => `${ROOT}/${host}/sessions/#`;
export const ALL_TOPICS = `${ROOT}/#`;

/** Decompose un topic du contrat ; null s'il ne le respecte pas. */
export function parseTopic(topic) {
  const p = String(topic).split('/');
  if (p[0] !== ROOT || p[p.length - 1] !== 'current') return null;
  if (p.length === 5 && p[2] === 'meta') return { host: p[1], kind: 'meta', code: p[3] };
  if (p.length === 6 && p[2] === 'sessions' && /^\d+$/.test(p[3])) {
    return { host: p[1], kind: 'session', pid: Number(p[3]), code: p[4] };
  }
  return null;
}

const isScalar = (v) => ['string', 'number', 'boolean'].includes(typeof v);

/** Encode {v,t} : v scalaire ou null (jamais objet/tableau), t entier. */
export function encode(v, t) {
  if (v !== null && !isScalar(v)) {
    throw new ContractError(`v doit etre un scalaire ou null (recu : ${Array.isArray(v) ? 'tableau' : typeof v})`);
  }
  if (typeof v === 'number' && !Number.isFinite(v)) throw new ContractError('v numerique non fini');
  if (!Number.isInteger(t)) throw new ContractError('t doit etre un entier (epoch s)');
  return JSON.stringify({ v, t });
}

/** Decode un payload. Vide (0 octet) => null (code efface). Invalide => ContractError. */
export function decode(payload) {
  const s = Buffer.isBuffer(payload) ? payload.toString('utf8') : String(payload);
  if (s.length === 0) return null;
  let o;
  try {
    o = JSON.parse(s);
  } catch {
    throw new ContractError('payload non JSON');
  }
  const keys = o && typeof o === 'object' && !Array.isArray(o) ? Object.keys(o) : [];
  if (keys.length !== 2 || !('v' in o) || !('t' in o)) throw new ContractError('payload attendu : exactement {v,t}');
  if (o.v !== null && !isScalar(o.v)) throw new ContractError('v doit etre un scalaire ou null');
  if (!Number.isInteger(o.t)) throw new ContractError('t doit etre un entier');
  return { v: o.v, t: o.t };
}
