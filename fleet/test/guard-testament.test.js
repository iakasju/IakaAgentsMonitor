// G-TESTAMENT (CA-7) : le testament est ce qui dit "hors ligne" quand personne ne l'annonce.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildConnectOptions, Publisher } from '../src/agent/publisher.js';
import { FakeClient } from './helpers/fakeMqtt.js';

test('G-TESTAMENT : options de connexion portant le will retained QoS 1', () => {
  const o = buildConnectOptions({ host: 'mbp', username: 'fleet-mbp', password: 'x', nowS: 1000 });
  assert.equal(o.will.topic, 'iakafleet/mbp/meta/online/current');
  assert.deepEqual(JSON.parse(o.will.payload), { v: false, t: 1000 });
  assert.equal(o.will.retain, true);
  assert.equal(o.will.qos, 1);
  assert.equal(o.keepalive, 30);
  assert.equal(o.clientId, 'iakafleet-agent-mbp');
});

test('G-TESTAMENT : arret propre => online=false publie AVANT la deconnexion', async () => {
  const c = new FakeClient();
  const p = new Publisher({ client: c, host: 'mbp', nowS: () => 2000 });
  await p.goOffline();
  c.end();
  const kinds = c.calls.map((x) => x.kind);
  assert.deepEqual(kinds, ['publish', 'end']);
  assert.equal(c.calls[0].topic, 'iakafleet/mbp/meta/online/current');
  assert.deepEqual(JSON.parse(c.calls[0].payload), { v: false, t: 2000 });
});
