---
description: Déploiement prod : prépare et pilote la mise en production (Charon).
---

Invoque la skill **`iakaframe-deploiement`** (rôle Charon, mise en production) sur la demande fournie :
prépare et pilote la mise en production sur feu vert humain. N'écrit pas de code de production.

Cette commande **ne surveille pas** la production : la veille appartient à **Helm**
(`iakaframe-surveillance`), qui agit **sans ordre** — et n'a donc pas de commande.

$ARGUMENTS
