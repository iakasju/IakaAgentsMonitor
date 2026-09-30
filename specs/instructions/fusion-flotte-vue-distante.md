# Instruction : fusion iakaFleet → IakaAgentsMonitor, et vue distante de la flotte (lecture seule)

> Cadrée par **Gandalf** (P1 — cadrage) le **2026-09-30**, sur **décision du décideur, non
> rouvrable** : *« agentsmonitor remonte d'un cran et regarde les autres machines/sessions en
> distance avec le mqtt vers le mqtt de naonedge vps »* — iakaFleet et IakaAgentsMonitor
> **fusionnent**. Exécution : **Gimli** (P2) après validation du jalon P1→P2.
> Contrat de bus associé (source unique) : `specs/contrat-bus-iaka.md`.
>
> ✅ **Cadrage VALIDÉ par le décideur le 2026-09-30** (P1→P2 franchi), recommandations retenues :
> Q1 texte de tâche **masqué par défaut** sur le central (`D3`, `D6`, `CA-5`) · Q2 groupement
> **minimal** dans le tableau V4, sans maquette (`D4`) · Q3 hooks + démon installés **après le Lot 0
> amendé**, Mac d'abord (lot `L-INSTALL`, § 7 étape 10) · Q4 iakaFleet **archivé après l'import** par le
> décideur, noms `iakafleet` **conservés** (`D1`, `D6`).
>
> Conventions : `F*` fait vérifié · `M*` mesure à faire par Gimli (le cadrage n'a pas de shell) ·
> `D*` décision · `R*` risque · `CA-*` critère.

---

## 1. Problème

Deux produits voient la même chose à deux étages. **IakaAgentsMonitor** montre, sur **un** poste, le
détail riche des agents (persona, tâche, délégation, modèle). **iakaFleet** montre, sur **toutes**
les machines, la présence et l'état des sessions (testament MQTT, battement), sans ce détail. Le
décideur veut **un seul produit** : le Monitor, qui garde sa vue locale et **voit en plus les autres
machines et leurs sessions** par le broker du VPS. Il faut décider où vit le code, unifier les trois
conventions de topics en présence, et migrer le travail livré sans le refaire.

## 2. Faits

| Id | Fait | Preuve |
|---|---|---|
| **F1** | Monitor : `main` = **`a27a3c8`** ; Tauri 2, `rumqttc 0.24` + `toml`, `rumqttd 0.19` en dev | `IakaAgentsMonitor/.git/refs/heads/main` · `src-tauri/Cargo.toml:19-27` |
| **F2** | Le lot MQTT de base du Monitor est **dans le code** : `sources/bus.rs`, `card.rs`, `tree.rs`, `roster.rs`, `labels.rs`, publieur `publishers/claude-code/` + tests | arborescence `src-tauri/src/**`, `publishers/claude-code/**` |
| **F3** | La source bus du Monitor a l'hôte **forcé à `127.0.0.1`** (« aucune variable pour le changer »), port `IAKAAM_MQTT_PORT` ; même règle dans le publieur | `src-tauri/src/sources/bus.rs:6,25,113` · `publishers/claude-code/iaka-agents-publish.mjs:24,29` |
| **F4** | Topic actuel des fiches : `iaka/agents/out/${RUNNER_ID}/${sessionId}/${agentId}` | `iaka-agents-publish.mjs:355` |
| **F5** | 🛑 **Le publieur écrit la fiche courante sur disque AVANT de la publier** : `~/.cache/iaka-agents/<sessionId>/<agentId>.json` = `{card, lastPublishedAt}` ; dossier de session supprimé au `SessionEnd`. ⇒ **un démon peut lire les fiches sans aucun broker local** | `iaka-agents-publish.mjs:36,126,136,404-405` |
| **F6** | 🛑 **Les hooks du publieur ne sont PAS installés** : 0 occurrence de `iaka-agents-publish` dans `~/.claude/settings.json`. Aucune fiche n'a donc jamais été produite en réel | recherche du 2026-09-30 |
| **F7** | Le broker local est l'**iakahub d'iakaTokenCounter** (`rumqttd`, anonyme, boucle locale, **en mémoire**, vit tant que la tray iakaTokenCounter tourne) | `specs/instructions/agnostique-mqtt.md:33-38` |
| **F8** | `rumqttc 0.24` a la fonctionnalité **`use-rustls` par défaut** : TLS disponible sans nouvelle crate | [docs.rs — rumqttc 0.24.0 features](https://docs.rs/crate/rumqttc/0.24.0/features) |
| **F9** | ⚠️ **Correction d'un fait rapporté** : l'amendement A1 (pause/reprise/arrêt) n'est **pas** « non arbitré » — il a été **arbitré QC1 à QC9 le 27/09** (HMAC + anti-rejeu + `kid` « pour préparer un MQTT central sur le VPS pilotant plusieurs machines », pause ≤ 10 min, choix de modèle inclus). Il n'est **pas codé** | `agnostique-mqtt.md:598` |
| **F10** | A1 vise `iaka/agents/in/<runner>/<sessionId>/<agentId>` | `agnostique-mqtt.md:464` |
| **F11** | iakaFleet : `main` = `6599d0b`, branche `feat/mvp-vue-flotte` = **`6ae1eca`** (locale et `origin`), arbre de travail sur `main` (aucun `src/` visible). Contenu de la branche (11 commits, gate Legolas PASS) **rapporté par Odin, NON LU** par le cadrage (objets git non lisibles sans shell) → `M1` | `iakaFleet/.git/refs/heads/{main,feat/mvp-vue-flotte}` |
| **F12** | Lot 0 (ACL/comptes VPS) cadré, **validé, non exécuté**, sur la racine `iakafleet/` | `~/work/naonedge-vps/specs/instructions/mqtt-acl-iakafleet.md` |
| **F13** | Le VPS réserve `iakaframe/<projet>/<agent>/<evenement>` (compte `iakaframe-publisher`), **non implémenté** par aucun publieur | `~/work/naonedge-vps/specs/instructions/mqtt-broker-vps.md` § 5 |
| **F14** | BACKLOG Lot 2 : `/<node>/<depot-projet>/<agent>/…` ; Lot 6 : ordres via `…/cmd`, allowlist, identité signée, confirmation, journal | `~/work/BACKLOG.md:589-606` |
| **F15** | Rapport Gimli/Legolas (via Odin) : `bridgeSessionId` est `null` sur une session sans `/rc`, ce qui **infirme** `F6` du MVP Fleet tel que formulé ⇒ le code `rc` n'est pas fiable en l'état. Constat exact **NON LU** → `M2` | ordre de mission |
| **F16** | Les `CLAUDE.md` des deux dépôts portent encore le gabarit « Forgejo iakabox `192.168.2.11` » | `IakaAgentsMonitor/CLAUDE.md:80-85` · `iakaFleet/CLAUDE.md:58-63` |

### Mesures confiées à Gimli (étape 0, avant tout code)

| Id | À mesurer |
|---|---|
| **M1** | `git -C ~/work/iakaFleet log --oneline main..feat/mvp-vue-flotte` + `ls-tree -r --name-only feat/mvp-vue-flotte` + verdict Legolas : consigner fichiers, tests, version de `mqtt` |
| **M2** | Le constat exact sur `bridgeSessionId` (présent/absent/`null` avec et sans `/rc`) |
| **M3** | Aucun usage existant de la racine `iaka/` hors Monitor : `grep -rn "iaka/" ~/work/*/src* ~/work/*/publishers` (hors `node_modules`) |
| **M4** | Connexion TLS `rumqttc` (racines `rustls-native-certs`) à `mqtts://mqtt.naonedge.com:8883` : un test jetable **après** exécution du Lot 0 amendé |
| **M5** | `git subtree` disponible (`git subtree --help`) |

## 3. Décisions

### `D1` — **Un seul dépôt : IakaAgentsMonitor.** iakaFleet y entre comme composant `fleet/`

Le Monitor **est** le produit (décision). Il comprend désormais **trois pièces à frontière nette** :

| Pièce | Où | Rôle | Tourne |
|---|---|---|---|
| **App** (GUI) | `src-tauri/`, `src/` (existant) | vue locale + **vue distante** | quand l'utilisateur l'ouvre (Mac d'abord) |
| **Publieur de hooks** | `publishers/claude-code/` (existant) | fiches d'agents, par hooks | dans chaque session Claude Code |
| **Démon de machine + CLI** | **`fleet/`** (importé d'iakaFleet) | présence, sessions, **relais des fiches vers le central** ; CLI `iakafleet` | en service, sur **chaque** machine, **sans GUI** |

**Pourquoi un seul dépôt plutôt que deux** : un contrat, un backlog, une version, des vecteurs de test
partagés (`contract-vectors/`) exécutés par les trois pièces dans la **même** CI ; deux dépôts
obligeraient à synchroniser à la main une convention que le décideur veut unique. Le dépôt mélange déjà
Rust et Node (publieur). **Pourquoi le démon reste séparé de l'app** : la présence d'une machine doit
exister **sans GUI** (PC Windows, serveurs, VPS) — une app de barre de menus fermée ne peut pas porter
un testament ; et un démon obligatoire dans l'app ferait de l'app un **centre**.

**Sort d'iakaFleet** : son code entre dans `fleet/` **avec son historique** (`git subtree add
--prefix=fleet <iakaFleet> feat/mvp-vue-flotte`) ; la branche **n'est pas fusionnée** dans le `main`
d'iakaFleet ; le dépôt reçoit un `README.md` de renvoi puis est **archivé** sur Forgejo **après l'import**
(geste du décideur, jamais d'un agent — Q4). Sa spec reste en place, marquée **absorbée** (historique de
décision). Les noms **`iakafleet`** (CLI, `~/.iakafleet/.env`, Vaultwarden `work/iakaFleet`) sont
**conservés** (Q4).

### `D2` — **Une seule convention de topics** : `iaka/<node>/{out,in}/…` — cf. `specs/contrat-bus-iaka.md`

Réconciliation des trois conventions : `<node>` = le `<host>` de Fleet, **en second segment** (intention
du Lot 2) ; le sens `out`/`in` du Monitor **porte l'ACL** ; trois granularités sous `out/` :
**machine** (`presence/`), **session** (`sessions/<pid>/`), **agent** (`agents/<runner>/<sid>/<agentId>`).
Le projet est un champ, pas un segment (motif au contrat § 1). La même convention vaut **sur le bus
local et sur le central** — le publieur et la source bus migrent **maintenant**, tant que les hooks
n'ont jamais été installés (`F6`) : la migration ne coûtera jamais moins cher.

### `D3` — Le démon **relaie les fiches depuis les fichiers**, pas depuis le bus local

Le démon lit `~/.cache/iaka-agents/**/*.json` (`F5`) comme il lit `~/.claude/sessions/*.json`, et publie
vers le **central** sous `iaka/<node>/out/agents/…`, en appliquant la confidentialité (sid haché, `cwd`
ramené au home, **`task` à `null` par défaut**, publié seulement si `IAKA_SHOW_TASKS=1` sur la machine — Q1). **Aucune dépendance** au bus local ni à iakaTokenCounter ; zéro
changement au publieur hors son topic. Fiche disparue du disque ⇒ effacement retenu (même règle
`G-PURGE` que les sessions). **`child_process` reste interdit** dans `fleet/` (`G-EXEC` ; durcissement `G-EXEC-2`, § 13.1).

### `D4` — L'app : « **Ce poste** » par les sources locales, « **Autres machines** » par le central

- **Ce poste** : exactement comme aujourd'hui (bus local, repli fichiers). **Rien ne change si aucun
  broker central n'est configuré** : le Monitor marche seul.
  ⚠️ **AMENDÉ le 2026-09-30** après recette : « exactement comme aujourd'hui » était mal posé (la vue locale
  montrait moins que `iakafleet ls`). « Ce poste » est désormais alimenté par les **mêmes sources disque
  que le démon** (toutes les sessions vivantes et tous leurs agents), avec un titre — cf.
  **`specs/instructions/ce-poste-vue-locale-lisibilite.md`** (`D1`–`D4`). « Le Monitor marche seul » reste vrai.
- **Autres machines** : nouvelle source `sources/remote.rs` — `rumqttc` en TLS (`F8`), abonnement
  `iaka/+/out/#`, identifiants du compte **de la machine** (`fleet-<node>`, lecture de toute la flotte)
  ou `fleet-console`, lus dans `~/.iakafleet/.env` (fichier du démon, `D6`). Les messages de **son propre
  `<node>`** sont **ignorés** (pas de doublon avec la vue locale).
- **États de machine** : la règle du MVP Fleet `D9` (en ligne / muette / hors ligne / inconnue /
  sessions illisibles ; sessions d'une machine non en ligne affichées « ? dernier connu ») est
  **portée en Rust** et **prouvée par les mêmes vecteurs** que la CLI (`contract-vectors/`).
- **Absence jamais déguisée en vide** : central configuré mais injoignable / refusé ⇒ la section
  « Autres machines » affiche **« broker injoignable : <motif> »**, jamais une liste vide.
- **Présentation** : le tableau V4 existant, **groupé par machine** (en-tête de groupe = nom, OS, état,
  depuis quand), puis sessions, puis agents indentés sous leur session quand le `sid` se rattache
  (`sha256(sessionId)` du fichier de session = `<sid>` de la fiche) ; sinon « agents non rattachés ».
  Jetons V4, pas de nouveau composant. **Groupement minimal, sans maquette** (décideur, Q2) ; à rouvrir
  seulement si le tableau devient illisible avec deux machines. Une fiche distante dont `task` est
  `null` affiche « tâche masquée », jamais une cellule vide.
- L'app **n'ouvre toujours aucun port** ; elle ne publie **rien** sur le central.

### `D5` — CLI d'abord, GUI ensuite — dans cet ordre de livraison

La CLI `iakafleet ls|watch|--json` (existante) gagne les **agents** sous chaque session (persona, tâche,
durée) ; elle est livrée et recettée **avant** la vue distante de l'app. Décision du décideur (Q4) ; le
GUI **est** le Monitor.

### `D6` — Configuration et secrets

Inchangés du MVP Fleet `D10` (`~/.iakafleet/.env`, variables `IAKAFLEET_*`), avec deux ajouts :
`IAKA_NODE` (prioritaire sur `IAKAFLEET_HOST`, conservé en alias), `IAKA_MASK_NAMES` (défaut
désactivé ; alias `IAKAFLEET_MASK_NAMES`) et **`IAKA_SHOW_TASKS`** (défaut **désactivé** ⇒ tâches
masquées vers le central, Q1) — § 4 du contrat. TLS obligatoire hors boucle locale, certificat vérifié, non débrayable.
Secrets : Vaultwarden `work/iakaFleet` (nom conservé pour ne pas rouvrir le Lot 0 validé), jamais en
clair ni commités.

### `D7` — Les ordres : **un seul point d'entrée**, pas de détail ici

Le futur envoi d'ordres converge **trois** chantiers : l'amendement A1 du Monitor (arbitré, `F9`), le
Lot 6 du BACKLOG, et `ordres-accuse-reception` d'iakaFleet. Point d'entrée unique :
**`iaka/<node>/in/agents/<runner>/<sid>/<agentId>`**, reçu par le **démon de `<node>`**, **remis à la
session** (mécanisme à instruire : contrôleur de hooks A1, channels MCP), qui l'applique **sous ses
propres permissions**. 🛑 **Jamais exécuté comme un shell** par le démon ni par l'app : le « verbe
`iakaframe bus` qui exécute une allowlist » du Lot 6 est à ré-examiner à cette aune dans l'instruction
dédiée. Rien de `in/` n'est codé ni autorisé par l'ACL dans ce lot.

## 4. Contraintes permanentes (gravées)

1. Un ordre reçu est **remis à la session Claude**, qui l'applique sous **ses** permissions ; **jamais
   exécuté comme un shell** par un composant du produit. `fleet/` n'importe pas `child_process`.
2. **TLS** vers tout broker non local ; **un identifiant MQTT par machine** ; **ACL par branche** (`out`
   écrit par sa seule machine).
3. **Secrets dans Vaultwarden**, jamais en clair, jamais commités.
4. **Confidentialité** : pas d'identifiant de session ni de socket vers le central ; `cwd` au home ;
   **texte de tâche masqué par défaut** (activable par machine) ; masquage facultatif des noms de session.
5. **Aucun centre obligatoire** : le Monitor marche seul ; le broker central, le bus local, iakaHub et
   iakabox sont facultatifs.
6. **Pas d'écriture dans `naonedge-vps`** depuis ce lot : l'ACL passe par l'amendement de
   `mqtt-acl-iakafleet.md` (§ 5).
7. **CLI d'abord, puis GUI.**

## 5. Amendement de `naonedge-vps/specs/instructions/mqtt-acl-iakafleet.md`

> **Écrit le 2026-09-30** par le cadrage, sur ordre d'Odin, dans ce fichier-là (en-tête requalifié
> « AMENDÉE, à RE-VALIDER »). Elle doit être **re-validée par le décideur avant exécution**.

| # | Changement |
|---|---|
| A-1 | Racine `iakafleet/<host>/#` ⇒ **`iaka/<node>/out/#`** dans tout le document (bloc ACL, CA-1 à CA-5, commentaires) |
| A-2 | `fleet-console` : `topic read iakafleet/#` ⇒ **`topic read iaka/+/out/#`** |
| A-3 | `fleet-<node>` : `topic readwrite iakafleet/<node>/#` ⇒ **`topic readwrite iaka/<node>/out/#`** **+ `topic read iaka/+/out/#`** (l'app Monitor de la machine lit la flotte avec l'identifiant de la machine) |
| A-4 | CA-3 (cloisonnement) : réécrire en « `fleet-<mac>` **ne peut pas publier** sous `iaka/<pc>/out/` » ; la lecture de `iaka/<pc>/out/#` par `fleet-<mac>` devient **permise** (A-3) |
| A-5 | Nouveau CA : **aucun** compte (console ou machine) ne peut publier sous `iaka/+/in/#` (`PUBACK 135`) |
| A-6 | CA-5 (testament) : topic `iaka/<node>/out/presence/online/current` |
| A-7 | Comptes, mots de passe, procédure de hachage local, piège d'inode, Vaultwarden : **inchangés** |

## 6. Périmètre

**Inclus** : import de `fleet/` avec historique ; contrat v2 et vecteurs partagés ; migration des topics
dans `fleet/`, le publieur et `sources/bus.rs` ; relais des fiches par le démon ; agents dans la CLI ;
source distante de l'app + règle d'état en Rust + groupement par machine ; documentation ; README de
renvoi dans iakaFleet ; recette réelle Mac + PC.

**Exclu** : tout ordre (`in/`), A1 compris ; toute écriture dans `naonedge-vps` ; l'installation des hooks
et du démon sur les machines (**gestes du décideur**, § 8) ; GUI sous Windows/Linux ; persistance,
historique ; événements `out/events/` ; convergence d'`iakatokencounter/` et d'`iakaframe/` vers `iaka/`
(listée au backlog, non faite) ; toute modification d'iakaTokenCounter, iakahub, iakaHub, iakaframe.

## 7. Étapes (Gimli)

0. Mesures `M1`–`M3`, `M5` (consignées au premier commit). `M4` après le Lot 0 amendé.
1. **Import** : `git subtree add --prefix=fleet <iakaFleet> feat/mvp-vue-flotte` dans IakaAgentsMonitor.
   `npm test` de `fleet/` vert **sans modification** (preuve que l'import n'a rien cassé).
2. **Vecteurs** `contract-vectors/*.json` (normalisation `<node>`, hachage `<sid>`, topics, états de
   machine) — **rouges** d'abord dans les trois pièces.
3. **`fleet/` en contrat v2** : `contract.js` (racine, `presence/`, `sessions/`), `IAKA_NODE`,
   `IAKA_MASK_NAMES` ; code `rc` retiré ou rectifié selon `M2`. Vecteurs verts côté Node.
4. **Relais des fiches** (`D3`) : `fleet/src/agent/cards.js` (lecture tolérante, hachage, `cwd`,
   masquage), publication différentielle, purge. Tests sur fixtures.
5. **CLI** : agents sous leur session (`D5`), `--json` inclus. **Livrable intermédiaire recettable.**
6. **Publieur + bus local en v2** : topic `iaka/<node>/out/agents/…` (normalisation copiée, vecteurs
   verts), filtre de `sources/bus.rs` `iaka/<node>/out/agents/+/+/+`, cohérence topic↔fiche dans
   `card.rs`. A1 (non codé) : son topic cible devient `iaka/<node>/in/agents/…` — noter dans
   `agnostique-mqtt.md` **par Gimli sur consigne de ce lot**.
7. **Source distante de l'app** (`D4`) : `sources/remote.rs`, `fleet_model.rs` (vecteurs verts côté Rust),
   lecture de `~/.iakafleet/.env`, exclusion du propre `<node>`, message « broker injoignable ».
8. **Rendu groupé par machine** (`src/render.ts`, `src/types.ts`), jetons V4.
9. **Documentation** : `README`, `CLAUDE.md` (projet, commandes `fleet/`, backlog, **section Forgejo
   corrigée** `F16`), `docs/installation.md` de `fleet/` mis à jour ; `README.md` de renvoi dans
   `~/work/iakaFleet` (seul fichier touché dans ce dépôt, hors `specs/`).
10. **Lot `L-INSTALL` — installation (décideur, Q3), APRÈS le Lot 0 amendé ET APRÈS `G-EXEC-2`
    (§ 13.1 : le démon ne s'installe en service qu'avec sa garde durcie et `node --permission`).** (a) **Mac d'abord** :
    démon `fleet/` en service (launchd) + **hooks du publieur** (copie dans `~/.claude/` et extrait
    `hooks` **ajouté** à `~/.claude/settings.json` — geste du décideur, guidé par la doc ; aucun agent
    n'écrit dans `~/.claude/`) ; vérifier `presence/cards_source_ok = true`. (b) **PC ensuite**, quand le
    lien sera rétabli : démon (tâche planifiée) + hooks. **Sans les hooks, la vue distante ne montre
    aucun agent** : c'est attendu, et l'app le dit (`R2`). Consigner machine par machine ce qui est fait.
11. **Recette** `CA-R*` (après `L-INSTALL` (a) ; volet PC après (b)).
12. `iakaframe snapshot --reason version --version v0.3.0 --note "fusion flotte + vue distante"`.

## 8. Ce qui appartient au décideur

Re-valider le Lot 0 amendé (§ 5) et autoriser son exécution ; **lot `L-INSTALL`** (étape 10) : démon
et **hooks du publieur** sur le Mac d'abord, le PC quand le lien sera rétabli (`F6` — sans les hooks,
la vue distante montre machines et sessions, pas d'agents) ; archiver `iakaFleet` sur Forgejo après
l'import ; tester (`CA-R6`).

## 9. Questions ouvertes

Aucune : les quatre questions du cadrage ont été tranchées le 2026-09-30 (cf. en-tête).

## 10. Risques

| # | Risque | Gravité | Mitigation |
|---|---|---|---|
| R1 | Trois implémentations d'une même règle (Node démon, Node publieur, Rust) **divergent** | haute | `contract-vectors/` exécutés par les trois (CA-2) |
| R2 | 🛑 **Vue distante vide faute de hooks** (`F6`) prise pour « aucun agent » | haute | La CLI et l'app affichent « fiches d'agents : aucune source sur cette machine » quand `~/.cache/iaka-agents` n'existe pas (code `presence/cards_source_ok`) |
| R3 | Rattachement fiche ↔ session faux après `/clear` (`sessionId` périmé dans le fichier de session) | moyenne | Agents « non rattachés » affichés, jamais perdus |
| R4 | Le Lot 0 validé sur `iakafleet/` est exécuté **tel quel** avant amendement | moyenne | § 5 ; CA-R1 exige l'ACL `iaka/` |
| R5 | Import subtree conflictuel ou `fleet/` cassé par l'import | faible | Étape 1 : tests de `fleet/` verts sans modification |
| R6 | Format interne des fichiers de session et de hooks (rappel MVP Fleet `R1`, Monitor `R10`) | moyenne | Parsing tolérant, états « illisible » explicites |

## 11. Critères d'acceptation

- [ ] **CA-1** *(reformulé le 2026-09-30, écart n° 2 du gate)* — `fleet/` importé **avec l'historique
      d'iakaFleet conservé dans le graphe** : le commit d'import a pour **second parent** la tête de
      `feat/mvp-vue-flotte` (`6ae1eca`), et ces commits se lisent **à leurs chemins d'origine** (sans
      préfixe `fleet/`) par `git log <commit d'import>^2 -- <chemin>` — **pas** par `git log -- fleet/`,
      qui ne montre que les commits postérieurs à l'import (propriété de `git subtree add` sans
      `--squash`). Vérif : `git rev-parse <commit d'import>^2` = `6ae1eca…` et
      `git rev-list --count <commit d'import>^2` ≥ nombre de commits d'iakaFleet. Tests de `fleet/` verts
      avant toute modification.
- [ ] **CA-2** *(précisé le 2026-09-30, écart n° 3)* — Chaque pièce exécute **les vecteurs qu'elle
      consomme selon la matrice de `specs/contrat-bus-iaka.md` § 6** (Node `node --test`, Rust
      `cargo test`), vus rouges avant ; les cases « sans objet » sont des **exclusions déclarées**. Vérif :
      pour chaque case « oui » de la matrice, un test nommé charge le fichier de vecteurs correspondant.
- [ ] **CA-3** *(réécrit le 2026-09-30, écart n° 1)* — Plus aucun **topic** v1 :
      `grep -rnE '(^|[^.])iakafleet/|iaka/agents/out' fleet/src src-tauri/src publishers` ⇒ **0**. Le motif
      exclut délibérément `~/.iakafleet/.env`, **nom de chemin conservé** (Q4, `D6`) qui n'est pas un topic ;
      l'ancien motif `iakafleet/` le comptait à tort (8 lignes, défaut de rédaction, pas du code).
- [ ] **CA-4** — `G-EXEC` (niveau 1, livré) : aucun `child_process` dans `fleet/src` ni `fleet/bin`.
      ⚠️ **Contournable** (chaîne concaténée, `process.binding`…) : le durcissement est le lot
      **`G-EXEC-2`** (§ 13), **exigé avant `L-INSTALL`**.
- [ ] **CA-5** — Relais : fixture de fiche ⇒ publiée sous `iaka/<node>/out/agents/<runner>/<sid12>/<agentId>`, `sessionId` = `<sid12>`, `cwd` relatif ; fiche supprimée ⇒ payload vide retenu ; **sans réglage ⇒ `task` = `null`** (masqué par
défaut, Q1) ; `IAKA_SHOW_TASKS=1` ⇒ `task` publié tel quel ; `IAKA_MASK_NAMES=1` ⇒ `name` des sessions à `null`.
- [ ] **CA-6** — Aucun `sessionId` brut, `bridgeSessionId`, `messagingSocketPath` ni chemin absolu dans un payload vers le central (test sur toutes les publications d'un scénario).
- [ ] **CA-7** — App **sans** central configuré : comportement identique à `a27a3c8` (tests existants verts, aucune connexion sortante hors `127.0.0.1:1883`).
- [ ] **CA-8** — App, central injoignable ⇒ « broker injoignable : <motif> », jamais une section vide ; son propre `<node>` n'apparaît pas en double.
- [ ] **CA-9** — `cargo test`, `clippy -D warnings`, `npm run typecheck`, `npm run build`, `npm run test:publisher`, `npm test` (fleet) verts ; l'app n'ouvre aucun port.
- [ ] **CA-R1** — Lot 0 **amendé** exécuté et coché (ACL `iaka/`, testament, refus sur `in/`).
- [ ] **CA-R1b** — **`L-INSTALL` (a) fait sur le Mac** : démon en service, hooks installés par le décideur, `presence/cards_source_ok = true` ; (b) PC consigné « fait » ou « en attente du lien ».
- [ ] **CA-R2** — CLI depuis le Mac **et** le PC : les deux machines, leurs sessions, et les agents là où les hooks sont installés (tâches affichées « masquée » sauf `IAKA_SHOW_TASKS=1`).
- [ ] **CA-R3** *(mode d'observation reformulé le 2026-09-30 ; « Ce poste » s'entend avec son titre et
      toutes ses sessions, `ce-poste-vue-locale-lisibilite.md` — à recetter **après** ce lot et
      `corr-hors-ligne-duree.md`)* — App sur le Mac : « Ce poste » + le PC dans
      « Autres machines » ; démon du PC arrêté ⇒ PC **hors ligne** en ≤ 60 s dans l'app. **Comment
      l'observer** : un gestionnaire de service relance le démon en ~1 s (launchd `KeepAlive` ; le
      `ThrottleInterval` de 10 s ne joue pas pour un service qui a vécu longtemps), donc après un `kill -9`
      l'état hors ligne n'est visible que ~1 s. On prouve donc **séparément** : (i) le **testament**, par un
      **abonné lancé avant** le `kill -9` (`mosquitto_sub … -t 'iaka/<node>/out/presence/#' -v`), qui doit
      recevoir `online=false` à la seconde du kill ; (ii) l'**état hors ligne durable**, par un **arrêt
      propre sans relance** (`launchctl bootout` sur le Mac, `Stop-ScheduledTask` puis désactivation sur le
      PC) — l'app doit afficher hors ligne en ≤ 60 s et **le rester** ; puis réinstaller le service.
- [ ] **CA-R4** — Couper le réseau du Mac : la vue « Ce poste » continue **avec toutes ses sessions et son
      titre** (source disque, `ce-poste-vue-locale-lisibilite.md` `D1`), le libellé du démon passe à l'état
      lu en dernier ; « Autres machines » affiche le motif. *(Précisé le 2026-09-30.)*
- [ ] **CA-R5** — Aucun secret dans le dépôt ; secrets dans Vaultwarden `work/iakaFleet`.
- [ ] **CA-R6** — **Testé par le décideur.**

### 11.1 État de la recette au 2026-09-30 (soir) — consigné, **rien n'est coché sans le décideur**

| Critère | État | Preuve / reste |
|---|---|---|
| **CA-R1** | **prouvé côté Mac** | Lot 0 amendé rejoué avec les vrais secrets : **0 FAIL**. Testament prouvé **en réel** : abonné console présent pendant `launchctl kill SIGKILL`, `{"v":false,…}` reçu à la seconde du kill (l'ACL le couvre, #3552 écarté). **Reste** : volet PC de CA-5 du Lot 0 (décideur, quand le lien sera rétabli) |
| **CA-R1b** | **(a) fait sur le Mac** · **(b) PC en attente du lien** | Démon sous launchd, lancé sous **`node --permission`**, **TLS réel** vers `mqtt.naonedge.com:8883` ; hooks du publieur installés **par le décideur** ; fiches d'agents visibles sous leurs sessions dans `iakafleet ls` |
| **CA-R2** | **Mac fait** · **PC non fait** | `iakafleet ls` sur le Mac : 4 sessions, agents rattachés sous `azime-be` et `work-2a`, tâches affichées « masquée » (défaut Q1) |
| **CA-R3** | **non fait** | L'app Tauri n'a **jamais été lancée à l'écran** ; PC non joint. Mode d'observation reformulé ci-dessus |
| **CA-R4** | **non fait** | App non lancée au moment du relevé. *Mise à jour (2026-09-30, plus tard)* : app lancée en `tauri dev` par le décideur ; écarts constatés (pas de titre « Ce poste », vue locale incomplète, lisibilité, CPU) instruits dans `ce-poste-vue-locale-lisibilite.md` ; CA-R3/CA-R4 à recetter après ce lot |
| **CA-R5** | **fait** | Legolas : `git grep` ⇒ 0 secret ; 3 items Vaultwarden `work/iakaFleet` conformes |
| **CA-R6** | **partiel** | Le décideur a lancé `iakafleet ls` ; ni l'app ni le PC |

**Écart instruit pendant cette recette** : la durée « hors ligne depuis » est fausse après un testament
(le `t` du testament est l'instant de **connexion** du démon — mesure : testament `t=1790797786`
antérieur au `true` du même démon `t=1790797788`). Correctif cadré :
**`specs/instructions/corr-hors-ligne-duree.md`** (≈ 0,3 j-h), à livrer **avant** la recette de CA-R3.

## 12. Estimation (jalon P1→P2)

| Poste | j-h |
|---|---:|
| Mesures + import subtree | 0,3 |
| Contrat v2 + vecteurs (3 pièces) | 0,5 |
| `fleet/` en v2 + relais des fiches | 0,9 |
| CLI : agents | 0,3 |
| Publieur + bus local en v2 | 0,4 |
| Source distante Rust + règle d'état | 0,8 |
| Rendu groupé | 0,5 |
| Documentation | 0,3 |
| Recette réelle | 0,5 |
| **Total** | **≈ 4,5** (fourchette **3,5 – 6**) — hors Lot 0 amendé (≈ 0,3, instruction `naonedge-vps`) |

**Complexité : moyenne à élevée** (trois langages/lieux pour une convention). **Risque : moyen.**
**Inconnues** : contenu réel de la branche Fleet (`M1`) ; TLS `rumqttc` contre le VPS (`M4`) ; hooks jamais
installés (`F6`) ; accès au PC Windows (volet (b) de `L-INSTALL`). Maquette écartée (Q2). *Ordre de grandeur
révisable, pas un engagement ferme.*

## 13. Suites du gate Legolas (PASS, `feat/fusion-flotte` @ `a5f4b2c`) — ajouté le 2026-09-30

> Quatre écarts **non bloquants** entre l'instruction et le code/la réalité, instruits ici ; plus deux
> décisions laissées au décideur par Gimli. Les écarts 1 à 3 étaient des **défauts de rédaction** :
> corrigés dans CA-1, CA-2, CA-3 et dans la matrice du contrat § 6, **sans changement de code exigé**.

### 13.1 `G-EXEC-2` — durcissement de la garde d'exécution (écart n° 4) — **lot suivant, AVANT `L-INSTALL`**

**Constat** : `fleet/test/guard-exec.test.js:18-21` cherche la seule chaîne `child_process` ; un
`import('node:child_pro' + 'cess')`, `process.binding('spawn_sync')` ou un `Worker` passeraient sans rougir.
Une liste d'interdits s'allonge sans jamais être complète : **on retourne la garde**. Deux couches :

1. **Statique — liste blanche des modules**, pas liste noire. Dans `fleet/src` et `fleet/bin`, tout
   `import` statique doit viser un spécifieur de la liste blanche **versionnée dans le test** (modules
   `node:` réellement utilisés — `fs`, `path`, `os`, `crypto`, `url`… — plus `mqtt` et les chemins
   relatifs). Sont **rouges** en plus, quelle que soit leur forme : `import(` **dynamique**, `require(`,
   `eval(`, `new Function`, `Function(`, `process.binding`, `process._linkedBinding`, `process.dlopen`,
   `worker_threads`, `node:cluster`, `node:vm`, `child_process` (y compris concaténé : la garde lit aussi
   les **littéraux de chaîne adjacents**). Un module ajouté à la liste blanche exige une ligne de
   motif dans le test (registre motivé, pas une liste muette). Analyse par expressions régulières sur le
   texte source, **sans dépendance d'analyse AST** (le dépôt n'en a pas ; une dépendance de plus pour une
   garde est un mauvais échange).
2. **Exécution — modèle de permissions de Node** : les gabarits de service (`fleet/deploy/*`) lancent le
   démon avec **`node --permission`**, sans `--allow-child-process` ni `--allow-worker`, et avec
   `--allow-fs-read` borné au home de l'utilisateur. Toute tentative de lancer un processus échoue alors
   **au runtime** (`ERR_ACCESS_DENIED`), quelle que soit la ruse du code. Le modèle est **stable depuis
   Node 22.14 / 23.5** ([Node.js — Permissions](https://nodejs.org/api/permissions.html)) ⇒ `engines.node`
   de `fleet/` passe à **`>=22.14`**. **Mesure préalable** : version de Node sur le PC Windows (sur le Mac
   et le VPS : v24 connue).

**Hors couverture, déclaré** : le code **des dépendances** (`mqtt` et son arbre) n'est pas lu par la
garde statique ; il est couvert par la couche 2 (le permis vaut pour tout le processus) et par
l'épinglage + lockfile.

**Critères** : `CA-G1` — chaque forme interdite ci-dessus, déposée dans un fichier témoin sous
`fleet/src`, rend la garde **rouge** (test de contrefactuel, un cas par forme) ; `CA-G2` — un import
hors liste blanche rend la garde rouge ; `CA-G3` — démon lancé par le gabarit de service : un
`child_process.spawn` injecté en test d'intégration échoue en `ERR_ACCESS_DENIED` ; le démon publie
normalement (lecture de `~/.claude/sessions`, `~/.cache/iaka-agents`, `.git/HEAD` permise).
**Estimation : ≈ 0,4 j-h.**

### 13.2 Squelette iakaframe importé sous `fleet/` — **à trancher par le décideur** (décision a)

**Constat** : l'import a amené le squelette de l'ex-dépôt : `fleet/.claude/commands/` (35 commandes),
`fleet/global/` (copies des gardes par hooks), `fleet/iakaframe.json`, `fleet/.iakaframe`, `fleet/specs/`
(PROJET, état des lieux, instruction MVP) et **`fleet/CLAUDE.md`**, qui porte encore un remote
`iakaFleet.git`, un backlog « Lot 0 `iakafleet/` » et des consignes d'ex-projet. Claude Code charge les
`CLAUDE.md` des sous-dossiers qu'il lit : **un agent qui travaille dans `fleet/` recevrait ces consignes
périmées comme instructions**.

**Recommandation : NETTOYER**, dans un commit dédié `chore(fleet): retirer le squelette de l'ex-depot` :
supprimer `fleet/.claude/`, `fleet/global/`, `fleet/iakaframe.json`, `fleet/.iakaframe`, `fleet/specs/` et
`fleet/CLAUDE.md` ; reporter dans **`fleet/README.md`** ce qui reste utile (commandes, arborescence, règles
dures, et la commande de lecture de l'historique d'import de CA-1). Conserver `fleet/.env.example`,
`fleet/.gitignore`, `fleet/docs/`, `fleet/deploy/`. **Rien ne se perd** : ces fichiers restent dans
l'historique (second parent de l'import) et dans le dépôt iakaFleet archivé ; l'instruction MVP y reste
la référence des parties réutilisées. **Coût ≈ 0,15 j-h.** Alternative — les laisser — : zéro coût, mais
deux jeux d'instructions concurrents dans le même dépôt et des copies de gardes qui divergeront.

### 13.3 Historique de l'import — **à trancher par le décideur** (décision b)

**Recommandation : ACCEPTER TEL QUEL.** L'historique d'iakaFleet **est** conservé (second parent
`6ae1eca`), seulement **à ses chemins d'origine** ; CA-1 est reformulé en conséquence et la commande de
lecture est documentée (`fleet/README.md`, § 13.2).
Ré-importer « chemins réécrits sous `fleet/` » (`git filter-repo --to-subdirectory-filter fleet` sur un
clone, puis fusion) rendrait `git log -- fleet/` continu, mais : il **réécrit** les 15 commits (nouvelles
empreintes, deux historiques d'iakaFleet coexistant), il oblige à **reconstruire une branche déjà
poussée et gatée** — sans `push --force`, donc sur une branche neuve —, il demande un outil absent du
dépôt, et il **invalide le gate PASS** (re-gate complet). **Coût ≈ 0,5 j-h + re-gate**, pour un gain de
confort de lecture seulement.
