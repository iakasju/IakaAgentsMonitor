// G-EXEC (CA-3) : aucun fichier de src/ ni de bin/ ne peut lancer un processus (D2).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');

function walk(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    return statSync(p).isDirectory() ? walk(p) : p.endsWith('.js') || p.endsWith('.mjs') ? [p] : [];
  });
}

test('G-EXEC : ni child_process ni node:child_process dans src/ et bin/', () => {
  const files = [...walk(join(root, 'src')), ...walk(join(root, 'bin'))];
  const offenders = files.filter((f) => /child_process/.test(readFileSync(f, 'utf8')));
  assert.deepEqual(offenders, []);
});

test('G-EXEC : la verification de certificat TLS n\'est jamais debrayee (D10)', () => {
  const files = [...walk(join(root, 'src')), ...walk(join(root, 'bin'))];
  const offenders = files.filter((f) => /rejectUnauthorized/.test(readFileSync(f, 'utf8')));
  assert.deepEqual(offenders, []);
});
