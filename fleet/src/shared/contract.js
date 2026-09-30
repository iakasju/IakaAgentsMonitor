// Contrat du bus iaka v2 (specs/contrat-bus-iaka.md) : partage par le demon ET la console, source
// unique cote Node. Presence/sessions : topic = code ; payload = {"v":<scalaire|null>,"t":<epoch s>} ;
// retained, QoS 1. Agents : fiche JSON v1 du Monitor (exception declaree au modele code/valeur).
import { createHash } from 'node:crypto';

export const ROOT = 'iaka';

export const PRESENCE_CODES = [
  'online', 'os', 'os_release', 'hostname', 'agent_version', 'node_version',
  'iakaframe_version', 'sessions_dir_ok', 'sessions_count', 'heartbeat_s', 'last_tick_at',
  'cards_source_ok',
];

export const SESSION_CODES = [
  'status', 'status_since', 'stuck_suspect', 'name', 'kind', 'entrypoint',
  'claude_version', 'cwd', 'repo', 'branch', 'rc', 'started_at', 'sid',
];

export class ContractError extends Error {
  constructor(message) {
    super(message);
    this.name = 'ContractError';
  }
}

/** Par. 1 : forme normalisee du <node>. Refus nomme si vide ou contenant + # /. */
export function normalizeNode(raw) {
  if (typeof raw !== 'string') throw new ContractError('node invalide : chaine attendue');
  if (/[+#/]/.test(raw)) throw new ContractError(`node invalide : "${raw}" contient +, # ou /`);
  const node = raw
    .trim()
    .toLowerCase()
    .replace(/\.local$/, '')
    .replace(/[^a-z0-9-]/gu, '-');
  if (node === '' || /^-+$/.test(node)) throw new ContractError(`node invalide : "${raw}" donne une valeur vide`);
  return node;
}

/** Par. 1 : <sid> sur un broker central = sha256(sessionId) hex, 12 premiers caracteres. */
export function sidOf(sessionId) {
  return createHash('sha256').update(String(sessionId)).digest('hex').slice(0, 12);
}

const SEGMENT_RE = /^[A-Za-z0-9._-]{1,128}$/;
export const isValidSegment = (s) => typeof s === 'string' && SEGMENT_RE.test(s);

export const presenceTopic = (node, code) => `${ROOT}/${node}/out/presence/${code}/current`;
export const sessionTopic = (node, pid, code) => `${ROOT}/${node}/out/sessions/${pid}/${code}/current`;
export const agentTopic = (node, runner, sid, agentId) => `${ROOT}/${node}/out/agents/${runner}/${sid}/${agentId}`;
export const sessionsWildcard = (node) => `${ROOT}/${node}/out/sessions/#`;
export const agentsWildcard = (node) => `${ROOT}/${node}/out/agents/#`;
export const agentsFilter = (node) => `${ROOT}/${node}/out/agents/+/+/+`;
export const ALL_TOPICS = `${ROOT}/+/out/#`;

/** Decompose un topic du contrat ; null s'il ne le respecte pas. */
export function parseTopic(topic) {
  const p = String(topic).split('/');
  if (p[0] !== ROOT || p[2] !== 'out' || !isValidSegment(p[1])) return null;
  const node = p[1];
  if (p[3] === 'presence' && p.length === 6 && p[5] === 'current') return { node, kind: 'presence', code: p[4] };
  if (p[3] === 'sessions' && p.length === 7 && p[6] === 'current' && /^\d+$/.test(p[4])) {
    return { node, kind: 'session', pid: Number(p[4]), code: p[5] };
  }
  if (p[3] === 'agents' && p.length === 7 && [p[4], p[5], p[6]].every(isValidSegment)) {
    return { node, kind: 'agent', runner: p[4], sid: p[5], agentId: p[6] };
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
