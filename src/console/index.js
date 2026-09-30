// Console CLI (D9) : lecture seule. Ne publie JAMAIS. L'absence ne se deguise pas en vide.
import { loadConfig } from '../shared/config.js';
import { ALL_TOPICS, decode, ContractError } from '../shared/contract.js';
import { buildFleet } from './fleetModel.js';
import { renderFleet } from './render.js';

const USAGE = `usage : iakafleet <ls|watch> [--json]
  ls      instantane de la flotte puis sortie
  watch   vue rafraichie (Ctrl+C pour quitter)
  --json  sortie machine
codes de sortie : 0 ok (y compris flotte vide) | 2 broker/authentification/config en echec | 1 usage
`;

class FleetError extends Error {}

function describe(e, url) {
  const authHint = e?.code === 4 || e?.code === 5 || /not authorized|bad user/i.test(e?.message ?? '')
    ? ' (authentification refusee)'
    : '';
  return `broker MQTT injoignable ou refuse (${url}) : ${e?.message ?? e}${authHint}`;
}

/**
 * Se connecte, s'abonne a iakafleet/# et accumule les retained dans `store`.
 * Resout {client, store, done} ; done() -> promesse resolue quand plus rien n'arrive depuis settleMs.
 */
function open(cfg, deps, { reconnect }) {
  const settleMs = deps.settleMs ?? 1500;
  const capMs = deps.capMs ?? 5000;
  const store = new Map();
  return new Promise((resolve, reject) => {
    const client = deps.connect(cfg.url, {
      clientId: `iakafleet-console-${Math.random().toString(36).slice(2, 10)}`,
      username: cfg.consoleUser,
      password: cfg.consolePass,
      protocolVersion: 4,
      clean: true,
      keepalive: 30,
      reconnectPeriod: reconnect ? 3000 : 0,
      connectTimeout: capMs,
    });
    let connected = false;
    let settled = false;
    let settleTimer = null;
    let settleResolve = null;
    const fail = (e) => {
      if (settled) return;
      settled = true;
      clearTimeout(capTimer);
      clearTimeout(settleTimer);
      client.end?.(true);
      reject(new FleetError(describe(e, cfg.url)));
    };
    const capTimer = setTimeout(() => fail(new Error('delai depasse : aucune reponse du broker')), capMs);
    const armSettle = () => {
      clearTimeout(settleTimer);
      settleTimer = setTimeout(() => settleResolve?.(), settleMs);
    };
    client.on('error', (e) => {
      if (!connected) fail(e);
      else deps.stderr.write(`iakafleet : erreur MQTT : ${e.message}\n`);
    });
    client.on('message', (topic, payload) => {
      let m;
      try {
        m = decode(payload);
      } catch (e) {
        if (e instanceof ContractError) return; // message hors contrat : ignore
        throw e;
      }
      if (m === null) store.delete(topic);
      else store.set(topic, m);
      if (settleResolve) armSettle();
    });
    client.on('connect', () => {
      connected = true;
      client.subscribe(ALL_TOPICS, { qos: 1 }, (err, granted) => {
        if (err) return fail(err);
        if (granted?.some((g) => g.qos === 128)) {
          return fail(new Error('abonnement iakafleet/# refuse par le broker (ACL)'));
        }
        if (settled) return;
        settled = true;
        clearTimeout(capTimer);
        resolve({
          client,
          store,
          done: () =>
            new Promise((r) => {
              settleResolve = r;
              armSettle();
            }),
        });
      });
    });
  });
}

function emit(fleet, nowS, json, out) {
  if (json) {
    const body = { generated_at: nowS, machines: fleet.machines };
    if (fleet.machines.length === 0) body.message = "aucune machine n'a publie";
    out.write(`${JSON.stringify(body, null, 2)}\n`);
  } else {
    out.write(renderFleet(fleet, nowS));
  }
}

/** Point d'entree testable. Retourne le code de sortie. */
export async function run(argv, deps) {
  const out = deps.stdout;
  const err = deps.stderr;
  const nowS = deps.nowS ?? (() => Math.floor(Date.now() / 1000));
  const args = argv.filter((a) => !a.startsWith('--'));
  const json = argv.includes('--json');
  const cmd = args[0];
  if (argv.includes('--help') || argv.includes('-h')) {
    out.write(USAGE);
    return 0;
  }
  if (cmd !== 'ls' && cmd !== 'watch') {
    err.write(USAGE);
    return 1;
  }

  let cfg;
  try {
    cfg = loadConfig({ env: deps.env, readFile: deps.readFile, home: deps.home, hostname: deps.hostname ?? (() => 'console') });
  } catch (e) {
    err.write(`iakafleet : configuration refusee : ${e.message}\n`);
    return 2;
  }
  if (!cfg.consoleUser || !cfg.consolePass) {
    err.write('iakafleet : IAKAFLEET_CONSOLE_USER / IAKAFLEET_CONSOLE_PASS absents (~/.iakafleet/.env)\n');
    return 2;
  }

  let session;
  try {
    session = await open(cfg, deps, { reconnect: cmd === 'watch' });
  } catch (e) {
    err.write(`iakafleet : ${e.message}\n`);
    return 2;
  }
  const { client, store, done } = session;
  try {
    await done();
    if (cmd === 'ls') {
      emit(buildFleet(store, nowS()), nowS(), json, out);
      return 0;
    }
    // watch : redessin periodique jusqu'a Ctrl+C (ou deps.frames pour les tests)
    const frames = deps.frames ?? Infinity;
    const tickMs = deps.tickMs ?? 1000;
    let stop = false;
    deps.onInterrupt?.(() => { stop = true; });
    for (let i = 0; i < frames && !stop; i += 1) {
      if (deps.stdout.isTTY) out.write('\x1b[2J\x1b[H');
      emit(buildFleet(store, nowS()), nowS(), json, out);
      await new Promise((r) => setTimeout(r, tickMs));
    }
    return 0;
  } finally {
    client.end?.(true);
  }
}
