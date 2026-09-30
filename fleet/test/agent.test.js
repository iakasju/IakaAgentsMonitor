import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createAgent } from '../src/agent/index.js';
import { decode, parseTopic } from '../src/shared/contract.js';
import { FakeClient } from './helpers/fakeMqtt.js';

const baseSession = (pid, over = {}) => ({
  pid, status: 'idle', statusSince: 10, stuckSuspect: false, name: `s${pid}`, kind: 'interactive',
  entrypoint: 'cli', claudeVersion: '2.1.285', cwd: '/home/u/work/p', rc: false, startedAt: 5, ...over,
});

function build({ sessions = [baseSession(1)], maskNames = false, showTasks = false, alive = () => true, dirOk = true, cards = { dirOk: true, cards: [] } } = {}) {
  const client = new FakeClient();
  let clock = 1000;
  const state = { sessions, dirOk, cards };
  const tm = { intervals: [], timeouts: [] };
  const timers = {
    setInterval: (fn, ms) => { tm.intervals.push({ fn, ms }); return tm.intervals.length; },
    clearInterval: () => {},
    setTimeout: (fn, ms) => { tm.timeouts.push({ fn, ms }); return tm.timeouts.length; },
    clearTimeout: () => {},
  };
  const cfg = {
    url: 'mqtt://127.0.0.1:1', node: 'mbp', hostnameRaw: 'MBP.local', home: '/home/u', agentUser: 'u', agentPass: 'p',
    sessionsDir: '/x', cardsDir: '/c', scanS: 5, heartbeatS: 60, maskNames, showTasks,
  };
  const agent = createAgent(cfg, {
    connect: () => client,
    nowS: () => clock,
    readSessions: () => ({ dirOk: state.dirOk, sessions: state.sessions }),
    readCards: () => state.cards,
    findGit: () => ({ repo: 'p', branch: 'main' }),
    findVersion: () => '0.41.0',
    alive,
    timers,
    info: { platform: 'darwin', release: '25.2', nodeVersion: 'v24', agentVersion: '0.1.0' },
    settleMs: 1,
  });
  const tick = (s = 0) => { clock += s; };
  return { agent, client, state, tm, tick };
}

async function connect(h) {
  h.client.emit('connect');
  await new Promise((r) => setImmediate(r));
  h.tm.timeouts.at(-1).fn(); // fin de la fenetre de purge
}

test('CA-2 : chaque message = {v,t} deux cles, topics iaka/<node>/out/.../current', async () => {
  const h = build();
  await connect(h);
  assert.ok(h.client.published.length > 10);
  for (const m of h.client.published) {
    assert.match(m.topic, /^iaka\/mbp\/out\/(presence|sessions)\/.+\/current$/);
    assert.equal(m.opts.retain, true);
    assert.equal(m.opts.qos, 1);
    const d = decode(m.payload);
    assert.deepEqual(Object.keys(d).sort(), ['t', 'v']);
    assert.ok(d.v === null || ['string', 'number', 'boolean'].includes(typeof d.v));
    assert.ok(Number.isInteger(d.t));
  }
  const vals = Object.fromEntries(h.client.published.map((m) => [m.topic, decode(m.payload).v]));
  assert.equal(vals['iaka/mbp/out/presence/online/current'], true);
  assert.equal(vals['iaka/mbp/out/presence/hostname/current'], 'MBP.local');
  assert.equal(vals['iaka/mbp/out/presence/iakaframe_version/current'], '0.41.0');
  assert.equal(vals['iaka/mbp/out/presence/sessions_count/current'], 1);
  assert.equal(vals['iaka/mbp/out/sessions/1/cwd/current'], '~/work/p');
  assert.equal(vals['iaka/mbp/out/sessions/1/branch/current'], 'main');
});

test('non publie, delibere : sessionId, bridge, socket, contenu (topics ET payloads, CA-6)', async () => {
  const h = build({ sessions: [baseSession(1, { sessionId: 'sess-brut-1' })] });
  await connect(h);
  const all = h.client.published.map((m) => `${m.topic}\n${m.payload}`).join('\n');
  assert.doesNotMatch(all, /sessionId|bridge|socket|sess-brut-1|\/home\/u/i);
  // ...mais le <sid> haché est publié, cle de rattachement des fiches
  const sid = h.client.published.find((m) => m.topic === 'iaka/mbp/out/sessions/1/sid/current');
  assert.match(decode(sid.payload).v, /^[0-9a-f]{12}$/);
});

test('CA-8 : deux balayages identiques => zero republication ; heartbeat => last_tick_at seul', async () => {
  const h = build();
  await connect(h);
  h.client.published.length = 0;
  h.tick(5);
  h.agent.scan();
  h.agent.scan();
  assert.equal(h.client.published.length, 0);
  h.tick(60);
  h.agent.scan();
  assert.deepEqual(h.client.published.map((m) => m.topic), ['iaka/mbp/out/presence/last_tick_at/current']);
});

test('CA-8 : apres reconnexion => republication INTEGRALE', async () => {
  const h = build();
  await connect(h);
  const first = h.client.published.length;
  h.client.published.length = 0;
  await connect(h);
  assert.equal(h.client.published.length, first);
});

test('CA-8 : un changement de statut ne republie que ce code', async () => {
  const h = build();
  await connect(h);
  h.client.published.length = 0;
  h.state.sessions = [baseSession(1, { status: 'busy', statusSince: 20 })];
  h.tick(5);
  h.agent.scan();
  assert.deepEqual(h.client.published.map((m) => m.topic).sort(), [
    'iaka/mbp/out/sessions/1/status/current',
    'iaka/mbp/out/sessions/1/status_since/current',
  ]);
});

test('CA-12 : masquage => name null pour TOUTES les sessions, autres codes inchanges', async () => {
  const on = build({ sessions: [baseSession(1), baseSession(2)], maskNames: true });
  await connect(on);
  const off = build({ sessions: [baseSession(1), baseSession(2)], maskNames: false });
  await connect(off);
  const get = (h, t) => decode(h.client.published.find((m) => m.topic === t).payload).v;
  for (const pid of [1, 2]) {
    assert.equal(get(on, `iaka/mbp/out/sessions/${pid}/name/current`), null);
    assert.equal(get(off, `iaka/mbp/out/sessions/${pid}/name/current`), `s${pid}`);
    assert.equal(get(on, `iaka/mbp/out/sessions/${pid}/status/current`), get(off, `iaka/mbp/out/sessions/${pid}/status/current`));
  }
});

test('D6 : purge au demarrage => retained du pid mort efface, pid vivant conserve', async () => {
  const h = build({ sessions: [], alive: (pid) => pid === 2 });
  h.client.emit('connect');
  await new Promise((r) => setImmediate(r));
  assert.deepEqual(h.client.subscribed[0].topic, ['iaka/mbp/out/sessions/#', 'iaka/mbp/out/agents/#']);
  h.client.deliver('iaka/mbp/out/sessions/1/status/current', '{"v":"busy","t":1}');
  h.client.deliver('iaka/mbp/out/sessions/2/status/current', '{"v":"idle","t":1}');
  h.tm.timeouts.at(-1).fn();
  const cleared = h.client.published.filter((m) => m.payload === '').map((m) => m.topic);
  assert.deepEqual(cleared, ['iaka/mbp/out/sessions/1/status/current']);
});

test('R1 : repertoire illisible => sessions_dir_ok=false ecrit', async () => {
  const h = build({ sessions: [], dirOk: false });
  await connect(h);
  const m = h.client.published.find((x) => x.topic === 'iaka/mbp/out/presence/sessions_dir_ok/current');
  assert.equal(decode(m.payload).v, false);
});

test('CA-7 : stop() => online=false publie AVANT end()', async () => {
  const h = build();
  await connect(h);
  h.client.calls.length = 0;
  await h.agent.stop();
  assert.deepEqual(h.client.calls.map((c) => c.kind), ['publish', 'end']);
  assert.equal(h.client.calls[0].topic, 'iaka/mbp/out/presence/online/current');
  assert.equal(decode(h.client.calls[0].payload).v, false);
});

test('l\'agent ne s\'abonne qu\'a son propre sous-arbre de sessions', async () => {
  const h = build();
  await connect(h);
  assert.deepEqual(h.client.subscribed.map((s) => s.topic), [['iaka/mbp/out/sessions/#', 'iaka/mbp/out/agents/#']]);
  for (const m of h.client.published) assert.ok(parseTopic(m.topic).node === 'mbp');
  // jamais d'abonnement sur in/ : le demon ne lit aucun ordre (contrat par. 3)
  assert.ok(h.client.subscribed.every((s) => JSON.stringify(s.topic).indexOf('/in/') === -1));
});
