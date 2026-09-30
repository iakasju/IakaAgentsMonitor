// D7 : depot et branche par lecture de .git/HEAD, sans jamais appeler git.
import { lstatSync, readFileSync } from 'node:fs';
import { basename, dirname, isAbsolute, join, resolve } from 'node:path';

function parseHead(text) {
  const t = text.trim();
  const ref = /^ref:\s*refs\/heads\/(.+)$/.exec(t);
  if (ref) return ref[1];
  if (/^[0-9a-f]{7,}$/i.test(t)) return `detached:${t.slice(0, 7)}`;
  return null;
}

/**
 * Remonte de cwd jusqu'au premier dossier contenant .git.
 * deps : { lstat, readFile }. Retourne { repo, branch } (null/null si rien trouve).
 */
export function findGit(cwd, deps = {}) {
  const lstat = deps.lstat ?? lstatSync;
  const readFile = deps.readFile ?? ((p) => readFileSync(p, 'utf8'));
  if (typeof cwd !== 'string' || cwd === '') return { repo: null, branch: null };
  let dir = resolve(cwd);
  for (;;) {
    const dotGit = join(dir, '.git');
    let st = null;
    try {
      st = lstat(dotGit);
    } catch {
      st = null;
    }
    if (st) {
      const repo = basename(dir);
      try {
        let headPath;
        if (st.isDirectory()) {
          headPath = join(dotGit, 'HEAD');
        } else {
          const m = /^gitdir:\s*(.+)$/m.exec(readFile(dotGit));
          if (!m) return { repo, branch: null };
          const gd = m[1].trim();
          headPath = join(isAbsolute(gd) ? gd : resolve(dir, gd), 'HEAD');
        }
        return { repo, branch: parseHead(readFile(headPath)) };
      } catch {
        return { repo, branch: null };
      }
    }
    const parent = dirname(dir);
    if (parent === dir) return { repo: null, branch: null };
    dir = parent;
  }
}
