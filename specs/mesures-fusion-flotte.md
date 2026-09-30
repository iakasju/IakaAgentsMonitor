# Mesures de l'étape 0 — fusion iakaFleet → IakaAgentsMonitor

> Consignées par Gimli (P2) le **2026-09-30**, avant tout code (instruction `fusion-flotte-vue-distante.md`,
> § 2 et § 7 étape 0). Une mesure est **mesurée** (avec sa valeur) ou **non mesurée** (avec la raison).

| Id | État | Valeur |
|---|---|---|
| **M1** contenu de `feat/mvp-vue-flotte` | **mesurée** | 11 commits `main..feat/mvp-vue-flotte` (de `221331f` skeleton à `6ae1eca` docs). **111 fichiers** dont 68 hors squelette iakaframe : `src/` (agent 5 fichiers, console 3, shared 2), `bin/` (2), `test/` (13 fichiers de test + fixtures + faux client MQTT), `deploy/` (launchd, systemd, Windows), `docs/installation.md`, `scripts/lint.mjs`. Dépendance **`mqtt` 5.16.0** épinglée (`package.json`). **Tests : 65 (64 verts, 1 sauté)** après import, `npm test` de `fleet/` **sans modification** ; lint : 26 fichiers, 0 problème. Le **verdict Legolas n'est pas lu** (aucun rapport dans les objets git) : « gate PASS » reste **rapporté**, non vérifié ici. |
| **M2** `bridgeSessionId` | **mesurée** | Sur les 3 fichiers `~/.claude/sessions/*.json` de ce Mac : clé **absente** sur 2, présente avec la valeur **`null`** sur 1. Aucune session avec `/rc` (chaîne non vide) observée. La formulation de `F6` du MVP Fleet (« n'apparaît que sur la session Remote Control ») est donc **infirmée** (la clé existe aussi à `null`). **Le code était déjà correct** : `rc = typeof bridgeSessionId === 'string' && !== ''` (`fleet/src/agent/sessions.js`), test `M5 : rc = chaîne non vide ; clé présente mais null ⇒ false`. **Rectifié : la doc seulement** (`fleet/docs/installation.md`). |
| **M3** racine `iaka/` hors Monitor | **mesurée** | `grep` sur `~/work` (hors `node_modules`, `.git`, `target`, `.wt`, `dist`) : **aucun producteur ou consommateur** de la racine `iaka/` hors IakaAgentsMonitor. Seules occurrences hors dépôt : les spécifications `naonedge-vps/specs/instructions/mqtt-acl-iakafleet.md` (amendée pour `iaka/<node>/out/#`) et les specs/`PROJET.md` d'iakaFleet. Les racines `iakatokencounter/`, `iakaframe/`, `iakaboxlogs/` restent disjointes. |
| **M4** TLS `rumqttc` → `mqtts://mqtt.naonedge.com:8883` | **non mesurée** | Dépend du Lot 0 amendé, exécuté ailleurs. Aucune connexion vers `mqtt.naonedge.com` n'a été tentée dans ce lot (faux broker + `rumqttd` éphémère sur la boucle locale). Le chemin TLS est câblé (`Transport::tls_with_default_config`, racines système, panic converti en motif) mais **non exercé**. |
| **M5** `git subtree` | **mesurée** | Disponible (`git-subtree` dans `/Library/Developer/CommandLineTools/usr/libexec/git-core`, git 2.50.1). `git subtree --help` sort en code 1 faute de page de manuel, mais la sous-commande `add` fonctionne (import réalisé). |

## Constats annexes de l'import

- Le sous-arbre rapporte **tout le contenu de la branche à la racine de `fleet/`**, y compris le squelette iakaframe
  du dépôt source (`fleet/.claude/`, `fleet/global/`, `fleet/iakaframe.json`, `fleet/.iakaframe`, `fleet/CLAUDE.md`,
  `fleet/specs/`). Import laissé **tel quel** (étape 1 : « sans modification ») ; nettoyage éventuel = décision du décideur.
- `git log -- fleet/` ne montre que le commit d'import : un `git subtree add` **sans `--squash`** garde les 11 commits
  d'iakaFleet dans l'histoire (`git log --oneline` : 83 commits), mais leurs chemins sont à la racine, pas sous `fleet/`.
  Pour lire l'historique d'un fichier : `git log --follow -- fleet/<fichier>` ou `git log 6ae1eca`.
