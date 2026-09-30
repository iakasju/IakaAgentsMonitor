# Instruction : MVP — vue de la flotte en lecture seule

> Cadrée par **Gandalf** (P1 — cadrage) le **2026-09-30**, sur ordre de mission d'Odin portant les
> arbitrages du décideur du même jour. Exécution : **Gimli** (P2), **après validation du jalon
> P1→P2 par le décideur**. Dépôt cible : `~/work/iakaFleet/` (v0.1.0, onboardé iakaframe 0.41.0).
>
> **Cadrage en lecture seule.** Chaque `F*` a été **lu sur le disque** ou **vérifié sur le web**
> pendant cette session. Ce qui n'a pas pu l'être (pas de shell au cadrage) est marqué
> **NON MESURÉ** et confié à l'**étape 0** de Gimli (`M*`). Aucun fait n'est repris sur parole.
>
> Conventions : `F*` = fait vérifié · `M*` = mesure à faire par Gimli · `D*` = décision ·
> `R*` = risque · `CA-*` = critère d'acceptation.

---

## 1. Problème

Le Remote Control (`/rc`) ne dit **ni qui tourne où, ni dans quel état** : pas d'accusé de
réception, une session d'un autre poste n'est visible que si `/rc` est actif des deux côtés, la
machine et l'état réel restent inconnus. Avant de pouvoir **ordonner** quoi que ce soit à une
session distante, il faut pouvoir la **voir** de façon fiable.

Le MVP livre donc la **vue en lecture seule** : depuis n'importe quelle machine, lister les
machines de la flotte (en ligne / hors ligne / muettes, OS, versions) et leurs sessions Claude Code
(état, dépôt, branche, joignable par `/rc` ou non) — **y compris la mort d'une machine, que
personne n'annonce** (testament MQTT).

---

## 2. Faits

### 2.1 L'existant iakaframe — mesuré sur le disque

| Id | Fait | Preuve |
|---|---|---|
| **F1** | Le broker du VPS est en service : Mosquitto 2.1.2, `mqtts://mqtt.naonedge.com:8883` (TLS terminé par Mosquitto, certificat Let's Encrypt recopié depuis Caddy) + WSS 443 ; CA-1 à CA-11 cochés le 2026-09-21, **CA-12 « testé par le décideur » encore ouvert** | `~/work/naonedge-vps/specs/instructions/mqtt-broker-vps.md:470-509` · `~/work/naonedge-vps/CLAUDE.md` § Backlog |
| **F2** | 🛑 **L'ACL du broker ne connaît AUCUN droit sur `iakafleet/`** : trois comptes seulement (`iakaframe-publisher` write `iakaframe/#`, `bridge` et `dashboard` read `iakaframe/#`), **aucune règle globale** — par défaut un compte n'a accès à rien. **Rien de ce lot ne peut publier sans un geste sur le broker** (→ Lot 0) | `~/work/naonedge-vps/stack/mosquitto/acl:1-10` |
| **F3** | Le client `iakalog.mjs` ne parle que `mqtt://` (regex ligne 52), ouvre une socket `net.connect` (ligne 75), publie en **QoS 0** sans `retain` ni testament, et **se déconnecte après un message**. Il est fait pour un tir unique, pas pour une présence | `~/work/iakaframe/library/skills/iakaframe-log-conversation/iakalog.mjs:52,64-75` |
| **F4** | Le contrat code/valeur d'iakaTokenCounter : **le topic est le code**, le payload est `{"v":<scalaire>,"t":<epoch s>}` (jamais un objet ni un tableau), `retained`, **QoS 1** ; valeur `null` = « inconnu daté » ; **effacer un code = payload vide 0 octet en retained** ; un segment `<host>` est prévu | `~/work/iakaTokenCounter/specs/contrat-mqtt-conso.md:136-155,227-234,43-45` |
| **F5** | **Claude Code écrit un fichier par session vivante** : `~/.claude/sessions/<pid>.json`, avec `pid`, `sessionId`, `cwd`, `startedAt` (ms), `version`, `kind`, `entrypoint`, `name`, **`status`**, `statusUpdatedAt`, `updatedAt`, `pidDomain`, `messagingSocketPath`. Trois fichiers lus sur le Mac le 2026-09-30 : `status` vaut `"idle"` (1) et `"busy"` (2) ; `entrypoint` vaut `"cli"` (2) et `"claude-desktop"` (1) ; `version` `2.1.284`/`2.1.285` | `~/.claude/sessions/80213.json`, `75634.json`, `83541.json` |
| **F6** | **Un champ `bridgeSessionId` n'apparaît que sur la session pilotée par Remote Control** — celle de ce cadrage (`83541`, valeur `session_01K8Un…`, identique à l'URL de session claude.ai). Les deux autres ne l'ont pas. ⚠️ **Inférence tirée d'UN échantillon** : à confirmer (`M5`) | `~/.claude/sessions/83541.json` |
| **F7** | iakaHub (candidat « démon par machine ») : **sur `main`, le démon quitte en erreur si `DISCORD_BOT_TOKEN` est absent** — une machine sans provisionnement Discord ne peut pas le lancer | `~/work/iakaHub/src/index.js:68-72` |
| **F8** | iakaHub : le dépôt est **sur `main`** (`.git/HEAD`) et `src/core/scheduler.js` **n'y existe pas** — le lot `HUB-VEILLE` (branche `feat/hub-veille`) **n'est pas fusionné** ; l'instruction du lot le dit validée au gate P1→P2 le 2026-08-10. L'état réel de la branche est **NON MESURÉ** ici (pas de shell) → `M6` | `~/work/iakaHub/.git/HEAD:1` · recherche de fichiers `src/**` · `hub-veille-horloge-canal-sortant-et-sondes.md:671-687` |
| **F9** | iakaHub : service macOS **en production** (launchd) ; service Windows (tâche planifiée) **jamais recetté** (« revue statique, aucun `pwsh` sur la machine de build ») | `~/work/iakaHub/deploy/README.md:46,186-198` |
| **F10** | iakaHub n'a **aucune dépendance MQTT** (`discord.js`, `yaml` seulement) | `~/work/iakaHub/package.json:19-22` |
| **F11** | Le CLI iakaframe est le paquet npm **`@naonedge/iakaframe`**, version courante **0.41.0**, bin `iakaframe` → `src/index.js` ; `iakaframe --version` existe | `~/work/iakaframe/cli/package.json:2-8` · `~/work/iakaframe/cli/README.md:22-25` |
| **F12** | Le dépôt iakaFleet est **vierge de code** (structure iakaframe seule) ; `.gitignore` couvre déjà `.env` et `node_modules/` | `~/work/iakaFleet/.gitignore:1-2` · `specs/etat-des-lieux.md:10-14` |
| **F13** | ⚠️ Le `CLAUDE.md` du dépôt porte encore le gabarit « Forgejo iakabox `192.168.2.11` » alors que `origin` = `git.naonedge.com` : **section périmée** à corriger par Gimli (étape 9) | `~/work/iakaFleet/CLAUDE.md:58-63` |

### 2.2 Faits externes — vérifiés sur le web le 2026-09-30

| Id | Fait | Source |
|---|---|---|
| **F14** | `status` du fichier de session a **quatre valeurs** connues : `busy`, `shell`, `idle`, `waiting`. `statusUpdatedAt` date la **dernière transition**, ce n'est **pas un battement** : une session longuement `busy` garde un horodatage ancien, c'est normal. La vivacité se juge sur le **pid**, pas sur l'horodatage | [cc-hands PR #14](https://github.com/brandon-fryslie/cc-hands/pull/14) · [claude-monitor PR #64](https://github.com/maxtomachine/claude-monitor/pull/64) · [c9watch PR #132](https://github.com/minchenlee/c9watch/pull/132) (pids morts à écarter) |
| **F15** | 🛑 **Bug connu, fermé « not planned »** : une session « parquée » en plein tour garde `status: "busy"` **pour toujours** (v2.1.220). Signature : `parkedJobId` présent et `updatedAt > statusUpdatedAt` | [anthropics/claude-code#87131](https://github.com/anthropics/claude-code/issues/87131) |
| **F16** | Autre défaut connu : après `/clear`, le `sessionId` du fichier **n'est pas mis à jour**. ⇒ le **pid** est la seule clé stable d'une session vivante | [anthropics/claude-code#36213](https://github.com/anthropics/claude-code/issues/36213) · [#95439](https://github.com/anthropics/claude-code/issues/95439) |
| **F17** | 🛑 **Ce fichier n'a pas de page de documentation officielle trouvée** sur `code.claude.com` : c'est un **format interne**, lu par de nombreux outils tiers, susceptible de changer sans préavis. Présence sous **Windows NON MESURÉE** (`M1`) | recherche web du 2026-09-30 (aucun résultat `code.claude.com`) |
| **F18** | **MQTT.js** (`npm i mqtt`) : dernière version **5.16.0** (publiée mi-septembre 2026), Node ≥ 18, réécrite en TypeScript ; gère TLS (`mqtts://`), testament, QoS 1, keepalive, reconnexion | [npmjs.com/package/mqtt](https://www.npmjs.com/package/mqtt) |
| **F19** | Mosquitto : ACL par `user` / `topic read|write|readwrite` ; substitution `%u`/`%c` possible via `pattern` | [mosquitto.conf(5)](https://mosquitto.org/man/mosquitto-conf-5.html) |
| **F20** | 🛑 **Mosquitto accepte au CONNECT un testament sur un topic que l'ACL interdit, puis le jette en silence à la livraison** (aucun refus journalisé). ⇒ un testament mal couvert par l'ACL **ne dira jamais « hors ligne »**, et rien ne le signalera | [eclipse-mosquitto#3552](https://github.com/eclipse-mosquitto/mosquitto/issues/3552) |
| **F21** | Depuis Node 18.20.2 / 20.12.2 (CVE-2024-27980), lancer un `.cmd`/`.bat` sans `shell: true` échoue en `EINVAL` sous Windows — or `iakaframe` y est un shim `.cmd`. Obtenir sa version **en l'exécutant** imposerait un shell | [Node.js, avril 2024](https://nodejs.org/en/blog/vulnerability/april-2024-security-releases-2) |
| **F22** | Pour la phase « ordres » (hors MVP, noté pour mémoire) : Claude Code offre les **channels** — un serveur MCP pousse un événement **dans une session en cours**, qui y réagit sous ses propres permissions (research preview) | [code.claude.com/docs/en/channels](https://code.claude.com/docs/en/channels) |

### 2.3 Mesures confiées à Gimli — étape 0, AVANT tout code

| Id | À mesurer | Pourquoi |
|---|---|---|
| **M1** | Sur le **PC Windows** : `~/.claude/sessions/` existe-t-il (`%USERPROFILE%\.claude\sessions`) ? mêmes champs ? valeur de `pidDomain` ? | `F17` : tout le MVP côté Windows en dépend. **S'il n'existe pas : s'arrêter et remonter** (cf. `R1`) |
| **M2** | `os.hostname()` sur les deux machines (Node, pas le shell) | fixe les identifiants `<host>` et les comptes MQTT (`D5`) |
| **M3** | Emplacement réel de `@naonedge/iakaframe/package.json` sur les deux machines (`npm root -g`, lien du bin) | `D8` : lecture de version sans exécution |
| **M4** | `npm view mqtt version` + arbre de dépendances de `mqtt@<version>` (`npm ls --all` après installation) | épinglage exact (`D3`) |
| **M5** | Ouvrir une session avec `/rc`, une sans : `bridgeSessionId` présent/absent ? | confirme `F6` avant d'en tirer le champ `rc` |
| **M6** | `git -C ~/work/iakaHub log --oneline main..feat/hub-veille` | consigner l'état réel de `HUB-VEILLE` (argument de `D1`) — **mesure, pas modification** |

> Chaque mesure est **consignée dans le message de commit de l'étape 1** (valeurs brutes).

---

## 3. Décision retenue

> **Un agent par machine lit, en lecture seule, les fichiers de session de Claude Code et publie
> leur état sur MQTT selon le contrat code/valeur ; une console CLI lit le broker et rend la flotte.
> Aucun des deux n'exécute rien, ne commande rien, et ne dépend d'aucun centre.**

### `D1` — Où vit l'agent : **paquet autonome dans iakaFleet**, pas une brique d'iakaHub au MVP — *soumis au gate*

L'ordre de mission nomme iakaHub comme **candidat**, à valider. Le cadrage **recommande de ne pas
l'y loger au MVP**, pour trois motifs mesurés :

1. **`F7`** — iakaHub **refuse de démarrer sans Discord**. L'y loger oblige soit à provisionner
   Discord sur chaque machine de la flotte, soit à modifier son démarrage — sur un dépôt dont le
   service Windows n'a jamais été recetté (`F9`).
2. **`F8`** — iakaHub a un lot validé (`HUB-VEILLE`) **non fusionné**. Greffer une brique MQTT sur
   `main` crée un conflit programmé ; attendre la fusion bloque ce lot sur un autre.
3. **Pas de centre imposé** — iakaHub est un **hub de passage Discord** ; lui confier la présence
   de flotte en ferait le composant obligatoire de toute machine.

**Ce que ça préserve** : le cœur de l'agent (`src/agent/*`) est écrit en **modules purs à
dépendances injectées** ; iakaHub pourra l'**importer** plus tard (instruction `liaison-iakahub`)
sans réécriture. **Si le décideur retient iakaHub comme hôte**, seuls changent l'étape 8
(service) et le lieu du code — le contrat (`D4`), la console et les critères restent valables.

### `D2` — L'agent **ne lance aucun processus** : zéro `child_process`

Règle permanente du décideur (un ordre n'est jamais exécuté comme un shell), appliquée **dès le
MVP** où il n'y a pas d'ordre : c'est le moment le moins cher pour la graver. L'agent lit des
fichiers et parle MQTT, rien d'autre. Conséquences : la branche git se lit dans `.git/HEAD`
(`D7`), la version d'iakaframe dans `package.json` (`D8`, et `F21` : l'exécuter exigerait un shell
sous Windows). **Vérifié par un test** (`CA-3`).

### `D3` — Client MQTT : **MQTT.js**, une dépendance épinglée — pas de client fait main

`iakalog.mjs` (`F3`) est un tir unique QoS 0. Une **présence** exige une session **persistante** :
keepalive, reconnexion, testament, QoS 1 avec acquittements, abonnement. C'est exactement ce que
« ne rien réimplémenter » interdit de réécrire. MQTT.js (`F18`) est la référence Node ; version
**épinglée exactement** (`M4`), `package-lock.json` commité. Le patch TLS d'`iakalog.mjs` **n'est
pas nécessaire au MVP** et reste au backlog iakaframe (il servira à la trace des ordres).

### `D4` — Contrat sur le fil **v1** : racine `iakafleet/<host>/`, modèle code/valeur d'iakaTokenCounter

Tout message : payload `{"v":<scalaire>,"t":<epoch s>}` (`F4`), **`retain: true`, QoS 1**.
Topic = `iakafleet/<host>/<code>/current`. Effacement d'un code = **payload vide 0 octet, retained**.

**Codes machine** — `iakafleet/<host>/meta/<code>/current` :

| Code | Type | Sens |
|---|---|---|
| `online` | boolean | `true` à la connexion ; **`false` porté par le testament** (`D6`) et par l'arrêt propre |
| `os` | string | `process.platform` (`darwin`/`win32`/`linux`) |
| `os_release` | string | `os.release()` |
| `hostname` | string | `os.hostname()` brut (le `<host>` du topic est sa forme normalisée) |
| `agent_version` | string | version du paquet iakaFleet |
| `node_version` | string | `process.version` |
| `iakaframe_version` | string\|null | `D8` ; `null` = introuvable (inconnu daté, jamais inventé) |
| `sessions_dir_ok` | boolean | le répertoire de sessions est-il lisible ? **`false` écrit, jamais sous-entendu** |
| `sessions_count` | number | sessions vivantes publiées |
| `heartbeat_s` | number | période du battement |
| `last_tick_at` | number | epoch s du dernier battement (**republié à chaque battement**, seul code non différentiel) |

**Codes session** — `iakafleet/<host>/sessions/<pid>/<code>/current` (clé = **pid**, `F16`) :

| Code | Type | Source (`F5`) |
|---|---|---|
| `status` | string | `status` **tel quel** (`idle`/`busy`/`waiting`/`shell`/autre) |
| `status_since` | number | `statusUpdatedAt` / 1000 |
| `stuck_suspect` | boolean | `true` si `parkedJobId` présent **et** `updatedAt > statusUpdatedAt` (`F15`) |
| `name` | string | `name` |
| `kind` · `entrypoint` | string | idem |
| `claude_version` | string | `version` |
| `cwd` | string | `cwd` **relatif au home** (`~/work/azime`) — le chemin absolu ne quitte pas la machine |
| `repo` | string\|null | nom du dossier racine git contenant `cwd` (`D7`) |
| `branch` | string\|null | branche, ou `detached:<sha7>` (`D7`) |
| `rc` | boolean | présence de `bridgeSessionId` (`F6`, sous réserve de `M5`) |
| `started_at` | number | `startedAt` / 1000 |

**Non publiés, délibérément** : `sessionId`, `bridgeSessionId` (valeur), `messagingSocketPath`,
tout contenu de conversation. On publie **l'état**, jamais un identifiant exploitable.

**Racine à part** : `iakafleet/#` n'est capté ni par le pont iakaboxlogs (`iakaboxlogs/#`) ni par le
futur pont du VPS (`iakaframe/#`). Le contrat est porté par **un seul module** (`src/shared/contract.js`)
importé par l'agent **et** la console.

### `D5` — Identité : **un `<host>` et un compte MQTT par machine**, ACL explicite

`<host>` = `IAKAFLEET_HOST` si défini, sinon `os.hostname()` en minuscules, suffixe `.local` retiré,
tout caractère hors `[a-z0-9-]` remplacé par `-`. **Refus au démarrage** si le résultat est vide ou
contient `+`, `#`, `/`.

Comptes et ACL (à poser au **Lot 0** dans `~/work/naonedge-vps/stack/mosquitto/acl`) — lignes
**explicites**, pas de `pattern %u` : il s'appliquerait aussi aux trois comptes existants (`F19`).

```
user fleet-console
topic read iakafleet/#

user fleet-<host-mac>
topic readwrite iakafleet/<host-mac>/#

user fleet-<host-pc>
topic readwrite iakafleet/<host-pc>/#
```

`readwrite` sur **son propre sous-arbre seulement** : l'agent doit **lire** ses anciens retained
pour les purger (`D6`) et **écrire** son testament (`F20` : un testament hors ACL est jeté en
silence). Un agent **ne peut ni lire ni écrire** le sous-arbre d'une autre machine. La console
**ne peut rien publier**. Mots de passe : `openssl rand -base64 24`, saisie interactive de
`mosquitto_passwd` (jamais `-b`), dépôt **Vaultwarden** `work/iakaFleet`, items
`iakaFleet/mqtt-fleet-<host>` et `iakaFleet/mqtt-fleet-console`.

### `D6` — Vivacité : **testament retained + battement**, et la purge des sessions fantômes

- **Testament** : à la connexion, will = `iakafleet/<host>/meta/online/current`, payload
  `{"v":false,"t":<instant de connexion>}`, **retain, QoS 1**. Keepalive **30 s** ⇒ une machine
  morte, endormie ou débranchée passe **hors ligne en ≤ ~45 s** sans que personne ne l'annonce.
- **Arrêt propre** (SIGINT/SIGTERM) : publier `online=false` **puis** se déconnecter (une
  déconnexion propre **n'émet pas** le testament).
- **Balayage** des fichiers de session toutes les **5 s** (`IAKAFLEET_SCAN_S`) ; publication
  **différentielle** (un code n'est republié que si sa valeur change) ; `last_tick_at` republié
  toutes les **60 s** (`IAKAFLEET_HEARTBEAT_S`).
- **Session vivante** = fichier présent **et** pid vivant (`process.kill(pid, 0)`, qui n'envoie
  aucun signal ; `EPERM` = vivant). Fichier présent + pid mort = **fin de session** (`F14`).
- **Fin de session** : effacer (payload vide retained) **chacun** de ses codes.
- **Purge au démarrage** : l'agent s'abonne à `iakafleet/<host>/sessions/#`, recueille les
  retained d'une exécution précédente, puis efface ceux dont le pid n'est pas vivant.
- **Reconnexion** : republication **intégrale** de l'état (la mémoire différentielle est remise
  à zéro) — même règle qu'iakaTokenCounter.

### `D7` — Dépôt et branche : **lecture de `.git/HEAD`**, pas de `git`

Remonter depuis `cwd` jusqu'au premier dossier contenant `.git`. Si `.git` est un **dossier** :
lire `.git/HEAD`. Si c'est un **fichier** (worktree) : suivre `gitdir: <chemin>` et lire son `HEAD`.
`ref: refs/heads/<b>` ⇒ `<b>` ; un sha ⇒ `detached:<sha7>` ; rien trouvé ⇒ `repo`/`branch` =
`null`. **Aucun** appel à `git` (`D2`). Portée MVP : **les dépôts où tourne une session**, pas un
inventaire de `~/work`.

### `D8` — Version d'iakaframe : **lecture de `package.json`**, jamais d'exécution

Chercher `@naonedge/iakaframe/package.json` : (1) `IAKAFLEET_IAKAFRAME_PKG` si défini ;
(2) parcourir `PATH` : sous POSIX, `realpath` du fichier `iakaframe` puis remonter au
`package.json` dont `name === "@naonedge/iakaframe"` ; sous Windows, à côté de `iakaframe.cmd`,
`node_modules/@naonedge/iakaframe/package.json`. Introuvable ⇒ `null`. Relu à chaque battement
(une mise à jour du CLI se voit sans redémarrer l'agent). Chemins réels : `M3`.

### `D9` — Console : **CLI**, trois gestes, et **l'absence ne se déguise jamais en vide**

`iakafleet ls` (instantané puis sortie) · `iakafleet watch` (rafraîchi) · `--json` (sortie
machine, pour les tests et l'outillage). Connexion en `fleet-console`, abonnement `iakafleet/#`,
instantané rendu quand plus aucun retained n'arrive depuis **1,5 s** (plafond **5 s**).

État d'une machine, **règle unique** (module pur `fleetModel.js`) :

| Condition | Affiché |
|---|---|
| `online = true` et `now − last_tick_at ≤ 3 × heartbeat_s` | **en ligne** |
| `online = true` mais battement périmé | **muette** (agent connecté, horloge arrêtée) |
| `online = false` | **hors ligne** depuis `t` |
| pas de code `online` | **inconnue** |
| `sessions_dir_ok = false` | alerte **« sessions illisibles »** sur la machine |

Les sessions d'une machine qui n'est pas **en ligne** sont affichées **« ? (dernier connu :
<status>, il y a <durée>) »** — jamais comme si elles tournaient. `stuck_suspect = true` s'affiche
**« busy ? »**.

🛑 **Broker injoignable, TLS refusé, authentification refusée ⇒ message explicite sur stderr et
code de sortie 2.** Jamais une table vide qui se lirait « aucune machine ». Flotte réellement vide ⇒
message « aucune machine n'a publié » et code 0.

### `D10` — Configuration et secrets

Fichier `~/.iakafleet/.env` (même chemin sous les trois OS, via `os.homedir()`), les variables
d'environnement priment. Variables : `IAKAFLEET_MQTT_URL` (défaut
`mqtts://mqtt.naonedge.com:8883`), `IAKAFLEET_HOST`, `IAKAFLEET_AGENT_USER`/`_PASS`,
`IAKAFLEET_CONSOLE_USER`/`_PASS`, `IAKAFLEET_SESSIONS_DIR` (défaut `~/.claude/sessions`),
`IAKAFLEET_SCAN_S`, `IAKAFLEET_HEARTBEAT_S`, `IAKAFLEET_IAKAFRAME_PKG`.
🛑 **`mqtt://` en clair n'est accepté que vers `127.0.0.1`/`localhost`** (tests) ; toute autre
adresse exige `mqtts://`, et **la vérification du certificat n'est pas débrayable** au MVP.
Aucun secret dans le dépôt ; `.env.example` sans valeurs.

---

## 4. Périmètre

**Inclus**
- Lot 0 (hors dépôt, sur autorisation) : ACL + comptes MQTT sur le broker du VPS.
- Le paquet Node `iakafleet` : agent (`iakafleet-agent`) + console (`iakafleet`), contrat v1.
- Tests `node --test` (zéro dépendance de test), fixtures de fichiers de session.
- Gabarits de service : launchd (macOS), tâche planifiée au logon (Windows), systemd user (Linux).
- Documentation d'installation ; mise à jour de `CLAUDE.md` du dépôt.
- Recette réelle sur **le Mac et le PC Windows**.

**Exclu** — explicitement, et sans « tant qu'on y est »
- **Tout envoi d'ordre**, tout topic d'ordre, tout abonnement de l'agent hors de son propre
  sous-arbre de sessions (instruction `ordres-accuse-reception`).
- **Toute exécution** (`child_process`, shell, `claude -p`) — `D2`.
- Toute modification d'**iakaHub**, d'**iakalog.mjs**, d'**iakaTokenCounter**, d'**iakaboxlogs**.
- Persistance, historique, base, trace dans iakaboxlogs.
- Interface graphique (widget, page WSS) ; compte `dashboard` du broker non touché.
- Inventaire des dépôts hors sessions ; supervision CPU/RAM/disque.
- Le VPS (`iakagent`) comme membre de la flotte : le gabarit systemd est livré, **non recetté**.
- Purge d'une machine retirée de la flotte (geste destructif → instruction `autorisations`).

---

## 5. Étapes d'implémentation (Gimli)

> Commits **atomiques**, conventional commits, un par étape. Tests **avant** le code pour les
> quatre gardes `G-*`, **vus rouges**, trace de l'échec dans le message de commit.

0. **Mesures `M1`–`M6`** (§ 2.3). Si `M1` montre que Windows n'a pas de `~/.claude/sessions` :
   **s'arrêter et remonter à Aragorn** — le MVP Windows est alors à re-cadrer.
1. **Squelette** : `package.json` (`"type":"module"`, `engines.node ">=20"`, bins `iakafleet` et
   `iakafleet-agent`, scripts `test`/`lint`), `mqtt` épinglé exactement (`M4`), `.env.example`,
   `README.md` minimal. Mesures consignées dans le message de commit.
2. **Gardes rouges** : `G-EXEC` (aucun `child_process` dans `src/`), `G-VIDE` (broker injoignable
   ⇒ code 2, pas une table vide), `G-TESTAMENT` (options de connexion : will sur
   `meta/online/current`, `{"v":false}`, retain, QoS 1), `G-PURGE` (session disparue ⇒ payload vide
   retained sur **chacun** de ses codes).
3. **Contrat** `src/shared/contract.js` : construction des topics, normalisation de `<host>` (`D5`),
   encodage/décodage `{v,t}`, refus d'un `v` objet/tableau.
4. **Config** `src/shared/config.js` (`D10`) : lecture `~/.iakafleet/.env`, priorité env, refus de
   `mqtt://` distant.
5. **Lecture de l'hôte** : `src/agent/sessions.js` (lecture du répertoire, parsing tolérant — un
   fichier illisible est **ignoré et journalisé**, jamais fatal —, vivacité du pid, `stuck_suspect`,
   `cwd` relatif) ; `src/agent/gitHead.js` (`D7`) ; `src/agent/iakaframeVersion.js` (`D8`). Fonctions
   pures à `fs` injecté, testées sur `test/fixtures/`.
6. **Publication** `src/agent/publisher.js` + `src/agent/index.js` : connexion + testament (`D6`),
   différentiel, battement, purge au démarrage, fin de session, arrêt propre, republication
   intégrale à la reconnexion. Client MQTT **injecté** (faux client en test). ⇒ `G-TESTAMENT`,
   `G-PURGE`, `G-EXEC` **verts**.
7. **Console** `src/console/fleetModel.js` (règle `D9`, pure) + `render.js` + `index.js`
   (`ls`, `watch`, `--json`). ⇒ `G-VIDE` **vert**.
8. **Service** : `deploy/launchd/com.naonedge.iakafleet-agent.plist`,
   `deploy/windows/Install-IakaFleetAgent.ps1` + `Uninstall-IakaFleetAgent.ps1` (tâche planifiée au
   logon, redémarrage sur échec — patron `~/work/iakaHub/deploy/`), `deploy/systemd/iakafleet-agent.service`
   (user). Placeholders, **aucun chemin machine en dur**.
9. **Documentation** : `docs/installation.md` (installer l'agent, poser `~/.iakafleet/.env`, lancer
   la console, lire les états), `CLAUDE.md` (ce qu'est le projet, stack, commandes, backlog,
   **section Forgejo corrigée** — `F13`).
10. **Lot 0 — broker** (session interactive autorisée, cf. `R6`) : lignes ACL de `D5` dans
    `~/work/naonedge-vps/stack/mosquitto/acl` (commit **dans ce dépôt-là**), `passwd` régénéré sur
    le VPS (saisie interactive), rechargement **SIGHUP** du broker, secrets dans Vaultwarden et dans
    `~/.iakafleet/.env` de chaque machine. *Peut être joué avant l'étape 1 si l'autorisation est là.*
11. **Recette réelle** `CA-R1`→`CA-R8` sur le Mac et le PC Windows.
12. `iakaframe snapshot --reason version --version v0.2.0 --note "MVP vue flotte lecture seule"` +
    récit de reprise.

---

## 6. Fichiers concernés

- `package.json`, `package-lock.json` — **créés** (dépendance unique `mqtt`, épinglée).
- `src/shared/contract.js`, `src/shared/config.js` — **créés**.
- `src/agent/index.js`, `sessions.js`, `gitHead.js`, `iakaframeVersion.js`, `publisher.js` — **créés**.
- `src/console/index.js`, `fleetModel.js`, `render.js` — **créés**.
- `test/*.test.js`, `test/fixtures/sessions/*.json`, `test/fixtures/git/**` — **créés**.
- `deploy/launchd/…plist`, `deploy/windows/*.ps1`, `deploy/systemd/…service` — **créés**.
- `docs/installation.md`, `README.md`, `.env.example` — **créés**.
- `CLAUDE.md` — **mis à jour** (projet, stack, commandes, backlog, section Forgejo).
- `specs/etat-des-lieux.md` — récit de reprise.
- *(hors dépôt, Lot 0)* `~/work/naonedge-vps/stack/mosquitto/acl` (+ `passwd` sur le VPS, non
  commité) ; Vaultwarden ; `~/.iakafleet/.env` sur chaque machine.

**Non touchés, vérifiable** : tout `~/work/iakaHub/`, `~/work/iakaframe/`, `~/work/iakaTokenCounter/`,
`~/work/iakaboxlogs/`, `~/.claude/` (**lu, jamais écrit**).

---

## 7. Risques

| # | Risque | Gravité | Mitigation |
|---|---|---|---|
| **R1** | 🛑 **Le format des fichiers de session est interne** (`F17`) : un changement de Claude Code rend l'agent aveugle | **haute** | Parsing **tolérant** (champ absent ⇒ `null`, jamais plantage) ; `sessions_dir_ok` et `sessions_count` publiés ; la console affiche « sessions illisibles ». Source alternative documentée (hooks `SessionStart`/`Stop`) **nommée pour l'itération suivante**, pas faite |
| **R2** | **Windows sans fichiers de session** (`M1`) | **haute** | Étape 0 **bloquante** : arrêt et re-cadrage plutôt qu'un agent Windows qui publie « 0 session » en se croyant juste |
| **R3** | 🛑 **Testament jeté en silence** si l'ACL ne couvre pas son topic (`F20`) : une machine morte resterait « en ligne » | **haute** | ACL `readwrite iakafleet/<host>/#` (`D5`) ; **`CA-R3` tue l'agent pour de vrai** et exige de voir « hors ligne » — un testament non prouvé n'est pas livré |
| **R4** | **« busy » éternel** (`F15`) ou `sessionId` périmé (`F16`) | moyenne | Clé = pid ; `stuck_suspect` publié et affiché « busy ? » ; vivacité jugée sur le pid |
| **R5** | **Fuite d'information** vers le broker (noms de session, chemins) | moyenne | Broker à soi, TLS, ACL fermée ; `cwd` relatif au home ; identifiants de session et socket **non publiés** (`D4`) |
| **R6** | **Lot 0 touche la prod** d'un autre dépôt (`naonedge-vps`) | moyenne | Session interactive autorisée par le décideur ; ACL **additive** (les trois comptes existants inchangés) ; `mosquitto_passwd` **sans `-c`** (il écrase le fichier) ; SIGHUP et non redémarrage |
| **R7** | **Deux agents sur le même `<host>`** (même client id) : le broker déconnecte l'ancien ⇒ bascule en boucle | faible | client id fixe `iakafleet-agent-<host>` documenté ; le service est l'**unique** lanceur |
| **R8** | **Horloges décalées** entre machines faussent la péremption | faible | Seuil large (3 × battement) ; `t` affiché brut en `--json` |
| **R9** | Chaîne d'approvisionnement `mqtt` | faible | Version épinglée exacte, lockfile commité, arbre mesuré (`M4`) |
| **R10** | La **mise en veille** du Mac fait passer la machine « hors ligne » | — | **Comportement voulu** : c'est la vérité. Documenté |

---

## 8. Critères d'acceptation

### Tests automatiques (`npm test`, sans réseau)

- [ ] **CA-1** — `npm test` et `npm run lint` verts ; les gardes `G-EXEC`, `G-VIDE`, `G-TESTAMENT`,
      `G-PURGE` ont été **vues rouges** avant leur code (trace dans `git log`).
- [ ] **CA-2** — Contrat : chaque message publié par l'agent a un payload `{"v","t"}` à **deux clés
      exactement**, `v` scalaire ou `null`, `t` entier ; tout topic commence par `iakafleet/<host>/`
      et finit par `/current`.
- [ ] **CA-3** — `G-EXEC` : aucun fichier de `src/` n'importe `child_process` (ni `node:child_process`).
- [ ] **CA-4** — Sessions (fixtures) : un fichier `idle`, un `busy`, un `busy` avec `parkedJobId` et
      `updatedAt > statusUpdatedAt` (⇒ `stuck_suspect = true`), un JSON corrompu (**ignoré, journalisé,
      les autres publiés**), un pid mort (**non publié**).
- [ ] **CA-5** — Git : dossier `.git` sur branche ⇒ nom ; worktree (fichier `.git`) ⇒ branche du
      worktree ; `HEAD` détaché ⇒ `detached:<sha7>` ; hors dépôt ⇒ `null`.
- [ ] **CA-6** — `G-PURGE` : une session qui disparaît ⇒ payload **vide, retained** sur **chacun** de
      ses codes ; au démarrage, un retained d'un pid mort est effacé, celui d'un pid vivant conservé.
- [ ] **CA-7** — `G-TESTAMENT` : options de connexion portant will `iakafleet/<host>/meta/online/current`,
      `{"v":false,…}`, `retain: true`, `qos: 1`, keepalive 30 ; arrêt propre ⇒ `online=false` publié
      **avant** la déconnexion.
- [ ] **CA-8** — Différentiel : deux balayages identiques ⇒ **zéro** republication hors `last_tick_at` ;
      après reconnexion ⇒ republication **intégrale**.
- [ ] **CA-9** — Règle `D9` : les cinq états machine (en ligne, muette, hors ligne, inconnue, sessions
      illisibles) et l'affichage « ? (dernier connu …) » des sessions d'une machine non en ligne.
- [ ] **CA-10** — `G-VIDE` : broker injoignable, auth refusée ⇒ message sur stderr + **code 2** ; flotte
      vide ⇒ message explicite + code 0.
- [ ] **CA-11** — Config : `mqtt://` vers une adresse autre que `127.0.0.1`/`localhost` ⇒ **refus au
      démarrage** ; `<host>` invalide ⇒ refus nommé.

### Recette réelle (Gimli, puis le décideur)

- [ ] **CA-R1** — Lot 0 : `mosquitto_sub -u fleet-console … -t 'iakafleet/#'` fonctionne ; le même
      compte **ne peut pas publier** (MQTT 5, `PUBACK` code **135 Not authorized** — pas de conclusion
      sur un `-V 4` silencieux, piège déjà payé au CA-6 du broker) ; `fleet-<mac>` **ne peut pas**
      publier sous `iakafleet/<pc>/`. Les trois comptes existants fonctionnent comme avant.
- [ ] **CA-R2** — Agents lancés **par leur service** sur le Mac **et** le PC : `iakafleet ls` depuis le
      Mac **et** depuis le PC montre **les deux machines en ligne**, OS, versions (dont
      `iakaframe_version`) et les sessions ouvertes, avec **dépôt et branche** corrects.
- [ ] **CA-R3** — 🛑 **Testament prouvé** : tuer l'agent **sans arrêt propre** (`kill -9` / fin de
      tâche forcée) ⇒ la machine passe **hors ligne** dans la console en **≤ 60 s**. Idem en coupant
      le réseau de la machine.
- [ ] **CA-R4** — Transition vivante : lancer une tâche dans une session ⇒ `busy` visible en
      `watch` en **≤ 10 s** ; fin du tour ⇒ `idle`. Fermer la session ⇒ elle disparaît en ≤ 10 s.
- [ ] **CA-R5** — Redémarrage de l'agent après fermeture d'une session pendant son arrêt ⇒ la
      session fantôme est **purgée** au démarrage.
- [ ] **CA-R6** — `rc` : une session sous `/rc` est marquée joignable, une autre non (sous réserve `M5`).
- [ ] **CA-R7** — Aucun secret dans l'arbre git (`git grep -iE 'IAKAFLEET_.*PASS=.+'` ⇒ rien) ;
      secrets présents dans Vaultwarden `work/iakaFleet`.
- [ ] **CA-R8** — **Testé par le décideur** depuis une de ses machines.

---

## 9. Instructions suivantes (listées, non détaillées)

1. **`ordres-accuse-reception.md`** — envoyer un ordre à une session précise ; accusés
   *reçu → remis → accepté/refusé → terminé* ; **remise à la session** (pistes à instruire : channels
   MCP `F22`, hooks), **jamais d'exécution par l'outil** ; trace des ordres et accusés (réutilisation
   d'iakaboxlogs, patch TLS d'`iakalog.mjs` alors requis) ; comptes/ACL d'émission.
2. **`autorisations-gestes-destructifs.md`** — qui peut ordonner quoi à qui ; confirmation humaine des
   gestes destructifs ; purge d'une machine retirée ; rotation/révocation d'un compte de machine.
3. **`console-graphique.md`** *(optionnel)* — widget IakaCockpit ou page WSS (compte navigateur dédié).
4. **`liaison-iakahub.md`** *(optionnel)* — iakaHub importe l'agent, ou notifie Discord d'une machine
   hors ligne ; couplage **jamais obligatoire**.
5. **`source-hooks-sessions.md`** *(si `R1` se matérialise)* — état des sessions par hooks documentés
   plutôt que par le fichier interne.

---

## 10. Estimation (jalon P1→P2)

> Ordre de grandeur **assumé et révisable**, pas un engagement ferme. Rappelé à la clôture du lot.

| Poste | j-h |
|---|---:|
| Étape 0 — mesures sur deux machines | 0,25 |
| Lot 0 — ACL + comptes + secrets (prod VPS) | 0,25 |
| Contrat + config | 0,25 |
| Lecture de l'hôte (sessions, git, version) + fixtures | 0,6 |
| Publication (testament, différentiel, purge, reconnexion) + gardes | 0,75 |
| Console (modèle, rendu, `ls`/`watch`/`--json`) | 0,5 |
| Services 3 OS + documentation | 0,5 |
| Recette réelle Mac + PC | 0,4 |
| **Total** | **≈ 3,5** (fourchette **3 – 4,5**) |

**Complexité : moyenne.** Aucune brique difficile ; l'effort va à l'**honnêteté** de la vue (testament
prouvé, absence jamais déguisée en vide, fantômes purgés).
**Risque : moyen**, concentré sur un format **non documenté** (`R1`) et sur **Windows** (`R2`, `F9`).

**Inconnues susceptibles de faire glisser**
1. `M1` négatif (pas de fichiers de session sous Windows) : **re-cadrage**, +1 à +1,5 j-h (hooks).
2. Service Windows jamais recetté dans le portefeuille (`F9`) : **+0,25 à +0,5**.
3. Accès au PC Windows pour la recette : **délai**, pas charge.
4. `bw` absent du Mac à ce jour (backlog `naonedge-vps`) : versement Vaultwarden par l'interface web.
5. Autorisation du Lot 0 (prod VPS) : **délai**, pas charge.
