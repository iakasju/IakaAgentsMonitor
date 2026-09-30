// G-VIDE (CA-10) : l'absence ne se deguise jamais en "flotte vide".
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { run } from '../src/console/index.js';
import { FakeClient } from './helpers/fakeMqtt.js';

function harness(clientSetup) {
  const out = [];
  const err = [];
  const client = new FakeClient();
  const env = { IAKAFLEET_MQTT_URL: 'mqtts://broker.invalid:8883', IAKAFLEET_CONSOLE_USER: 'u', IAKAFLEET_CONSOLE_PASS: 'p' };
  const deps = {
    env,
    stdout: { write: (s) => out.push(s) },
    stderr: { write: (s) => err.push(s) },
    connect: () => { queueMicrotask(() => clientSetup(client)); return client; },
    nowS: () => 5000,
    settleMs: 20,
    capMs: 200,
    readFile: () => { throw Object.assign(new Error('absent'), { code: 'ENOENT' }); },
  };
  return { deps, out, err, client };
}

test('G-VIDE : broker injoignable => stderr explicite + code 2, aucune table', async () => {
  const h = harness((c) => c.emit('error', Object.assign(new Error('connect ECONNREFUSED'), { code: 'ECONNREFUSED' })));
  const code = await run(['ls'], h.deps);
  assert.equal(code, 2);
  assert.match(h.err.join(''), /ECONNREFUSED|injoignable/i);
  assert.equal(h.out.join(''), '');
});

test('G-VIDE : authentification refusee => code 2', async () => {
  const h = harness((c) => c.emit('error', Object.assign(new Error('Connection refused: Not authorized'), { code: 5 })));
  const code = await run(['ls'], h.deps);
  assert.equal(code, 2);
  assert.match(h.err.join(''), /Not authorized|authentification/i);
});

test('G-VIDE : broker muet (jamais connecte) => code 2 au plafond', async () => {
  const h = harness(() => {});
  assert.equal(await run(['ls'], h.deps), 2);
});

test('G-VIDE : flotte reellement vide => message explicite + code 0', async () => {
  const h = harness((c) => c.emit('connect'));
  const code = await run(['ls'], h.deps);
  assert.equal(code, 0);
  assert.match(h.out.join(''), /aucune machine n'a publi/);
});
