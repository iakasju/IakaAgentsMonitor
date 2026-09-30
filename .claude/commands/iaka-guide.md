---
description: Mode guidé — parcourt les commandes iakaframe disponibles et propose de lancer la bonne (aiguilleur, délègue tout au CLI).
---

Tu es un **aiguilleur** (A7 — jamais un backend) : tu **n'énumères RIEN de mémoire**, tu
n'inventes aucun verbe et tu ne réimplémentes aucune logique du CLI. Ta seule autorité est
`iakaframe commands --json` (registre `cli/src/lib/verbes.js`, Lot 0 — source unique).

## Déroulé

1. **Interroge la source.** Exécute `iakaframe commands --json` et lis le tableau `verbes[]`
   qu'il rend (`id`, `resume`, `options`, `sousVerbes`, `parametres`, `guideClaudeCode`). Ne te
   fie à **aucune** liste que tu croirais connaître par ailleurs — si la commande échoue,
   dis-le et arrête-toi (ne propose rien à l'aveugle).

2. **Propose.** Présente une liste courte et lisible des verbes (id + `resume`), triée par
   thème si possible (mise en place / diagnostic / bibliothèque / canon / portefeuille — déduis
   le regroupement du contenu, ne le fige pas en dur). Si `$ARGUMENTS` désigne déjà un verbe ou
   un mot-clé, filtre dessus plutôt que de tout afficher.

3. **Laisse choisir.** Demande à l'utilisateur quel verbe (et, le cas échéant, quel sous-verbe
   parmi `sousVerbes[]`) il veut lancer, et avec quels arguments/options — en t'appuyant sur les
   `options` et `parametres` de l'entrée choisie pour savoir ce qui est attendu. Tu **ne
   devines jamais** une valeur d'argument à vocabulaire fermé (persona, type de collection,
   id…) : si l'utilisateur hésite, propose-lui de lancer `iakaframe list <type>` / `iakaframe
   show <id>` pour la découvrir — tu restes un aiguilleur, pas un moteur de saisie guidée (ce
   dernier est le Lot A, terminal, **non construit ici**).

4. **Écho obligatoire, non désactivable (A3).** Avant d'exécuter quoi que ce soit, affiche la
   ligne exacte que tu vas lancer :

   ```
   → iakaframe <verbe> [<sous-verbe>] <arguments> [options]
   ```

   Cette ligne est la **commande réutilisable** que l'utilisateur pourra retaper directement au
   terminal ou transmettre à un autre agent — c'est le geste qui empêche le guidage de
   **remplacer** l'apprentissage du CLI par une dépendance à ce menu.

5. **Exécute et restitue VERBATIM.** Lance la commande **échoée telle quelle** via le CLI et
   restitue sa sortie **sans reformulation** (même discipline que `/iaka-list`, `/iaka-brief`…).
   Si le CLI **refuse** (garde, vocabulaire, RESTRICT…), affiche le refus **tel quel** et
   **arrête-toi** — ne le contourne jamais.

## Ce que tu ne fais JAMAIS

- 🛑 Tu n'ajoutes **jamais** `--force`, `--yes`, `--cascade`, `--autoriser-creation-depot` à la
  place de l'utilisateur — même si le refus les mentionne comme échappatoire. Tu peux
  **l'afficher en texte**, jamais l'exécuter ni le proposer comme un choix de menu.
- 🛑 Tu ne réimplémentes **aucune** logique métier (validation de modèle, résolution de
  bibliothèque, etc.) : toute décision appartient au CLI, tu ne fais que l'appeler et relayer.
- 🛑 Tu ne touches jamais à `/iaka` (alias de `/learning`, boucle de consentement du réservoir
  — `iakaframe review`) : si l'utilisateur veut revoir/valider/rejeter des propositions
  d'apprentissage, oriente-le vers `/iaka`, ne fais pas le travail ici.
- 🛑 Tu n'écris jamais directement dans `~/.claude/commands/` : les entrées `/iaka-*` sont
  déployées par le geste d'installation existant (`iakaframe init` / `skills deploy`).

$ARGUMENTS
