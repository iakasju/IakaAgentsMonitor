#!/usr/bin/env node
// Point d'entree de la console. Pas de garde "fichier principal" : elle est muette sous Windows.
import mqtt from 'mqtt';
import { run } from '../src/console/index.js';

const code = await run(process.argv.slice(2), {
  env: process.env,
  stdout: process.stdout,
  stderr: process.stderr,
  connect: (url, opts) => mqtt.connect(url, opts),
  onInterrupt: (cb) => process.on('SIGINT', cb),
});
process.exit(code);
