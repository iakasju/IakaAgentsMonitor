import { test } from 'node:test';
import assert from 'node:assert/strict';
import { findIakaframeVersion } from '../src/agent/iakaframeVersion.js';

const pkg = (name, version) => JSON.stringify({ name, version });

test('POSIX : realpath du bin puis remontee au package.json du bon nom', () => {
  const files = { '/lib/node_modules/@naonedge/iakaframe/package.json': pkg('@naonedge/iakaframe', '0.41.0') };
  const v = findIakaframeVersion({
    env: { PATH: '/nope:/bin' },
    platform: 'linux',
    exists: (p) => p === '/bin/iakaframe',
    realpath: () => '/lib/node_modules/@naonedge/iakaframe/src/index.js',
    readFile: (p) => { if (p in files) return files[p]; throw new Error('absent'); },
  });
  assert.equal(v, '0.41.0');
});

test('Windows : a cote de iakaframe.cmd, node_modules/@naonedge/iakaframe', () => {
  const target = 'C:\\npm\\node_modules\\@naonedge\\iakaframe\\package.json';
  const v = findIakaframeVersion({
    env: { PATH: 'C:\\x;C:\\npm' },
    platform: 'win32',
    exists: (p) => p === 'C:\\npm\\iakaframe.cmd',
    readFile: (p) => { if (p === target) return pkg('@naonedge/iakaframe', '0.40.2'); throw new Error('absent'); },
  });
  assert.equal(v, '0.40.2');
});

test('IAKAFLEET_IAKAFRAME_PKG prime ; mauvais nom => null', () => {
  const ok = findIakaframeVersion({ env: { IAKAFLEET_IAKAFRAME_PKG: '/p.json' }, readFile: () => pkg('@naonedge/iakaframe', '9.9.9') });
  assert.equal(ok, '9.9.9');
  const bad = findIakaframeVersion({ env: { IAKAFLEET_IAKAFRAME_PKG: '/p.json' }, readFile: () => pkg('autre', '1.0.0') });
  assert.equal(bad, null);
});

test('introuvable => null, jamais invente', () => {
  assert.equal(findIakaframeVersion({ env: { PATH: '/a:/b' }, platform: 'linux', exists: () => false }), null);
  assert.equal(findIakaframeVersion({ env: {}, platform: 'linux' }), null);
});

test('mesure reelle (M3) : sur cette machine, version lue sans execution', { skip: !process.env.IAKAFLEET_LIVE }, () => {
  assert.match(findIakaframeVersion(), /^\d+\.\d+\.\d+/);
});
