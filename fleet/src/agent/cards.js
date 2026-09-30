// Relais des fiches d'agents (instruction de fusion D3) : le demon LIT, en lecture seule, les
// fichiers que le publieur de hooks ecrit deja (~/.cache/iaka-agents/<sessionId>/<agentId>.json =
// {card, lastPublishedAt}) et les republie vers le central. Aucune dependance au bus local, aucun
// processus lance. Lecture tolerante (R6) : un fichier illisible est ignore et journalise.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { isValidSegment, sidOf } from '../shared/contract.js';
import { relativeToHome } from './sessions.js';

const MAX_PAYLOAD_BYTES = 4096; // contrat v1 du Monitor, par. Fiche

/**
 * Lit le dossier de fiches.
 * deps : { readdir, readFile, isDir, log }
 * Retourne { dirOk, cards: [{ sessionDir, agentFile, card }] }. dirOk=false si le dossier n'existe
 * pas (hooks jamais installes) : ecrit `cards_source_ok=false`, jamais sous-entendu (R2).
 */
export function readCards(dir, deps = {}) {
  const readdir = deps.readdir ?? readdirSync;
  const readFile = deps.readFile ?? ((p) => readFileSync(p, 'utf8'));
  const isDir = deps.isDir ?? ((p) => statSync(p).isDirectory());
  const log = deps.log ?? (() => {});
  let sessionDirs;
  try {
    sessionDirs = readdir(dir);
  } catch (e) {
    log(`dossier de fiches illisible (${dir}) : ${e.code ?? e.message}`);
    return { dirOk: false, cards: [] };
  }
  const cards = [];
  for (const sessionDir of sessionDirs) {
    const sdir = join(dir, sessionDir);
    let files;
    try {
      if (!isDir(sdir)) continue;
      files = readdir(sdir);
    } catch (e) {
      log(`dossier de session ignore (${sessionDir}) : ${e.code ?? e.message}`);
      continue;
    }
    for (const agentFile of files) {
      if (!agentFile.endsWith('.json')) continue;
      try {
        const raw = JSON.parse(readFile(join(sdir, agentFile)));
        const card = raw?.card;
        if (card === null || typeof card !== 'object' || Array.isArray(card)) throw new Error('objet {card} attendu');
        cards.push({ sessionDir, agentFile, card });
      } catch (e) {
        log(`fiche ignoree (${sessionDir}/${agentFile}) : ${e.message}`);
      }
    }
  }
  return { dirOk: true, cards };
}

const str = (v) => (typeof v === 'string' ? v : null);
const int = (v) => (Number.isInteger(v) ? v : null);

/**
 * Fiche locale -> publication centrale. Liste BLANCHE de champs (un champ inconnu ne quitte jamais
 * la machine) ; `sessionId` remplace par <sid> (sha256, 12 hex) ; `cwd` ramene au home ; `task` a
 * null sauf cfg.showTasks (Q1 : masque par defaut). Retourne null si la fiche est inexploitable.
 * { topicParts: {runner, sid, agentId}, payload: string }
 */
export function toCentral({ sessionDir, card }, cfg) {
  const runner = str(card.runner);
  const agentId = str(card.agentId);
  const sessionId = str(card.sessionId) ?? sessionDir;
  if (!isValidSegment(runner) || !isValidSegment(agentId) || !sessionId) return null;
  const sid = sidOf(sessionId);
  const parentId = str(card.parentId);
  const out = {
    v: 1,
    runner,
    runnerVersion: str(card.runnerVersion),
    model: str(card.model),
    persona: str(card.persona),
    agentId,
    parentId: parentId !== null && isValidSegment(parentId) ? parentId : null,
    sessionId: sid,
    project: str(card.project),
    cwd: relativeToHome(str(card.cwd), cfg.home),
    task: cfg.showTasks ? str(card.task) : null,
    estimate: str(card.estimate),
    startedAt: int(card.startedAt),
    lastSeenAt: int(card.lastSeenAt),
    state: card.state === 'idle' ? 'idle' : 'running',
  };
  if (out.startedAt === null || out.lastSeenAt === null) return null; // fiche v1 invalide
  const payload = JSON.stringify(out);
  if (Buffer.byteLength(payload, 'utf8') > MAX_PAYLOAD_BYTES) return null;
  return { topicParts: { runner, sid, agentId }, payload };
}
