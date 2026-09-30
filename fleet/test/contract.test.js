// Formes du contrat v2 hors vecteurs partages (ceux-ci : vectors.test.js).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { encode, decode, ContractError, sidOf } from '../src/shared/contract.js';

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

test('sidOf : deterministe, 12 hex, sans lien lisible avec le sessionId', () => {
  assert.equal(sidOf('x'), sidOf('x'));
  assert.notEqual(sidOf('x'), sidOf('y'));
  assert.match(sidOf('sess-1'), /^[0-9a-f]{12}$/);
});
