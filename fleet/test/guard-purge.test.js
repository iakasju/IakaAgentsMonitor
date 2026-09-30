// G-PURGE (CA-6) : une session finie ne laisse aucun fantome retained sur le broker.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Publisher } from '../src/agent/publisher.js';
import { FakeClient } from './helpers/fakeMqtt.js';

const codes = { status: 'idle', status_since: 10, name: 'x', repo: 'r', branch: 'main', rc: false };

test('G-PURGE : session disparue => payload vide retained sur CHACUN de ses codes', () => {
  const c = new FakeClient();
  const p = new Publisher({ client: c, node: 'mbp', nowS: () => 100 });
  p.sync({ presence: {}, sessions: [{ pid: 42, codes }] });
  c.published.length = 0;
  p.sync({ presence: {}, sessions: [] });
  const cleared = c.published.map((m) => m.topic).sort();
  const expected = Object.keys(codes).map((k) => `iaka/mbp/out/sessions/42/${k}/current`).sort();
  assert.deepEqual(cleared, expected);
  for (const m of c.published) {
    assert.equal(m.payload, '');
    assert.equal(m.opts.retain, true);
    assert.equal(m.opts.qos, 1);
  }
});

test('G-PURGE : au demarrage, retained d\'un pid mort efface, pid vivant conserve', () => {
  const c = new FakeClient();
  const p = new Publisher({ client: c, node: 'mbp', nowS: () => 100 });
  const retained = [
    'iaka/mbp/out/sessions/111/status/current',
    'iaka/mbp/out/sessions/111/repo/current',
    'iaka/mbp/out/sessions/222/status/current',
  ];
  p.purgeRetained(retained, (pid) => pid === 222);
  assert.deepEqual(c.published.map((m) => m.topic).sort(), [
    'iaka/mbp/out/sessions/111/repo/current',
    'iaka/mbp/out/sessions/111/status/current',
  ]);
  assert.ok(c.published.every((m) => m.payload === '' && m.opts.retain === true));
});
