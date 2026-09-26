---
description: Canon PROJET (connaissance du produit, revisee EN PLACE, versionnee) : init|path|config|list|add|replace|remove
---

<!-- GENERE par cli/scripts/gen-iaka-commands.mjs depuis cli/src/lib/verbes.js (verbe `produit`).
     NE PAS EDITER A LA MAIN : la description derive de `resume`, regenerer via le script. -->

Affiche d'abord la ligne **`→ iakaframe produit $ARGUMENTS`** (la commande effectivement
exécutée), **PUIS** exécute **`iakaframe produit $ARGUMENTS`** et **restitue la sortie VERBATIM**
(aucune reformulation). Commande d'ÉCRITURE : respecte scrupuleusement les gardes du CLI (refus affichés tels
quels, jamais contournés) — ne compose **jamais** `--force`/`--yes`/`--cascade` à la place de
l'utilisateur.

$ARGUMENTS
