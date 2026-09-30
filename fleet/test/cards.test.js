// D3 / CA-5 / CA-6 : relais des fiches d'agents depuis les fichiers du publieur.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { readCards, toCentral } from '../src/agent/cards.js';
import { createAgent } from '../src/agent/index.js';
import { sidOf } from '../src/shared/contract.js';
import { FakeClient } from './helpers/fakeMqtt.js';

const dir = join(dirname(fileURLToPath(import.meta.url)), 'fixtures', 'cards');
const cfgBase = { home: '/home/u', showTasks: false };

test('readCards : lecture tolerante (casse, non-objet, .txt ignores et journalises)', () => {
  const logs = [];
  const r = readCards(dir, { log: (m) => logs.push(m) });
  assert.equal(r.dirOk, true);
  assert.deepEqual(r.cards.map((c) => `${c.sessionDir}/${c.agentFile}`).sort(), [
    'sess-brut-A/child-1.json', 'sess-brut-A/main.json', 'sess-brut-B/main.json',
  ]);
  assert.equal(logs.filter((l) => /fiche ignoree/.test(l)).length, 2);
});

test('readCards : dossier absent => dirOk=false (hooks non installes), jamais un vide silencieux', () => {
  const r = readCards(join(dir, 'nexiste-pas'));
  assert.deepEqual(r, { dirOk: false, cards: [] });
});

test('CA-5 : fiche -> sid, cwd relatif, task NULL par defaut, liste blanche', () => {
  const [main] = readCards(dir).cards.filter((c) => c.sessionDir === 'sess-brut-A' && c.agentFile === 'main.json');
  const out = toCentral(main, cfgBase);
  assert.deepEqual(out.topicParts, { runner: 'claude-code', sid: sidOf('sess-brut-A'), agentId: 'main' });
  const card = JSON.parse(out.payload);
  assert.equal(card.sessionId, sidOf('sess-brut-A'));
  assert.equal(card.cwd, '~/work/alpha');
  assert.equal(card.task, null);
  assert.equal(card.persona, 'Gimli');
  assert.equal(card.estimate, '~10 min');
  assert.equal('secretExtra' in card, false);
  assert.equal('messagingSocketPath' in card, false);
  assert.doesNotMatch(out.payload, /Dupont|facturation|sess-brut-A|secretExtra|\.sock/);
});

test('CA-5 : IAKA_SHOW_TASKS=1 => task publiee telle quelle ; hors-home => dernier segment seul', () => {
  const cs = readCards(dir).cards;
  const main = toCentral(cs.find((c) => c.agentFile === 'main.json' && c.sessionDir === 'sess-brut-A'), { ...cfgBase, showTasks: true });
  assert.equal(JSON.parse(main.payload).task, 'corriger le bug de facturation client Dupont');
  const child = toCentral(cs.find((c) => c.agentFile === 'child-1.json'), cfgBase);
  assert.equal(JSON.parse(child.payload).cwd, '<hors-home>/repo');
  assert.equal(JSON.parse(child.payload).parentId, 'main');
});

test('toCentral : fiche inexploitable (segment invalide, dates absentes) => null', () => {
  const ok = { runner: 'r', agentId: 'a', startedAt: 1, lastSeenAt: 2, sessionId: 's' };
  assert.ok(toCentral({ sessionDir: 's', card: ok }, cfgBase));
  assert.equal(toCentral({ sessionDir: 's', card: { ...ok, agentId: 'a/b' } }, cfgBase), null);
  assert.equal(toCentral({ sessionDir: 's', card: { ...ok, runner: 'r+' } }, cfgBase), null);
  assert.equal(toCentral({ sessionDir: 's', card: { ...ok, startedAt: 'hier' } }, cfgBase), null);
});

function build(cardsState, { showTasks = false } = {}) {
  const client = new FakeClient();
  const tm = { timeouts: [] };
  const cfg = { url: 'x', node: 'mbp', hostnameRaw: 'MBP', home: '/home/u', agentUser: 'u', agentPass: 'p', sessionsDir: '/x', cardsDir: '/c', scanS: 5, heartbeatS: 60, maskNames: false, showTasks };
  const agent = createAgent(cfg, {
    connect: () => client,
    nowS: () => 1000,
    readSessions: () => ({ dirOk: true, sessions: [] }),
    readCards: () => cardsState.value,
    findGit: () => ({ repo: null, branch: null }),
    findVersion: () => null,
    alive: () => false,
    timers: { setInterval: () => 1, clearInterval() {}, setTimeout: (fn, ms) => { tm.timeouts.push(fn); return 1; }, clearTimeout() {} },
    info: { platform: 'darwin', release: '1', nodeVersion: 'v', agentVersion: '0' },
    settleMs: 1,
  });
  return { agent, client, tm };
}

const loadFixtureCards = () => readCards(dir);

test('CA-5 : scenario complet — publication, differentiel, fiche supprimee => payload vide retenu', async () => {
  const state = { value: loadFixtureCards() };
  const h = build(state);
  h.client.emit('connect');
  await new Promise((r) => setImmediate(r));
  h.tm.timeouts.at(-1)();
  const sidA = sidOf('sess-brut-A');
  const t = `iaka/mbp/out/agents/claude-code/${sidA}/main`;
  const pub = h.client.published.find((m) => m.topic === t);
  assert.ok(pub);
  assert.equal(pub.opts.retain, true);
  assert.equal(pub.opts.qos, 1);
  assert.equal(JSON.parse(pub.payload).sessionId, sidA);
  const ok = h.client.published.find((m) => m.topic === 'iaka/mbp/out/presence/cards_source_ok/current');
  assert.equal(JSON.parse(ok.payload).v, true);
  // differentiel : rien de neuf => rien republie
  h.client.published.length = 0;
  h.agent.scan();
  assert.equal(h.client.published.filter((m) => /agents/.test(m.topic)).length, 0);
  // la fiche disparait du disque => effacement retenu
  state.value = { dirOk: true, cards: state.value.cards.filter((c) => c.sessionDir !== 'sess-brut-B') };
  h.agent.scan();
  const cleared = h.client.published.filter((m) => m.payload === '').map((m) => m.topic);
  assert.deepEqual(cleared, [`iaka/mbp/out/agents/claude-code/${sidOf('sess-brut-B')}/main`]);
});

test('CA-6 : aucun sessionId brut, bridgeSessionId, socket, chemin absolu dans TOUT ce qui est publie', async () => {
  const h = build({ value: loadFixtureCards() });
  h.client.emit('connect');
  await new Promise((r) => setImmediate(r));
  h.tm.timeouts.at(-1)();
  const all = h.client.published.map((m) => `${m.topic}\n${m.payload}`).join('\n');
  assert.ok(h.client.published.length > 5);
  assert.doesNotMatch(all, /sess-brut-|bridgeSessionId|messagingSocket|\.sock|\/home\/u|\/opt\/hors-home|Dupont/);
});

test('G-PURGE : au demarrage, fiche retenue d\'une execution precedente absente du disque => effacee', async () => {
  const state = { value: { dirOk: true, cards: [] } };
  const h = build(state);
  h.client.emit('connect');
  await new Promise((r) => setImmediate(r));
  h.client.deliver('iaka/mbp/out/agents/claude-code/deadbeef0000/main', '{"v":1}');
  h.tm.timeouts.at(-1)();
  const cleared = h.client.published.filter((m) => m.payload === '').map((m) => m.topic);
  assert.deepEqual(cleared, ['iaka/mbp/out/agents/claude-code/deadbeef0000/main']);
});

test('R2 : dossier de fiches absent => cards_source_ok=false ECRIT', async () => {
  const h = build({ value: { dirOk: false, cards: [] } });
  h.client.emit('connect');
  await new Promise((r) => setImmediate(r));
  h.tm.timeouts.at(-1)();
  const ok = h.client.published.find((m) => m.topic === 'iaka/mbp/out/presence/cards_source_ok/current');
  assert.equal(JSON.parse(ok.payload).v, false);
});
