import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  normalizeHost, encode, decode, parseTopic, metaTopic, sessionTopic, ContractError,
} from '../src/shared/contract.js';

test('normalizeHost : minuscules, .local retire, hors [a-z0-9-] => tiret', () => {
  assert.equal(normalizeHost('MBP-de-sjupin.local'), 'mbp-de-sjupin');
  assert.equal(normalizeHost('DESKTOP-DP5CQVK'), 'desktop-dp5cqvk');
  assert.equal(normalizeHost('Mon Mac_2'), 'mon-mac-2');
});

test('normalizeHost : refus nommes (CA-11)', () => {
  for (const bad of ['', '   ', '.local', '+', 'a/b', 'a#b', 'a+b']) {
    assert.throws(() => normalizeHost(bad), ContractError, bad);
  }
  assert.throws(() => normalizeHost(undefined), ContractError);
});

test('encode : deux cles exactement, v scalaire ou null, t entier (CA-2)', () => {
  assert.deepEqual(JSON.parse(encode('idle', 5)), { v: 'idle', t: 5 });
  assert.deepEqual(Object.keys(JSON.parse(encode(null, 5))).sort(), ['t', 'v']);
  assert.equal(JSON.parse(encode(false, 1)).v, false);
  for (const bad of [{}, [], undefined, () => {}, NaN]) assert.throws(() => encode(bad, 1), ContractError);
  assert.throws(() => encode('x', 1.5), ContractError);
});

test('decode : vide => null ; invalide => refus ; aller-retour', () => {
  assert.equal(decode(''), null);
  assert.equal(decode(Buffer.alloc(0)), null);
  assert.deepEqual(decode(encode(3, 9)), { v: 3, t: 9 });
  for (const bad of ['nope', '[]', '{"v":1}', '{"v":{},"t":1}', '{"v":1,"t":1,"x":2}', '{"v":1,"t":1.5}']) {
    assert.throws(() => decode(bad), ContractError, bad);
  }
});

test('topics : construction et parse', () => {
  assert.equal(metaTopic('h', 'online'), 'iakafleet/h/meta/online/current');
  assert.equal(sessionTopic('h', 42, 'status'), 'iakafleet/h/sessions/42/status/current');
  assert.deepEqual(parseTopic('iakafleet/h/meta/os/current'), { host: 'h', kind: 'meta', code: 'os' });
  assert.deepEqual(parseTopic('iakafleet/h/sessions/42/rc/current'), { host: 'h', kind: 'session', pid: 42, code: 'rc' });
  assert.equal(parseTopic('iakaframe/h/meta/os/current'), null);
  assert.equal(parseTopic('iakafleet/h/sessions/x/rc/current'), null);
  assert.equal(parseTopic('iakafleet/h/meta/os'), null);
});
