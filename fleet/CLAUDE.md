# CLAUDE.md — Instructions pour Claude Code

> Ce fichier est lu en priorité par Claude Code à chaque session.
> Pour la vision complète du projet, lire `specs/PROJET.md`.
> Pour la méthode de collaboration, voir `methode-de-travail.md` (iakaframe).

---

## Rôles (rappel)

- **Cowork** (réflexion) rédige les instructions dans `specs/instructions/`. Il ne
  modifie jamais le code.
- **Claude Code** (toi) lis l'instruction correspondante AVANT chaque tâche, puis
  implémentes, builds, testes et commites.

---

## Ce qu'est ce projet

<!-- 2-4 lignes : à quoi sert le projet, pour qui, le résultat produit. -->

**fleet/** (ex-iakaFleet, importe dans IakaAgentsMonitor avec son historique) : demon de machine et CLI, en
**lecture seule**. Un **demon par machine** lit `~/.claude/sessions/<pid>.json` et les fiches d'agents
(`~/.cache/iaka-agents/`) et publie sur MQTT sous `iaka/<node>/out/` ; une **console CLI** lit le broker.
> Ce fichier est celui de l'ex-dépôt ; **le `CLAUDE.md` racine du dépôt prime** (commandes, conventions). Vision : `specs/PROJET.md`. Instruction du MVP : `specs/instructions/mvp-vue-flotte-lecture-seule.md`.

Stack : **Node >= 20, ESM, JavaScript sans build**, une seule dependance applicative `mqtt` (MQTT.js, epinglee
exactement, `package-lock.json` commite), tests `node --test` (zero dependance de test).
Broker par defaut `mqtts://mqtt.naonedge.com:8883` (racine de topics `iaka/<node>/out/...`, contrat v2 :
`../specs/contrat-bus-iaka.md`, code dans `src/shared/contract.js`, vecteurs partages `../contract-vectors/`). Config : `~/.iakafleet/.env` (voir `.env.example`, `docs/installation.md`).

Regles dures : **aucun `child_process`** dans `src/` ni `bin/` (garde `G-EXEC`) ; un ordre recu n'est jamais
execute comme un shell ; la verification TLS n'est pas debrayable ; aucun secret dans le depot.

Arborescence : `src/shared/` (contrat, config) · `src/agent/` (sessions, cards, gitHead, iakaframeVersion, publisher,
index) · `src/console/` (fleetModel, render, index) · `bin/` (points d'entree) · `deploy/` (services 3 OS) ·
`test/` (+ `fixtures/sessions/`).

---

## Commandes à utiliser

```bash
npm install                        # une dependance : mqtt (epinglee)
npm test                           # node --test (sans reseau, sans broker)
npm run lint                       # syntaxe + regles du projet (pas de typecheck : JS sans build)
node bin/iakafleet-agent.js        # agent (en pratique : lance par le service, deploy/)
node bin/iakafleet.js ls|watch [--json]   # console (code de sortie 2 = broker/auth/config en echec)
IAKAFLEET_LIVE=1 npm test          # ajoute la mesure reelle de la version d'iakaframe (M3)
```

<!-- Si un Makefile/scripts existent, exiger de passer par eux plutôt que les
     commandes brutes (docker compose, cargo, etc.). -->

---

## Conventions

- **Langue du code** : anglais (identifiants, commits techniques).
- **Langue de la doc et des échanges** : français.
- **Commits** : *conventional commits* (`feat:`, `fix:`, `docs:`, `chore:`, `wip:`).
- **Commits atomiques et fréquents** : après chaque étape logique (filet de
  sécurité pour pouvoir revenir en arrière). Jamais de `reset --hard` ni de
  `push --force` de ton côté.
- **MVP d'abord, puis itérer.** Pas de sur-ingénierie.
- **Self-hosted / open-source d'abord** pour tout choix de backend ; cloud en
  fallback justifié seulement.
- **Réutiliser l'existant** (infra, services, MCP) avant de réimplémenter.
- En dev, **mocker les appels API** coûteux/limités (voir `specs/mock/`).

---

## Dépôt git : Forgejo VPS (git.naonedge.com)

Remote `origin` : **Forgejo du VPS NaonEdge** `https://git.naonedge.com/sjupin/iakaFleet.git`,
**HTTPS + token** (l'ancien Forgejo LAN iakabox est mort ; le NAS reste un miroir secondaire sous
le nom `nas`). Token via `$FORGEJO_TOKEN` ou `.git/config` local — **jamais commité**. Création de
dépôt par l'API (description **ASCII**). Détail : skill `iakaframe-forgejo`.

## Cycle de documentation (état des lieux)

Régénérer l'état des lieux **à chaque changement de version** et **à chaque pause /
préparation de reprise** :

```bash
iakaframe snapshot --reason version --version vX.Y.Z --note "..."
iakaframe snapshot --reason pause   --note "où on s'arrête, quoi reprendre"
iakaframe snapshot --reason reprise --note "reprise"
```

Génère `specs/etat-des-lieux.md` + `.html` (faits git auto). **Compléter le récit de
reprise** dans le `.md` (ce qui vient d'être fait, ce qui reste, prochaine étape).

---

## Avant toute tâche non triviale

1. Lire l'instruction correspondante dans `specs/instructions/`.
2. Si elle n'existe pas → le signaler ; ne pas improviser une feature lourde sans
   spec. Proposer un plan court d'abord.
3. Implémenter étape par étape, avec commits intermédiaires.
4. Lancer typecheck + lint + tests avant de considérer la tâche finie.
5. Pour toute action vraiment destructive hors denylist : **demander confirmation
   par message texte avant d'agir.**

---

## Backlog

<!-- Liste des features priorisées. Chaque entrée pointe vers son instruction. -->

- [~] MVP vue flotte lecture seule — `specs/instructions/mvp-vue-flotte-lecture-seule.md` (P2 en cours ; recette
      reelle en attente du Lot 0 et de l'acces au PC Windows)
- [ ] Prerequis Lot 0 : ACL + comptes MQTT `iakafleet/` — `naonedge-vps/specs/instructions/mqtt-acl-iakafleet.md`
- [ ] Ordres avec accuse de reception — `specs/instructions/ordres-accuse-reception.md` (a cadrer)
- [ ] Autorisations / gestes destructifs — `specs/instructions/autorisations-gestes-destructifs.md` (a cadrer)
- [ ] Console graphique, liaison iakaHub, source par hooks (optionnels, a cadrer)
