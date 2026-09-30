#!/usr/bin/env node
// identity-remind.mjs — Rappel d'identite iakaframe (hook UserPromptSubmit, portage macOS).
// Le stdout est injecte comme contexte additionnel avant la reponse de l'agent.
// Nudge doux, complement du garde bloquant identity-guard.mjs (Stop/SubagentStop).

process.stdout.write(
  "[Garde d'identite iakaframe] Regle de la methode : tout agent qui prend la parole " +
  "s'identifie ET annonce ce qu'il fait. La POSITION de la pastille porte le sens. " +
  "Ouverture = pastille AVANT le bloc, en TOUTE PREMIERE ligne -> <pastille> [ROYAUME][Agent] " +
  "(ex: pastille jaune puis [PORTEFEUILLE][Odin]) suivi d'une courte annonce ; " +
  "cloture = pastille APRES le bloc en derniere ligne -> [ROYAUME][Agent] <pastille>. " +
  "Double badge par intervention. Une delegation = chaine de badges.\n"
);
process.exit(0);
