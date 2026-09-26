// guard-core.mjs — Coeur d'enforcement iakaframe RUNNER-AGNOSTIQUE (Lot 0, socle de parite).
//
// POURQUOI : chaque garde Claude melangeait (a) le PARSING de son payload (forme du transcript,
// noms de champs) et (b) la LOGIQUE DE DECISION (regex des badges, appartenance d'un chemin a un
// perimetre, appartenance d'un agent au roster). La logique (b) est INTRINSEQUEMENT
// runner-agnostique : c'est le levier de la parite d'enforcement hors Claude (cf.
// specs/instructions/parite-enforcement-multirunner.md, section 5.2).
//
// Ce module ne contient QUE la logique (b) : des VERDICTS PURS.
//   - AUCUN I/O (pas de readFileSync, pas de stdin, pas de process.exit, pas de journal) ;
//   - AUCUNE forme de payload d'un runner particulier ;
//   - des fonctions pures, deterministes, testables par fixtures.
// Chaque runner fournit un ADAPTATEUR mince qui : parse SON payload -> reconstruit l'entree
// canonique -> appelle un verdict d'ici -> traduit le verdict dans SA semantique (exit code,
// exception middleware, annotation de reponse...).
//
// STRATEGIE ALIGN (calquee sur cli/src/lib/vocab.js <-> core vocab.json) : ce fichier est
// DUPLIQUE a l'identique dans chaque kit runner (kit-claude, kit-codex...) pour rester
// self-contained au deploiement. Un test de parite (cli/test/guard-core-parity.test.js) verrouille
// que toutes les copies sont OCTET POUR OCTET identiques -> pas de re-divergence de la regle.
//
// NE PAS ajouter d'I/O ni de dependance ici : la purete est la garantie de portabilite.

// ---------------------------------------------------------------------------
// Verdict IDENTITE (badges d'ouverture / cloture). La POSITION de la pastille porte le sens :
//   ouverture = pastille AVANT le bloc  ->  🟡 [ROYAUME][Agent]
//   cloture   = pastille APRES le bloc  ->  [ROYAUME][Agent] 🟡
// ---------------------------------------------------------------------------

export const PASTILLES = [0x1f7e1, 0x1f535, 0x1f534, 0x1f7e2, 0x1f7e3, 0x1f7e0]
  .map((cp) => String.fromCodePoint(cp));
const pastAlt = PASTILLES.map((p) => p.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|");
const bracket = "\\[[^\\]]+\\]\\s*`?\\s*\\[[^\\]]+\\]"; // [ROYAUME][Agent]
export const reOpen = new RegExp("^(?:" + pastAlt + ")\\s*`?\\s*" + bracket);
export const reClose = new RegExp(bracket + "\\s*`?\\s*(?:" + pastAlt + ")(?:\\s|$)");

export const linesOf = (txt) =>
  txt.split("\n").map((l) => l.replace(/\s+$/, "")).filter((l) => l.trim() !== "");

// Verdict IDENTITE pur. Entree canonique = `turn` : les messages-texte de l'assistant du TOUR
// courant, du plus RECENT au plus ANCIEN (turn[0] = dernier message du tour). L'adaptateur runner
// est responsable de reconstruire ce tableau depuis son transcript.
// Renvoie :
//   { skip: true }                      -> tour vide -> rien a juger (l'adaptateur => allow)
//   { skip: false, startOk, stopOk }    -> verdict sur le tour courant
export function verdictIdentity(turn) {
  if (!Array.isArray(turn) || turn.length === 0) return { skip: true };

  // Ouverture : acceptee si N'IMPORTE QUEL message-texte du tour ouvre par un badge.
  const opensWith = (txt) => {
    const ne = linesOf(txt);
    return ne.length > 0 && reOpen.test(ne[0].trim());
  };
  let startOk = turn.some(opensWith);

  // Cloture : portee par le DERNIER message-texte du tour (turn[0]).
  const nonEmpty = linesOf(turn[0]);
  let stopOk;
  if (nonEmpty.length === 1) {
    const single = reOpen.test(nonEmpty[0].trim()) || reClose.test(nonEmpty[0].trim());
    stopOk = single;
    // Tour reduit a un unique one-liner : on tolere ouverture OU cloture pour les deux.
    if (turn.length === 1) startOk = single;
  } else {
    stopOk = false;
    const idxs = [nonEmpty.length - 1];
    if (nonEmpty.length >= 3) idxs.push(nonEmpty.length - 2);
    for (const idx of idxs) {
      if (idx === 0) continue;
      if (reClose.test(nonEmpty[idx].trim())) { stopOk = true; break; }
    }
  }

  return { skip: false, startOk, stopOk };
}

// ---------------------------------------------------------------------------
// Verdict PERIMETRE (chemins hors projet). Pur : recoit des chemins DEJA resolus (absolus) et
// les reperes du perimetre ; ne touche pas au disque, ne lit pas d'env. L'adaptateur runner
// resout les chemins (cwd, tilde) et fournit portfolioDir/harnessSettings propres a son foyer.
// ---------------------------------------------------------------------------

// Appartenance : `target` est SOUS `base` si le chemin relatif ne sort pas (pas de "..",
// pas un chemin absolu). base===target -> dans la base (la racine elle-meme).
// `relativeFn` = implementation de path.relative (injectee par l'adaptateur : Node `node:path`).
export function isUnder(base, target, relativeFn, isAbsoluteFn) {
  if (!base || !target) return false;
  const rel = relativeFn(base, target);
  return rel === "" || (!rel.startsWith("..") && !isAbsoluteFn(rel));
}

// Classe un chemin absolu resolu contre perimetre + reperes du foyer runner.
// Renvoie : ALLOW_PROJECT | ALLOW_PORTFOLIO | DENY_HARNESS | HORS
//   - projectDir       : racine du projet courant (perimetre autorise) ;
//   - portfolioDir     : racine du portefeuille runner (ex. ~/.claude) -> ALLOW_PORTFOLIO ;
//   - harnessSettings  : fichier de config du harnais reserve a l'humain -> DENY_HARNESS (prime).
export function verdictPerimeter(absPath, projectDir, opts) {
  const { portfolioDir, harnessSettings, relativeFn, isAbsoluteFn } = opts || {};
  // DENY harnais prime sur tout.
  if (harnessSettings && absPath === harnessSettings) return "DENY_HARNESS";
  if (projectDir && isUnder(projectDir, absPath, relativeFn, isAbsoluteFn)) return "ALLOW_PROJECT";
  // ALLOW portefeuille (hors settings deja capte ci-dessus).
  if (portfolioDir && isUnder(portfolioDir, absPath, relativeFn, isAbsoluteFn)) return "ALLOW_PORTFOLIO";
  return "HORS";
}

export const isPerimeterBlocking = (verdict) => verdict === "HORS" || verdict === "DENY_HARNESS";

// ---------------------------------------------------------------------------
// Verdict DELEGATION (roster). Pur : appartenance d'un agent cible au roster iakaframe (ou aux
// sous-agents natifs toleres). L'adaptateur runner extrait `agent` de son payload et applique la
// semantique (refus = exit 2 chez Claude/Codex, journal verbatim, etc.).
// ---------------------------------------------------------------------------

// ⚠️ `feanor` MANQUE ICI, et ce n'est pas un oubli de la scission du squad prod : il manquait
// DEJA. L'ajouter changerait le comportement de delegation de Fëanor (`Task(agent: feanor)` est
// refuse aujourd'hui), ce que personne n'a demande — un changement non arbitre passe sous couvert
// d'un autre lot. Constate, NON corrige. Ticket : `ROSTER-FEANOR`.
export const ROSTER = Object.freeze([
  "odin", "aragorn", "gandalf", "gimli", "legolas", "charon", "helm", "loki", "nathalie",
]);
// Sous-agents natifs toleres hors roster iakaframe (Claude Code, valeurs par defaut).
export const BUILTINS = Object.freeze([
  "Explore", "Plan", "general-purpose", "claude", "claude-code-guide", "statusline-setup",
]);
export const AGENT_UNSET = "(non precise)";

// Verdict DELEGATION pur. Renvoie { known, refused } :
//   - known   : l'agent est du roster iakaframe OU un sous-agent natif tolere ;
//   - refused : agent inconnu ET explicitement precise -> a refuser (exit 2 chez les CLIs).
// Un agent absent (AGENT_UNSET) n'est jamais refuse (rien a juger).
export function verdictDelegation(agent, opts) {
  const roster = (opts && opts.roster) || ROSTER;
  const builtins = (opts && opts.builtins) || BUILTINS;
  const rosterSet = new Set(roster.map((a) => String(a).toLowerCase()));
  const builtinSet = new Set(builtins);
  const a = agent == null ? AGENT_UNSET : String(agent);
  const known = rosterSet.has(a.toLowerCase()) || builtinSet.has(a);
  const refused = !known && a !== AGENT_UNSET;
  return { known, refused };
}
