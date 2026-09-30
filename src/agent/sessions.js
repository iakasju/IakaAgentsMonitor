// Lecture des fichiers de session de Claude Code (F5) : LECTURE SEULE, parsing tolerant (R1).
// Format interne non documente : un champ absent devient null, un fichier illisible est ignore
// et journalise, jamais fatal.
import { readdirSync, readFileSync } from 'node:fs';
import { join, sep } from 'node:path';

/** Vivacite d'un pid : kill(pid, 0) n'envoie aucun signal ; EPERM = vivant. */
export function isPidAlive(pid, kill = process.kill.bind(process)) {
  if (!Number.isInteger(pid) || pid < 1) return false;
  try {
    kill(pid, 0);
    return true;
  } catch (e) {
    return e?.code === 'EPERM';
  }
}

/** cwd ramene au home ('~/work/azime') ; hors home : seul le dernier segment quitte la machine. */
export function relativeToHome(cwd, home) {
  if (typeof cwd !== 'string' || cwd === '') return null;
  const norm = (p) => p.split(sep).join('/').replace(/\/+$/, '');
  const c = norm(cwd);
  const h = norm(home);
  if (c === h) return '~';
  if (c.startsWith(`${h}/`)) return `~${c.slice(h.length)}`;
  const last = c.split('/').filter(Boolean).pop();
  return last ? `<hors-home>/${last}` : null;
}

const str = (v) => (typeof v === 'string' ? v : null);
const secs = (ms) => (typeof ms === 'number' && Number.isFinite(ms) ? Math.floor(ms / 1000) : null);

/** Normalise un objet de session brut (champs manquants => null). */
export function normalizeSession(raw, fallbackPid) {
  const pid = Number.isInteger(raw.pid) ? raw.pid : fallbackPid;
  const stuck =
    raw.parkedJobId !== undefined && raw.parkedJobId !== null &&
    typeof raw.updatedAt === 'number' && typeof raw.statusUpdatedAt === 'number' &&
    raw.updatedAt > raw.statusUpdatedAt;
  return {
    pid,
    status: str(raw.status),
    statusSince: secs(raw.statusUpdatedAt),
    stuckSuspect: stuck,
    name: str(raw.name),
    kind: str(raw.kind),
    entrypoint: str(raw.entrypoint),
    claudeVersion: str(raw.version),
    cwd: str(raw.cwd),
    // M5 : la cle existe aussi avec la valeur null ; seule une chaine non vide prouve /rc.
    rc: typeof raw.bridgeSessionId === 'string' && raw.bridgeSessionId !== '',
    startedAt: secs(raw.startedAt),
  };
}

/**
 * Lit le repertoire de sessions.
 * deps : { readdir, readFile, alive, log }
 * Retourne { dirOk, sessions }. dirOk=false si le repertoire est illisible (jamais sous-entendu).
 */
export function readSessions(dir, deps = {}) {
  const readdir = deps.readdir ?? readdirSync;
  const readFile = deps.readFile ?? ((p) => readFileSync(p, 'utf8'));
  const alive = deps.alive ?? isPidAlive;
  const log = deps.log ?? (() => {});
  let names;
  try {
    names = readdir(dir);
  } catch (e) {
    log(`repertoire de sessions illisible (${dir}) : ${e.code ?? e.message}`);
    return { dirOk: false, sessions: [] };
  }
  const sessions = [];
  for (const name of names) {
    const m = /^(\d+)\.json$/.exec(name);
    if (!m) continue; // ignore les <pid>.<hash>.key et tout autre fichier
    let raw;
    try {
      raw = JSON.parse(readFile(join(dir, name)));
      if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) throw new Error('objet attendu');
    } catch (e) {
      log(`fichier de session ignore (${name}) : ${e.message}`);
      continue;
    }
    const s = normalizeSession(raw, Number(m[1]));
    if (!alive(s.pid)) continue; // fichier present + pid mort = fin de session (F14)
    sessions.push(s);
  }
  sessions.sort((a, b) => a.pid - b.pid);
  return { dirOk: true, sessions };
}
