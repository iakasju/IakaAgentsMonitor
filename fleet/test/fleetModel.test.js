import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildFleet } from '../src/console/fleetModel.js';
import { renderFleet, formatDuration } from '../src/console/render.js';

const NOW = 10_000;
const put = (store, host, code, v, t = NOW) => store.set(`iakafleet/${host}/meta/${code}/current`, { v, t });
const sess = (store, host, pid, code, v, t = NOW) => store.set(`iakafleet/${host}/sessions/${pid}/${code}/current`, { v, t });
const get = (fleet, host) => fleet.machines.find((m) => m.host === host);

function fleet() {
  const s = new Map();
  // en ligne
  put(s, 'a', 'online', true); put(s, 'a', 'heartbeat_s', 60); put(s, 'a', 'last_tick_at', NOW - 30);
  put(s, 'a', 'os', 'darwin'); put(s, 'a', 'iakaframe_version', '0.41.0');
  sess(s, 'a', 1, 'status', 'busy'); sess(s, 'a', 1, 'repo', 'r'); sess(s, 'a', 1, 'branch', 'main');
  sess(s, 'a', 2, 'status', 'busy'); sess(s, 'a', 2, 'stuck_suspect', true);
  // muette : connecte mais horloge arretee
  put(s, 'b', 'online', true); put(s, 'b', 'heartbeat_s', 60); put(s, 'b', 'last_tick_at', NOW - 500);
  sess(s, 'b', 5, 'status', 'idle', NOW - 500);
  // hors ligne (testament)
  put(s, 'c', 'online', false, NOW - 7200);
  sess(s, 'c', 9, 'status', 'busy', NOW - 7300);
  // inconnue : pas de code online
  put(s, 'd', 'os', 'linux');
  // sessions illisibles
  put(s, 'e', 'online', true); put(s, 'e', 'heartbeat_s', 60); put(s, 'e', 'last_tick_at', NOW - 1); put(s, 'e', 'sessions_dir_ok', false);
  return buildFleet(s, NOW);
}

test('CA-9 : les cinq etats machine', () => {
  const f = fleet();
  assert.equal(get(f, 'a').state, 'online');
  assert.equal(get(f, 'b').state, 'mute');
  assert.equal(get(f, 'c').state, 'offline');
  assert.equal(get(f, 'c').since, NOW - 7200);
  assert.equal(get(f, 'd').state, 'unknown');
  assert.deepEqual(get(f, 'e').alerts, ['sessions illisibles']);
  assert.deepEqual(get(f, 'a').alerts, []);
});

test('CA-9 : limite de peremption = 3 x heartbeat (inclus)', () => {
  const s = new Map();
  put(s, 'x', 'online', true); put(s, 'x', 'heartbeat_s', 10); put(s, 'x', 'last_tick_at', NOW - 30);
  assert.equal(buildFleet(s, NOW).machines[0].state, 'online');
  put(s, 'x', 'last_tick_at', NOW - 31);
  assert.equal(buildFleet(s, NOW).machines[0].state, 'mute');
});

test('CA-9 : online=true sans last_tick_at => muette, jamais en ligne', () => {
  const s = new Map();
  put(s, 'x', 'online', true);
  assert.equal(buildFleet(s, NOW).machines[0].state, 'mute');
});

test('CA-9 : sessions d\'une machine non en ligne => "? (dernier connu ...)"', () => {
  const txt = renderFleet(fleet(), NOW);
  assert.match(txt, /c {2}\[hors ligne depuis 2 h\]/);
  assert.match(txt, /\? \(dernier connu : busy, il y a 2 h\)/);
  assert.match(txt, /\? \(dernier connu : idle, il y a 8 min\)/); // machine muette
  assert.equal(get(fleet(), 'c').sessions[0].live, false);
  assert.equal(get(fleet(), 'a').sessions[0].live, true);
});

test('stuck_suspect => "busy ?" ; sessions_dir_ok=false => alerte affichee', () => {
  const txt = renderFleet(fleet(), NOW);
  assert.match(txt, /busy \?/);
  assert.match(txt, /! sessions illisibles/);
  assert.match(txt, /r@main/);
});

test('flotte vide => message explicite', () => {
  assert.match(renderFleet({ machines: [] }, NOW), /aucune machine n'a publi/);
});

test('formatDuration', () => {
  assert.equal(formatDuration(5), '5 s');
  assert.equal(formatDuration(150), '2 min');
  assert.equal(formatDuration(7200), '2 h');
  assert.equal(formatDuration(200000), '2 j');
});
