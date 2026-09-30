// Vecteurs partages (contract-vectors/) : la convention est unique parce que les trois pieces
// (ce demon Node, le publieur, l'app Rust) passent les MEMES cas. Voir specs/contrat-bus-iaka.md par. 6.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  normalizeNode, ContractError, presenceTopic, sessionTopic, agentTopic, sessionsWildcard,
  agentsFilter, ALL_TOPICS, parseTopic, sidOf,
} from '../src/shared/contract.js';
import { machineState, machineAlerts } from '../src/console/fleetModel.js';

const dir = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'contract-vectors');
const load = (n) => JSON.parse(readFileSync(join(dir, n), 'utf8'));

test('vecteurs : normalisation de <node>', () => {
  const v = load('node-normalization.json');
  assert.ok(v.ok.length > 0 && v.refused.length > 0);
  for (const c of v.ok) assert.equal(normalizeNode(c.input), c.expected, c.input);
  for (const bad of v.refused) assert.throws(() => normalizeNode(bad), ContractError, JSON.stringify(bad));
});

test('vecteurs : hachage de <sid> (12 hex)', () => {
  const v = load('sid-hash.json');
  assert.ok(v.cases.length > 0);
  for (const c of v.cases) assert.equal(sidOf(c.sessionId), c.expected, c.sessionId);
});

test('vecteurs : construction des topics', () => {
  const b = load('topics.json').build;
  for (const c of b.presence) assert.equal(presenceTopic(c.node, c.code), c.expected);
  for (const c of b.session) assert.equal(sessionTopic(c.node, c.pid, c.code), c.expected);
  for (const c of b.agent) assert.equal(agentTopic(c.node, c.runner, c.sid, c.agentId), c.expected);
  for (const c of b.sessionsWildcard) assert.equal(sessionsWildcard(c.node), c.expected);
  for (const c of b.agentsFilter) assert.equal(agentsFilter(c.node), c.expected);
  assert.equal(ALL_TOPICS, b.allOut);
});

test('vecteurs : decomposition des topics', () => {
  const { parse } = load('topics.json');
  assert.ok(parse.length > 0);
  for (const c of parse) assert.deepEqual(parseTopic(c.topic), c.expected, c.topic);
});

test('vecteurs : regle d\'etat de machine + alertes', () => {
  const { cases } = load('machine-state.json');
  assert.ok(cases.length > 0);
  for (const c of cases) {
    const meta = new Map(Object.entries(c.meta));
    assert.deepEqual(machineState(meta, c.nowS), { state: c.expected.state, since: c.expected.since, basis: c.expected.basis }, c.name);
    assert.deepEqual(machineAlerts(meta), c.expected.alerts, c.name);
  }
});
