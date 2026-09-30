#!/usr/bin/env node
// Point d'entree de l'agent. Pas de garde "fichier principal" : elle est muette sous Windows.
import mqtt from 'mqtt';
import { readFileSync } from 'node:fs';
import { release } from 'node:os';
import { loadConfig } from '../src/shared/config.js';
import { createAgent } from '../src/agent/index.js';

const pkg = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
const log = (m) => process.stderr.write(`[iakafleet-agent] ${m}\n`);

let cfg;
try {
  cfg = loadConfig();
} catch (e) {
  log(`configuration refusee : ${e.message}`);
  process.exit(2);
}
if (!cfg.agentUser || !cfg.agentPass) {
  log('IAKAFLEET_AGENT_USER / IAKAFLEET_AGENT_PASS absents (~/.iakafleet/.env)');
  process.exit(2);
}

const agent = createAgent(cfg, {
  connect: (url, opts) => mqtt.connect(url, opts),
  log,
  info: { platform: process.platform, release: release(), nodeVersion: process.version, agentVersion: pkg.version },
});

let stopping = false;
async function shutdown() {
  if (stopping) return;
  stopping = true;
  await agent.stop();
  process.exit(0);
}
process.on('SIGINT', shutdown);
process.on('SIGTERM', shutdown);
log(`agent demarre : host=${cfg.host} broker=${cfg.url}`);
