import { test } from 'node:test';
import assert from 'node:assert/strict';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { readSessions, isPidAlive, relativeToHome, normalizeSession } from '../src/agent/sessions.js';

const dir = join(dirname(fileURLToPath(import.meta.url)), 'fixtures', 'sessions');

test('CA-4 : idle, busy, stuck_suspect, corrompu ignore+journalise, pid mort non publie', () => {
  const logs = [];
  const { dirOk, sessions } = readSessions(dir, { alive: (pid) => pid !== 1005, log: (m) => logs.push(m) });
  assert.equal(dirOk, true);
  assert.deepEqual(sessions.map((s) => s.pid), [1001, 1002, 1003, 1006]);
  const by = Object.fromEntries(sessions.map((s) => [s.pid, s]));
  assert.equal(by[1001].status, 'idle');
  assert.equal(by[1002].status, 'busy');
  assert.equal(by[1002].stuckSuspect, false);
  assert.equal(by[1003].stuckSuspect, true);
  assert.equal(by[1001].statusSince, 1780000100);
  assert.ok(logs.some((l) => /1004\.json/.test(l)), 'le JSON corrompu est journalise');
  assert.ok(!logs.some((l) => /\.key/.test(l)), 'les .key ne sont jamais lus');
});

test('M5 : rc = chaine non vide ; cle presente mais null => false', () => {
  const { sessions } = readSessions(dir, { alive: () => true, log: () => {} });
  const by = Object.fromEntries(sessions.map((s) => [s.pid, s]));
  assert.equal(by[1001].rc, false);
  assert.equal(by[1002].rc, false);
  assert.equal(by[1006].rc, true);
});

test('repertoire illisible => dirOk=false, jamais un vide silencieux', () => {
  const logs = [];
  const r = readSessions('/nope', { readdir: () => { throw Object.assign(new Error('x'), { code: 'ENOENT' }); }, log: (m) => logs.push(m) });
  assert.deepEqual(r, { dirOk: false, sessions: [] });
  assert.equal(logs.length, 1);
});

test('parsing tolerant : champs absents => null', () => {
  const s = normalizeSession({}, 7);
  assert.equal(s.pid, 7);
  assert.equal(s.status, null);
  assert.equal(s.statusSince, null);
  assert.equal(s.cwd, null);
  assert.equal(s.rc, false);
  assert.equal(s.stuckSuspect, false);
});

test('isPidAlive : vivant, mort, EPERM = vivant, invalide', () => {
  assert.equal(isPidAlive(process.pid), true);
  assert.equal(isPidAlive(1, () => { throw Object.assign(new Error(), { code: 'EPERM' }); }), true);
  assert.equal(isPidAlive(1, () => { throw Object.assign(new Error(), { code: 'ESRCH' }); }), false);
  assert.equal(isPidAlive(0), false);
  assert.equal(isPidAlive(NaN), false);
});

test('relativeToHome : le chemin absolu ne quitte pas la machine', () => {
  assert.equal(relativeToHome('/home/u/work/azime', '/home/u'), '~/work/azime');
  assert.equal(relativeToHome('/home/u', '/home/u/'), '~');
  assert.equal(relativeToHome('/srv/data/proj', '/home/u'), '<hors-home>/proj');
  assert.equal(relativeToHome('/home/uu/x', '/home/u'), '<hors-home>/x');
  assert.equal(relativeToHome(null, '/home/u'), null);
});
