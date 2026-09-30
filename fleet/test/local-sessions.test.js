// Vecteurs partages `contract-vectors/local-sessions.json` (specs/contrat-bus-iaka.md par. 6,
// ce-poste-vue-locale-lisibilite.md D2) : part du demon Node = la NORMALISATION d'un fichier de
// session (champs, stuck_suspect, cwd relatif, pid mort => absente). La regle d'assemblage `assemble`
// est consommee par l'app (Rust) seule : le demon ne construit aucune vue locale (exclusion declaree
// dans le champ `_doc` du fichier).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { readSessions, relativeToHome } from '../src/agent/sessions.js';

const dir = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'contract-vectors');
const vectors = JSON.parse(readFileSync(join(dir, 'local-sessions.json'), 'utf8'));

test('vecteurs local-sessions : normalisation d\'un fichier de session (fleet/ = Node)', () => {
  assert.ok(vectors.normalize.length >= 6, 'au moins 6 cas de normalisation');
  for (const c of vectors.normalize) {
    const { dirOk, sessions } = readSessions('/vecteur', {
      readdir: () => [`${c.fallbackPid}.json`],
      readFile: () => JSON.stringify(c.file),
      alive: () => c.alive,
      log: () => {},
    });
    assert.equal(dirOk, true, c.name);
    if (c.expected === null) {
      assert.deepEqual(sessions, [], c.name);
      continue;
    }
    assert.equal(sessions.length, 1, c.name);
    const s = sessions[0];
    const e = c.expected;
    assert.deepEqual(
      { pid: s.pid, sessionId: s.sessionId, status: s.status, statusSince: s.statusSince, stuckSuspect: s.stuckSuspect, name: s.name, cwdRelative: relativeToHome(s.cwd, vectors.home) },
      { pid: e.pid, sessionId: e.sessionId, status: e.status, statusSince: e.statusSince, stuckSuspect: e.stuckSuspect, name: e.name, cwdRelative: e.cwdRelative },
      c.name,
    );
  }
});

test('vecteurs local-sessions : la section assemble est declaree Rust seul (exclusion motivee)', () => {
  assert.ok(vectors.assemble.length >= 8, 'au moins 8 cas d\'assemblage (D2)');
  assert.match(vectors._doc, /Rust\) SEULE/);
});
