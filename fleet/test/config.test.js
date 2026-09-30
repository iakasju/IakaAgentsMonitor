import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseEnv, checkBrokerUrl, loadConfig, ConfigError, DEFAULT_URL } from '../src/shared/config.js';

const noFile = () => { throw Object.assign(new Error('absent'), { code: 'ENOENT' }); };

test('parseEnv : commentaires, guillemets, valeurs vides ignorees', () => {
  const o = parseEnv('# c\nA=1\nB="deux"\nC=\n  D = \'x y\' \r\nnope\n');
  assert.deepEqual(o, { A: '1', B: 'deux', D: 'x y' });
});

test('checkBrokerUrl : mqtt:// clair seulement en local (CA-11)', () => {
  assert.equal(checkBrokerUrl('mqtt://127.0.0.1:1883'), 'mqtt://127.0.0.1:1883');
  assert.equal(checkBrokerUrl('mqtt://localhost:1883'), 'mqtt://localhost:1883');
  assert.equal(checkBrokerUrl(DEFAULT_URL), DEFAULT_URL);
  assert.throws(() => checkBrokerUrl('mqtt://mqtt.naonedge.com:1883'), ConfigError);
  assert.throws(() => checkBrokerUrl('mqtt://192.168.1.10'), ConfigError);
  assert.throws(() => checkBrokerUrl('ws://localhost:9001'), ConfigError);
  assert.throws(() => checkBrokerUrl('pas une url'), ConfigError);
});

test('loadConfig : defauts, priorite env > fichier, masquage', () => {
  const file = 'IAKAFLEET_HOST=fromfile\nIAKAFLEET_SCAN_S=9\nIAKAFLEET_AGENT_USER=u\n';
  const c = loadConfig({ env: { IAKAFLEET_SCAN_S: '3' }, readFile: () => file, home: '/h', hostname: () => 'X' });
  assert.equal(c.node, 'fromfile');
  assert.equal(c.scanS, 3);
  assert.equal(c.heartbeatS, 60);
  assert.equal(c.agentUser, 'u');
  assert.equal(c.url, DEFAULT_URL);
  assert.equal(c.maskNames, false);
  assert.match(c.sessionsDir, /\.claude[\\/]sessions$/);
});

test('loadConfig : node derive du hostname, invalide => refus nomme (CA-11)', () => {
  const c = loadConfig({ env: {}, readFile: noFile, home: '/h', hostname: () => 'MBP-de-sjupin.local' });
  assert.equal(c.node, 'mbp-de-sjupin');
  assert.throws(() => loadConfig({ env: { IAKAFLEET_HOST: 'a/b' }, readFile: noFile, home: '/h' }), ConfigError);
  assert.throws(() => loadConfig({ env: {}, readFile: noFile, home: '/h', hostname: () => '' }), ConfigError);
});

test('loadConfig : mqtt:// distant refuse au demarrage', () => {
  assert.throws(
    () => loadConfig({ env: { IAKAFLEET_MQTT_URL: 'mqtt://mqtt.exemple.org' }, readFile: noFile, home: '/h', hostname: () => 'h' }),
    ConfigError,
  );
});

test('loadConfig : IAKAFLEET_MASK_NAMES=1 seulement', () => {
  const mk = (v) => loadConfig({ env: { IAKAFLEET_MASK_NAMES: v }, readFile: noFile, home: '/h', hostname: () => 'h' }).maskNames;
  assert.equal(mk('1'), true);
  assert.equal(mk('0'), false);
  assert.equal(mk(undefined), false);
});

test('loadConfig : IAKA_NODE prioritaire sur IAKAFLEET_HOST (alias), refus nomme', () => {
  const mk = (env) => loadConfig({ env, readFile: noFile, home: '/h', hostname: () => 'h' });
  assert.equal(mk({ IAKA_NODE: 'Mac-Pro', IAKAFLEET_HOST: 'ancien' }).node, 'mac-pro');
  assert.equal(mk({ IAKAFLEET_HOST: 'ancien' }).node, 'ancien');
  assert.throws(() => mk({ IAKA_NODE: 'a#b' }), ConfigError);
});

test('loadConfig : IAKA_MASK_NAMES (alias IAKAFLEET_MASK_NAMES) ; IAKA_SHOW_TASKS defaut DESACTIVE (Q1)', () => {
  const mk = (env) => loadConfig({ env, readFile: noFile, home: '/h', hostname: () => 'h' });
  assert.equal(mk({}).showTasks, false);
  assert.equal(mk({ IAKA_SHOW_TASKS: '1' }).showTasks, true);
  assert.equal(mk({ IAKA_SHOW_TASKS: 'true' }).showTasks, false);
  assert.equal(mk({ IAKA_MASK_NAMES: '1' }).maskNames, true);
  assert.equal(mk({ IAKAFLEET_MASK_NAMES: '1' }).maskNames, true);
  assert.equal(mk({ IAKA_MASK_NAMES: '0', IAKAFLEET_MASK_NAMES: '1' }).maskNames, false);
  assert.match(mk({}).cardsDir, /\.cache[\\/]iaka-agents$/);
});

test('loadConfig : entier invalide refuse', () => {
  assert.throws(
    () => loadConfig({ env: { IAKAFLEET_SCAN_S: 'abc' }, readFile: noFile, home: '/h', hostname: () => 'h' }),
    ConfigError,
  );
});
