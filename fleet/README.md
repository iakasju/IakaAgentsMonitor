# fleet/ - demon de machine et CLI `iakafleet`

Piece du produit **IakaAgentsMonitor** (importee d'iakaFleet, historique conserve). Le demon
`iakafleet-agent` tourne en service sur **chaque** machine, **sans GUI** : il lit en lecture seule
les fichiers de session de Claude Code et les fiches d'agents ecrites par le publieur de hooks,
puis publie sur le broker central sous `iaka/<node>/out/...`. La CLI `iakafleet ls|watch|--json`
lit la flotte (machines, sessions, agents).

- Contrat de bus (source unique) : `../specs/contrat-bus-iaka.md` (code : `src/shared/contract.js`)
- Instruction de fusion : `../specs/instructions/fusion-flotte-vue-distante.md`
- Vecteurs de contrat partages : `../contract-vectors/`
- Installation, variables, etats affiches : `docs/installation.md`
- Consignes de travail (commandes, conventions) : `../CLAUDE.md` (racine), qui prime. `fleet/` n'a pas de
  `CLAUDE.md` propre : le squelette de l'ex-depot a ete retire (instruction fusion § 13.2).

## Stack

**Node >= 22.14** (`engines` ; modele de permissions de Node stable, cf. regles dures), ESM, JavaScript sans build, une seule
dependance applicative `mqtt` (epinglee exactement, `package-lock.json` commite), tests `node --test`
(zero dependance de test). Broker par defaut `mqtts://mqtt.naonedge.com:8883`. Config :
`~/.iakafleet/.env` (modele : `.env.example`).

Arborescence : `src/shared/` (contrat, config) - `src/agent/` (sessions, cards, gitHead, iakaframeVersion,
publisher, index) - `src/console/` (fleetModel, render, index) - `bin/` (points d'entree) - `deploy/`
(services 3 OS) - `test/` (+ `fixtures/`) - `scripts/lint.mjs`.

## Commandes

```bash
npm ci                             # une dependance : mqtt (epinglee)
npm test                           # node --test (aucune connexion externe ; faux broker sur 127.0.0.1 pour permission.test.js)
npm run lint                       # syntaxe + regles du projet (JS sans build : pas de typecheck)
node bin/iakafleet-agent.js        # demon (en pratique lance par le service, deploy/)
node bin/iakafleet.js ls|watch [--json]   # console (code de sortie 2 = broker/auth/config en echec)
IAKAFLEET_LIVE=1 npm test          # ajoute la mesure reelle de la version d'iakaframe (M3)
```

## Regles dures

- **Aucun `child_process`** (ni worker, vm, cluster, eval, `Function`, import dynamique, `require`,
  `process.binding`...) dans `src/` ni `bin/`. Deux couches (`G-EXEC-2`) :
  1. **statique** : liste blanche MOTIVEE de modules importables (`test/guard-exec.test.js`) ; hors
     couverture declare : le code des dependances (`mqtt`), couvert par la couche 2 et le lockfile ;
  2. **execution** : les gabarits de service lancent le demon avec `node --permission` (lectures
     accordees une a une, **sans** `--allow-child-process`, `--allow-worker`, `--allow-fs-write`) :
     tout lancement de processus echoue en `ERR_ACCESS_DENIED` (`test/permission.test.js`).
- Un ordre recu n'est jamais execute comme un shell.
- La verification TLS n'est pas debrayable.
- Aucun secret dans le depot (Vaultwarden `work/iakaFleet`, config dans `~/.iakafleet/.env`).
- Commits : *conventional commits*, atomiques ; jamais de `reset --hard` ni de `push --force`.

## Historique de l'import

Les commits d'iakaFleet (`feat/mvp-vue-flotte`, tete `6ae1eca`) sont conserves dans le graphe comme
**second parent** du commit d'import (`c499c78`), a leurs **chemins d'origine** (sans prefixe `fleet/`).
`git log -- fleet/` ne montre donc que les commits posterieurs. Pour lire l'ancien historique :

```bash
git log c499c78^2 -- src/agent/publisher.js      # chemins d'origine, sans "fleet/"
git show c499c78^2:CLAUDE.md                     # ex-CLAUDE.md / specs/PROJET.md / instruction MVP
git rev-parse c499c78^2                          # = 6ae1ecae1128214926092eeeacb50c4cd67d1dc6
```

Le squelette retire (`.claude/commands/`, `global/`, `iakaframe.json`, `.iakaframe`, `specs/`,
`CLAUDE.md`) et l'instruction du MVP (`specs/instructions/mvp-vue-flotte-lecture-seule.md`) y restent lisibles.
