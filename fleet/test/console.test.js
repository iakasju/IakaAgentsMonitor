import { test } from 'node:test';
import assert from 'node:assert/strict';
import { run } from '../src/console/index.js';
import { FakeClient } from './helpers/fakeMqtt.js';

function harness(setup, envOver = {}) {
  const out = [];
  const err = [];
  const client = new FakeClient();
  const deps = {
    env: { IAKAFLEET_MQTT_URL: 'mqtts://broker.invalid:8883', IAKAFLEET_CONSOLE_USER: 'u', IAKAFLEET_CONSOLE_PASS: 'SECRET-PASS', ...envOver },
    stdout: { write: (s) => out.push(s) },
    stderr: { write: (s) => err.push(s) },
    connect: () => { queueMicrotask(() => setup(client)); return client; },
    nowS: () => 5000,
    settleMs: 20,
    capMs: 200,
    tickMs: 5,
    frames: 2,
    home: '/h',
    readFile: () => { throw Object.assign(new Error('absent'), { code: 'ENOENT' }); },
  };
  return { deps, out, err, client };
}

const feed = (c) => {
  c.emit('connect');
  setTimeout(() => {
    c.deliver('iakafleet/m1/meta/online/current', '{"v":true,"t":4990}');
    c.deliver('iakafleet/m1/meta/last_tick_at/current', '{"v":4990,"t":4990}');
    c.deliver('iakafleet/m1/meta/heartbeat_s/current', '{"v":60,"t":4990}');
    c.deliver('iakafleet/m1/sessions/7/status/current', '{"v":"idle","t":4990}');
    c.deliver('iakafleet/m1/sessions/7/repo/current', 'pas du json');
  }, 5);
};

test('ls : instantane texte, code 0, et la console ne publie JAMAIS', async () => {
  const h = harness(feed);
  assert.equal(await run(['ls'], h.deps), 0);
  assert.match(h.out.join(''), /m1 {2}\[en ligne\]/);
  assert.match(h.out.join(''), /idle/);
  assert.equal(h.client.published.length, 0);
  assert.equal(h.client.subscribed[0].topic, 'iakafleet/#');
  assert.equal(h.err.join(''), '');
});

test('ls --json : sortie machine', async () => {
  const h = harness(feed);
  assert.equal(await run(['ls', '--json'], h.deps), 0);
  const j = JSON.parse(h.out.join(''));
  assert.equal(j.machines[0].host, 'm1');
  assert.equal(j.machines[0].state, 'online');
  assert.equal(j.machines[0].sessions[0].pid, 7);
});

test('un code efface (payload vide) disparait de la vue', async () => {
  const h = harness((c) => {
    c.emit('connect');
    setTimeout(() => {
      c.deliver('iakafleet/m1/meta/online/current', '{"v":true,"t":4990}');
      c.deliver('iakafleet/m1/sessions/7/status/current', '{"v":"idle","t":4990}');
      c.deliver('iakafleet/m1/sessions/7/status/current', '');
    }, 5);
  });
  await run(['ls', '--json'], h.deps);
  assert.deepEqual(JSON.parse(h.out.join('')).machines[0].sessions, []);
});

test('abonnement refuse (ACL, qos 128) => code 2', async () => {
  const h = harness((c) => {
    c.subscribe = (t, o, cb) => queueMicrotask(() => cb(null, [{ topic: t, qos: 128 }]));
    c.emit('connect');
  });
  assert.equal(await run(['ls'], h.deps), 2);
  assert.match(h.err.join(''), /refuse/);
});

test('identifiants absents ou config invalide => code 2 sans connexion', async () => {
  const h = harness(() => {}, { IAKAFLEET_CONSOLE_PASS: '' });
  assert.equal(await run(['ls'], h.deps), 2);
  assert.match(h.err.join(''), /CONSOLE_USER/);
  const bad = harness(() => {}, { IAKAFLEET_MQTT_URL: 'mqtt://mqtt.exemple.org:1883' });
  assert.equal(await run(['ls'], bad.deps), 2);
  assert.match(bad.err.join(''), /mqtt:\/\/ en clair refuse/);
});

test('le mot de passe n\'apparait jamais dans les sorties', async () => {
  const h = harness((c) => c.emit('error', new Error('boom')));
  await run(['ls'], h.deps);
  assert.doesNotMatch(h.out.join('') + h.err.join(''), /SECRET-PASS/);
});

test('watch : redessine, code 0 ; echec initial => code 2', async () => {
  const h = harness(feed);
  assert.equal(await run(['watch'], h.deps), 0);
  assert.equal(h.out.filter((s) => /m1 {2}\[en ligne\]/.test(s)).length, 2);
  const bad = harness((c) => c.emit('error', new Error('ECONNREFUSED')));
  assert.equal(await run(['watch'], bad.deps), 2);
});

test('usage : commande inconnue => code 1 ; --help => 0', async () => {
  const h = harness(() => {});
  assert.equal(await run(['nope'], h.deps), 1);
  assert.equal(await run(['--help'], h.deps), 0);
});
