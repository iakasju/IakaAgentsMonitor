import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { findGit } from '../src/agent/gitHead.js';

// Un .git imbrique ne peut pas etre commite : les arbres sont fabriques en dossier temporaire.
function withTree(fn) {
  const root = mkdtempSync(join(tmpdir(), 'iakafleet-git-'));
  try {
    fn(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

test('CA-5 : .git dossier sur branche => nom de branche, depuis un sous-dossier', () => withTree((r) => {
  mkdirSync(join(r, 'proj', '.git'), { recursive: true });
  mkdirSync(join(r, 'proj', 'src', 'deep'), { recursive: true });
  writeFileSync(join(r, 'proj', '.git', 'HEAD'), 'ref: refs/heads/feat/x\n');
  assert.deepEqual(findGit(join(r, 'proj', 'src', 'deep')), { repo: 'proj', branch: 'feat/x' });
}));

test('CA-5 : worktree (.git fichier) => branche du worktree', () => withTree((r) => {
  mkdirSync(join(r, 'main', '.git', 'worktrees', 'wt1'), { recursive: true });
  writeFileSync(join(r, 'main', '.git', 'worktrees', 'wt1', 'HEAD'), 'ref: refs/heads/feat/wt\n');
  mkdirSync(join(r, 'wt1'));
  writeFileSync(join(r, 'wt1', '.git'), `gitdir: ${join(r, 'main', '.git', 'worktrees', 'wt1')}\n`);
  assert.deepEqual(findGit(join(r, 'wt1')), { repo: 'wt1', branch: 'feat/wt' });
}));

test('CA-5 : worktree avec gitdir relatif', () => withTree((r) => {
  mkdirSync(join(r, 'main', '.git', 'worktrees', 'w'), { recursive: true });
  writeFileSync(join(r, 'main', '.git', 'worktrees', 'w', 'HEAD'), 'ref: refs/heads/rel\n');
  mkdirSync(join(r, 'w'));
  writeFileSync(join(r, 'w', '.git'), 'gitdir: ../main/.git/worktrees/w\n');
  assert.equal(findGit(join(r, 'w')).branch, 'rel');
}));

test('CA-5 : HEAD detache => detached:<sha7>', () => withTree((r) => {
  mkdirSync(join(r, 'p', '.git'), { recursive: true });
  writeFileSync(join(r, 'p', '.git', 'HEAD'), '0123456789abcdef0123456789abcdef01234567\n');
  assert.equal(findGit(join(r, 'p')).branch, 'detached:0123456');
}));

test('CA-5 : hors depot => null/null ; cwd absent => null/null', () => withTree((r) => {
  mkdirSync(join(r, 'plain'));
  const res = findGit(join(r, 'plain'), { lstat: (p) => { throw Object.assign(new Error(), { code: 'ENOENT' }); } });
  assert.deepEqual(res, { repo: null, branch: null });
  assert.deepEqual(findGit(null), { repo: null, branch: null });
}));

test('depot au HEAD illisible => repo connu, branche null (jamais fatal)', () => withTree((r) => {
  mkdirSync(join(r, 'p', '.git'), { recursive: true });
  assert.deepEqual(findGit(join(r, 'p')), { repo: 'p', branch: null });
}));
