// Configuration (D10) : ~/.iakafleet/.env, les variables d'environnement priment.
// Aucun secret ici ; la verification du certificat TLS n'est pas debrayable.
import { homedir, hostname as osHostname } from 'node:os';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { normalizeNode, ContractError } from './contract.js';

export const DEFAULT_URL = 'mqtts://mqtt.naonedge.com:8883';

export class ConfigError extends Error {
  constructor(message) {
    super(message);
    this.name = 'ConfigError';
  }
}

/** Analyse un contenu .env : KEY=VALUE, commentaires #, guillemets simples/doubles retires. */
export function parseEnv(text) {
  const out = {};
  for (const raw of String(text).split(/\r?\n/)) {
    const line = raw.trim();
    if (line === '' || line.startsWith('#')) continue;
    const eq = line.indexOf('=');
    if (eq < 1) continue;
    const key = line.slice(0, eq).trim();
    let val = line.slice(eq + 1).trim();
    if (/^(".*"|'.*')$/.test(val)) val = val.slice(1, -1);
    if (val !== '') out[key] = val;
  }
  return out;
}

/** mqtt:// en clair uniquement vers 127.0.0.1 / localhost ; sinon mqtts:// obligatoire. */
export function checkBrokerUrl(url) {
  let u;
  try {
    u = new URL(url);
  } catch {
    throw new ConfigError(`IAKAFLEET_MQTT_URL invalide : "${url}"`);
  }
  if (u.protocol === 'mqtts:') return url;
  if (u.protocol === 'mqtt:') {
    if (u.hostname === '127.0.0.1' || u.hostname === 'localhost') return url;
    throw new ConfigError(
      `mqtt:// en clair refuse vers "${u.hostname}" : seul mqtts:// est accepte hors 127.0.0.1/localhost (D10)`,
    );
  }
  throw new ConfigError(`protocole "${u.protocol}" non accepte (mqtts:// attendu)`);
}

function positiveInt(name, raw, dflt) {
  if (raw === undefined || raw === '') return dflt;
  const n = Number(raw);
  if (!Number.isInteger(n) || n < 1) throw new ConfigError(`${name} doit etre un entier >= 1 (recu : "${raw}")`);
  return n;
}

/**
 * Charge la configuration.
 * deps : { env, readFile, home, hostname }  (tous injectables pour les tests)
 */
export function loadConfig(deps = {}) {
  const home = deps.home ?? homedir();
  const readFile = deps.readFile ?? ((p) => readFileSync(p, 'utf8'));
  const processEnv = deps.env ?? process.env;
  let fileEnv = {};
  try {
    fileEnv = parseEnv(readFile(join(home, '.iakafleet', '.env')));
  } catch (e) {
    if (e?.code !== 'ENOENT') throw new ConfigError(`~/.iakafleet/.env illisible : ${e.message}`);
  }
  const get = (k) => (processEnv[k] !== undefined && processEnv[k] !== '' ? processEnv[k] : fileEnv[k]);

  const url = checkBrokerUrl(get('IAKAFLEET_MQTT_URL') ?? DEFAULT_URL);
  const hostnameFn = deps.hostname ?? osHostname;
  const hostnameRaw = hostnameFn();
  // IAKA_NODE prioritaire (contrat v2 par. 1) ; IAKAFLEET_HOST reste un alias (D6).
  const rawNode = get('IAKA_NODE') ?? get('IAKAFLEET_HOST') ?? hostnameRaw;
  let node;
  try {
    node = normalizeNode(rawNode);
  } catch (e) {
    if (e instanceof ContractError) throw new ConfigError(e.message);
    throw e;
  }
  return {
    url,
    node,
    hostnameRaw,
    home,
    agentUser: get('IAKAFLEET_AGENT_USER'),
    agentPass: get('IAKAFLEET_AGENT_PASS'),
    consoleUser: get('IAKAFLEET_CONSOLE_USER'),
    consolePass: get('IAKAFLEET_CONSOLE_PASS'),
    sessionsDir: get('IAKAFLEET_SESSIONS_DIR') ?? join(home, '.claude', 'sessions'),
    scanS: positiveInt('IAKAFLEET_SCAN_S', get('IAKAFLEET_SCAN_S'), 5),
    heartbeatS: positiveInt('IAKAFLEET_HEARTBEAT_S', get('IAKAFLEET_HEARTBEAT_S'), 60),
    iakaframePkg: get('IAKAFLEET_IAKAFRAME_PKG'),
    // Par. 4 : noms publies par defaut (IAKA_MASK_NAMES=1 les masque) ; taches MASQUEES par defaut
    // (IAKA_SHOW_TASKS=1 les publie). Alias historique : IAKAFLEET_MASK_NAMES.
    maskNames: (get('IAKA_MASK_NAMES') ?? get('IAKAFLEET_MASK_NAMES')) === '1',
    showTasks: get('IAKA_SHOW_TASKS') === '1',
    cardsDir: get('IAKAFLEET_CARDS_DIR') ?? join(home, '.cache', 'iaka-agents'),
  };
}
