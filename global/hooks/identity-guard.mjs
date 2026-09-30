#!/usr/bin/env node
// identity-guard.mjs — Garde d'identite iakaframe (portage macOS du .ps1)
// Cable sur les hooks Stop et SubagentStop. Verifie que le TOUR courant OUVRE et
// CLOT par un badge. La POSITION de la pastille porte le sens :
//   ouverture = pastille AVANT le bloc  ->  🟡 [PORTEFEUILLE][Odin]
//   cloture   = pastille APRES le bloc  ->  [PORTEFEUILLE][Odin] 🟡
// ASSOUPLISSEMENT : l'ouverture est acceptee si N'IMPORTE QUEL message-texte du tour
// (depuis le dernier vrai prompt user ; les tool_result ne ferment pas le tour) ouvre
// par un badge. La cloture, elle, reste portee par le DERNIER message du tour.
// ANTI-COURSE DE FLUSH : si le verdict serait un refus, on attend brievement et on
// RELIT le transcript (jusqu'a 3 essais). Cela neutralise le cas ou le hook Stop lit
// le fichier avant que le dernier message-texte (porteur du badge de cloture) y soit
// ecrit -> sinon une narration d'outil intermediaire, sans badge, declenche un faux refus.
// Badge manquant -> exit 2 (refus, stderr renvoye a l'agent). FAIL-OPEN : tout bug
// interne => exit 0. Respecte stop_hook_active (anti-boucle).
//
// LIMITE ASSUMEE : ce garde ne lit que le canal ADRESSE (blocs type:"text").
// Les gestes (tool_use) lui sont invisibles -> voir delegation-guard.mjs.
//
// ARCHITECTURE (Lot 0, parite multirunner) : ce fichier est desormais l'ADAPTATEUR CLAUDE de la
// garde d'identite. Il ne fait QUE (a) lire/parser le transcript Claude pour reconstruire le
// `turn` canonique et (b) traduire le verdict en exit code. La LOGIQUE DE DECISION pure (regex
// des badges, calcul startOk/stopOk) vit dans ./guard-core.mjs, partage avec les autres runners
// (Codex...). Comportement Claude STRICTEMENT inchange (verrouille par fixtures + test de parite).

import { readFileSync } from "node:fs";
import { verdictIdentity } from "./guard-core.mjs";

const allow = () => process.exit(0);
const block = (msg) => { process.stderr.write(msg + "\n"); process.exit(2); };

// Sommeil synchrone (pas d'async dans un hook one-shot) sans dependance externe.
const sleep = (ms) => {
  try { Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms); }
  catch { /* SharedArrayBuffer indispo -> on n'attend pas, simple degradation */ }
};

// Reconstruit le `turn` canonique (messages-texte assistant du tour, anti-chrono) depuis le
// transcript Claude. C'est la SEULE partie specifique-Claude ; le verdict est delegue au coeur.
function claudeTurn(tp) {
  let lines;
  try { lines = readFileSync(tp, "utf8").split(/\r?\n/); }
  catch { return null; }

  // Messages assistant-texte du TOUR courant, du plus recent au plus ancien.
  // Le tour s'arrete au dernier VRAI prompt user (un tool_result ne ferme pas le tour).
  const turn = []; // ordre anti-chronologique (turn[0] = dernier message)
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim();
    if (!line) continue;
    let obj;
    try { obj = JSON.parse(line); } catch { continue; }

    if (obj.type === "user" && obj.message) {
      const c = obj.message.content;
      // tool_result pur => fait partie du tour, on continue ; sinon = vrai prompt => frontiere.
      const isToolResultOnly = Array.isArray(c) && c.length > 0
        && c.every((p) => p && p.type === "tool_result");
      if (isToolResultOnly) continue;
      break;
    }

    if (obj.type === "assistant" && obj.message && Array.isArray(obj.message.content)) {
      const parts = obj.message.content
        .filter((c) => c.type === "text" && c.text)
        .map((c) => String(c.text));
      if (parts.length) turn.push(parts.join("\n").trim());
    }
  }
  return turn;
}

// Evalue l'etat courant du transcript. Renvoie :
//   { skip: true }                      -> rien a juger (transcript illisible/vide) => allow
//   { skip: false, startOk, stopOk }    -> verdict sur le tour courant (via guard-core)
function evaluate(tp) {
  const turn = claudeTurn(tp);
  if (turn === null) return { skip: true }; // transcript illisible
  return verdictIdentity(turn);
}

try {
  const raw = readFileSync(0, "utf8");
  if (!raw.trim()) allow();
  const payload = JSON.parse(raw);
  if (payload.stop_hook_active) allow();

  const tp = payload.transcript_path;
  if (!tp) allow();

  // Boucle anti-course : on ne bloque qu'apres avoir laisse le flush se terminer.
  const ATTEMPTS = 3;
  const WAIT_MS = 150;
  let res = { skip: true };
  for (let attempt = 0; attempt < ATTEMPTS; attempt++) {
    res = evaluate(tp);
    if (res.skip || (res.startOk && res.stopOk)) allow();
    if (attempt < ATTEMPTS - 1) sleep(WAIT_MS); // laisser ecrire le dernier message
  }

  const miss = [];
  if (!res.startOk) miss.push("ouverture (pastille AVANT le bloc en PREMIERE ligne, ex: 🟡 [ROYAUME][Agent])");
  if (!res.stopOk) miss.push("cloture (pastille APRES le bloc en derniere ligne, ex: [ROYAUME][Agent] 🟡)");
  block(
    "Garde d'identite iakaframe : badge manquant -> " + miss.join(" + ") +
    ". Convention : ouverture = pastille AVANT, cloture = pastille APRES."
  );
} catch {
  allow(); // fail-open : un bug du garde ne fige jamais une session
}
