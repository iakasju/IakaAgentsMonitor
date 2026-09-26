---
description: Deploie l'union resolue des skills -> <cible>/.claude/skills/ (non destructif)
---

<!-- GENERE par cli/scripts/gen-iaka-commands.mjs depuis cli/src/lib/verbes.js (verbe `skills`).
     NE PAS EDITER A LA MAIN : la description derive de `resume`, regenerer via le script. -->

Affiche d'abord la ligne **`→ iakaframe skills $ARGUMENTS`** (la commande effectivement
exécutée), **PUIS** exécute **`iakaframe skills $ARGUMENTS`** et **restitue la sortie VERBATIM**
(aucune reformulation). Commande d'ÉCRITURE : respecte scrupuleusement les gardes du CLI (refus affichés tels
quels, jamais contournés) — ne compose **jamais** `--force`/`--yes`/`--cascade` à la place de
l'utilisateur.

$ARGUMENTS
