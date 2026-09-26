---
description: Revoir ce que l'agent a appris — alias de /learning (pilote la boucle de revue du réservoir, skill iakaframe-learning).
---

Alias de `/learning` : active le parcours de **revue d'apprentissage** (skill
`iakaframe-learning`), la fenêtre conversationnelle sur le **réservoir de propositions**, qui
**pilote** la commande `iakaframe review` (source unique — aucune logique de consentement/plafond
réimplémentée ici).

Déroule le parcours, en offrant **valider ET rejeter au même niveau** (symétrie +/−) et en
explicitant le **garde de consentement** (structurel = geste humain requis, jamais auto) :

1. **Lister** — `iakaframe review list` (par défaut les `en-attente` ; `--status applique|rejete`
   pour l'historique ; `--json` pour parser).
2. **Voir** — `iakaframe review show <id>` (quoi / où / **pourquoi** + artefact).
3. **Valider** — `iakaframe review apply <id>` (matérialise via `review`, restitue le résultat ou
   le refus **verbatim** — ne re-décide rien).
4. **Rejeter** — `iakaframe review reject <id>` (statut `rejete`, rien matérialisé) — **geste de
   premier plan**, aussi accessible que valider.

Ne lis/écris **jamais** le réservoir en direct : passe **toujours** par `iakaframe review`.

> **Frontière.** `reject` retire une proposition **en attente**. Défaire un ajout **déjà
> matérialisé** (détacher un skill, retirer une team/method/binding/skill, `memory remove`) relève
> de `/retrait` (skill `iakaframe-retrait`) — un renvoi, pas une duplication.

$ARGUMENTS
