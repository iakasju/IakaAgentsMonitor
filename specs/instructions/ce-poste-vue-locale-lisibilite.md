# Instruction : « Ce poste » — vue locale complète, titre, lisibilité et sobriété CPU

> Cadrée par **Gandalf** (P1) le **2026-09-30**, sur le retour de recette du décideur (app lancée en
> `tauri dev`, worktree `feat/fusion-flotte` @ `6ac4d47`) et son **« go sur 1, 2, 3 »** : (1) un vrai
> titre « Ce poste », (2) la vue locale alimentée comme la flotte, (3) la lisibilité. Exécution :
> **Gimli** (P2). Les **directions visuelles** de (3) sont produites en parallèle par **Loki**
> (`~/work/iakagraph/etudes/IakaAgentsMonitor/lisibilite/`) : **cette instruction ne choisit pas le visuel**,
> elle grave les **exigences testables** que toute direction retenue doit tenir.
>
> Conventions : `F*` fait · `M*` mesure par Gimli · `D*` décision · `R*` risque · `CA-*` critère.
>
> ✅ **TRANCHÉ par le décideur le 2026-09-30** (verbatim : *« A mes en gardant les coloris actuelles juste
> un peu accentués. reco ok »*) : `D1`, `D2` (union qui remplace Q3-A), `D3`, `D7` **acceptées telles
> quelles** ; `D6` **tranchée autrement** que la recommandation initiale (plancher **14 / 13 px**, trois
> crans **14 / 16 / 18 px**) ; direction visuelle **A « Tableau calme »** de Loki **+ la phrase d'état de C**
> en tête de « Ce poste » ; **coloris actuels conservés, seulement accentués** (`D8`). Plus aucune question
> ouverte (§ 9).

---

## 1. Problème

La recette montre une app qui **dit moins que la CLI** : la vue locale n'affiche qu'un agent (`odin`, projet
`work`) quand `iakafleet ls` montre 4 sessions et les agents `gimli`, `legolas`, `main` de `azime-be` ; la
section locale n'a **pas de titre** ; le texte est **trop petit et trop clair** ; la section « Autres
machines », vide tant que le PC n'est pas joint, est à peine lisible ; l'app (build de dev) a des **pics CPU
de 35 à 90 %** toutes les quelques secondes. La consigne `D4` de la fusion (« Ce poste fonctionne
exactement comme aujourd'hui ») a été tenue à la lettre — elle était **mal posée** : le décideur attend
que « Ce poste » montre **toutes** les sessions et **tous** les agents de la machine.

## 2. Faits

| Id | Fait | Preuve |
|---|---|---|
| **F1** | Aucun titre pour la vue locale ; seule « Autres machines » en a un (`title.textContent = "Autres machines"`) | `src/render.ts:603` · `renderAgents` `:464-493` |
| **F2** | « Autres machines » vide ⇒ `fleetNote("Aucune autre machine n'a publié", "empty")` | `src/render.ts:614` |
| **F3** | La vue locale vient d'une **bascule stricte** : bus local joignable ⇒ fiches du bus **seules** ; sinon ⇒ **détection par transcripts** (`claude_code_files::cards_from_dir`) **seule**. Jamais de fusion | `src-tauri/src/watcher.rs:87-97` |
| **F4** | 🛑 La détection par transcripts **parcourt tout `~/.claude/projects`** (`group_sessions` → `claude_transcript_files`) et lit la tête de chaque transcript, **vivant ou non**, à chaque tick | `src-tauri/src/sources/claude_code_files.rs:212-215,255-309` |
| **F5** | Tick du watcher : **5 s**, péremption `IAKAAM_LIVENESS_SECS` = 90 s par défaut ⇒ un agent sans écriture de transcript depuis 90 s **disparaît**, même si sa session vit | `watcher.rs:22,26` |
| **F6** | Le démon, lui, énumère les sessions **par `~/.claude/sessions/<pid>.json` + vivacité du pid** (`isPidAlive` = `kill(pid,0)`, `EPERM` = vivant) et les fiches par **`~/.cache/iaka-agents/<sessionId>/<agentId>.json`** | `fleet/src/agent/sessions.js:7-11,34,63` · `fleet/src/agent/cards.js:2,18` |
| **F7** | Le publieur écrit la fiche sur disque **avant** de la publier ; les hooks sont **installés sur le Mac** depuis le 2026-09-30 (`L-INSTALL` (a)) | `publishers/claude-code/iaka-agents-publish.mjs:36,126` · `fusion-flotte-vue-distante.md` § 11.1 |
| **F8** | `fleet_config::load()` fournit déjà le `<node>` de la machine à l'app | `src-tauri/src/fleet_config.rs:49-51,113` · `watcher.rs:62-63` |
| **F9** | Corps de texte à **9, 10, 11 et 12 px** (`font-size` en dur, ≥ 20 occurrences) ; valeurs secondaires en `--sc-ink-faint` | `src/styles.css:149,177,185,217,231,242,258,329,338,358,415,434,490,502,506,523,531` |
| **F10** | `--sc-ink-faint` = **`#a3a199`** (clair) et **`#63666c`** (sombre). Contraste estimé **≈ 2,5:1** sur blanc et **≈ 3:1** sur fond sombre — **sous 4,5:1** (à mesurer exactement, `M3`) | `src/styles.css:36,86,118` |
| **F11** | Exigences qui **doivent survivre** (liste-des-agents A1–A7) : carré arrondi à la couleur de pastille + lettre du persona (A2), indentation de délégation `├─ └─ │` (A1/A3), tri ▲/▼, modes **compact 5 colonnes / large 8 colonnes** mémorisés (A4), barre « Reste » et repère de dépassement (A4), groupement par dépôt avec bandeau coloré stable (A6/A7), colonne État désactivée tant que les ordres manquent (A6), largeur et hauteur ajustées au contenu (A6/A7), thèmes clair/sombre + bascule (A3), contraste ≥ 4,5:1 déjà exigé pour la ligne de groupe (A7) | `specs/instructions/liste-des-agents.md:5-11` |
| **F12** | Mesure du décideur (build **de dev**) : pics CPU 35–90 % toutes les quelques secondes, 44 Mo, une connexion TLS 8883 établie | retour de recette, relayé par Odin |

### Mesures confiées à Gimli (étape 0)

| Id | À mesurer |
|---|---|
| **M1** | Nombre de transcripts sous `~/.claude/projects` et **durée d'un tick** du watcher sur le Mac (instrumentation temporaire), bus local présent et absent : confirme ou infirme `F4` comme 1re cause des pics |
| **M2** | Le bus local (`127.0.0.1:1883`) était-il joint pendant la recette (infobulle « source : bus » ou « fichiers ») ? |
| **M3** | Contraste exact (formule WCAG 2.x) de **chaque** couple encre/fond de `styles.css`, clair et sombre |
| **M4** | CPU et mémoire **sur un build de publication** (`npm run tauri build`), selon le protocole `D7` |

## 3. Décisions

### `D1` — Source de « Ce poste » : **l'app lit elle-même les sources disque du démon** — ✅ *acceptée (option i)*

| Option | Pour | Contre | Coût |
|---|---|---|---|
| **(i) l'app lit `~/.claude/sessions` + `~/.cache/iaka-agents`** (portage Rust de `sessions.js`/`cards.js`, même règle, vecteurs partagés) | le Monitor **marche seul** : ni broker, ni démon ; même vérité que la CLI ; O(sessions vivantes) | un portage Rust de plus (vérifié par vecteurs) ; vivacité du pid à écrire en Rust | **≈ 1,0 j-h** |
| (ii) via le démon / bus local | un seul lecteur | le démon ne publie **que** vers le central ; il faudrait qu'il publie aussi en local, et la vue locale **dépendrait** du démon et d'un broker local (iakahub vit avec la tray iakaTokenCounter) — contraire à « aucun centre obligatoire » | ≈ 1,2 j-h |
| (iii) via le central | aucun code local | **dépend du réseau** : « Ce poste » vide hors ligne ; contraire à CA-R4 | écartée |

**Recommandation : (i).** Vivacité du pid : `libc::kill(pid, 0)` sous `cfg(unix)` (Mac, Linux) — `ESRCH` =
mort, `EPERM` = vivant — la crate `libc` est déjà dans l'arbre de dépendances de Tauri (**à vérifier**,
`cargo tree -i libc`) ; l'ajouter en dépendance directe explicite. Windows : hors périmètre (GUI Mac
d'abord).

### `D2` — Règle d'assemblage de « Ce poste » : **la session fait foi, les fiches la peuplent**

1. **Sessions** = fichiers de `~/.claude/sessions/` dont le pid est **vivant** (règle `sessions.js`,
   `stuck_suspect` compris). Une session vivante **s'affiche toujours**, même inactive depuis des heures
   (fin de la péremption à 90 s **pour les sessions**).
2. **Coordinateur** de chaque session = la fiche `main` du disque si elle existe, **sinon une ligne
   synthétique** tirée du fichier de session : persona = `main_persona` de la configuration (défaut
   `odin`, règle existante DA-M6), tâche = `name` de la session, état = `status` (`idle`/`busy`/…), projet
   = dépôt de `cwd`.
3. **Sous-agents** = fiches du disque de la session (`~/.cache/iaka-agents/<sessionId>/*.json`), **unies**
   aux fiches du **bus local** s'il est joint, dédoublonnées par (`runner`, `sessionId`, `agentId`) — la plus
   récente (`lastSeenAt`) l'emporte. Péremption des **sous-agents** inchangée (90 s).
4. **Repli par session** (mode dégradé sans hooks) : pour une session vivante **sans aucune fiche**, la
   détection par transcripts est appliquée **à cette seule session** (son transcript
   `~/.claude/projects/<cwd échappé>/<sessionId>.jsonl` et ses sous-agents), **jamais** au répertoire
   entier.
5. **Fiches orphelines** (`sessionId` sans session vivante — `/clear` rend le `sessionId` du fichier de
   session périmé) : affichées sous « agents non rattachés » tant qu'elles sont fraîches (90 s), jamais
   perdues (même règle que la vue distante, `R3` de la fusion).

**Conséquence assumée — amendement d'un arbitrage antérieur** : la **bascule stricte Q3-A**
d'`agnostique-mqtt.md` (bus **ou** fichiers, jamais les deux) est **remplacée** par cette union ; le bus
local reste une source (runners non-Claude à venir), il n'est plus exclusif. ✅ **Confirmé par le décideur
le 2026-09-30** ; Gimli ajoute en tête d'`agnostique-mqtt.md` une ligne « Q3-A remplacée par l'union `D2` de
`ce-poste-vue-locale-lisibilite.md` » (seule écriture autorisée dans cette instruction-là).

### `D3` — Sort de l'ancienne détection par transcripts : **gardée, réduite au repli par session** — ✅ *acceptée*

Retirée comme source principale (elle est la cause probable des pics CPU, `F4`, et masque les sessions
calmes, `F5`) ; conservée **pour une session sans fiche** (`D2`.4), là où elle apporte persona, modèle et
tâche que le fichier de session n'a pas. Son code de parcours du répertoire entier n'est plus appelé par le
watcher.

### `D4` — Titre « Ce poste »

Même gabarit visuel que le titre « Autres machines » (niveau de titre identique), contenu exact :

**`Ce poste — <node> · <N> session(s) · <M> agent(s)`**, puis sur la même ligne l'**état du démon** :

| Cas | Libellé |
|---|---|
| central configuré, présence du propre `<node>` lue | `démon : <libellé>` avec **exactement** les libellés de la vue distante (« en ligne », « muette… », « hors ligne (dernier signe de vie il y a X) »… — `corr-hors-ligne-duree.md` `D2`), **même fonction** de libellé |
| pas de central configuré | `démon : non observé (pas de broker central)` |
| `<node>` indéterminable | `Ce poste — (nom de machine indéterminé)` |

Le propre `<node>` reste **exclu du tableau** « Autres machines » ; sa présence n'est lue que pour ce libellé.
Le titre du tray (effectif) = `M` agents, inchangé.

**Phrase d'état en tête de « Ce poste »** (reprise de la direction C, validée le 2026-09-30) : sous le titre,
une phrase courte qui résume la machine — forme de référence : la phrase de tête de
`direction-C-coup-doeil.html` de Loki, **réduite aux données déjà calculées** (nombre de sessions, dont
`busy`, nombre d'agents actifs, état du démon). Aucune donnée nouvelle n'est introduite pour elle.

### `D5` — « Autres machines » vide : un message **lisible et informatif**

`Aucune autre machine n'a publié sur <hôte du broker>` + sous-ligne « Les machines apparaissent ici dès que
leur démon tourne. » — en **encre de texte secondaire conforme `D6`** (plus la classe `.empty` en
`--sc-ink-faint`).

### `D6` — Lisibilité : **direction A de Loki**, exigences testables — ✅ *tranché le 2026-09-30*

Direction retenue : **A « Tableau calme »** (`direction-A-tableau.html`), **+ la phrase d'état de C** (`D4`).
Références à **LIRE sans les modifier** (dépôt `iakagraph`, hors périmètre d'écriture de Gimli) :

- `/Users/sjupin/work/iakagraph/etudes/IakaAgentsMonitor/lisibilite/direction-A-tableau.html` — **source**
  de la mise en page, de l'échelle typographique et des valeurs de jetons (bloc `:root`, l. 6-12) ;
- `/Users/sjupin/work/iakagraph/etudes/IakaAgentsMonitor/lisibilite/CONTRASTES.md` — rapports calculés et
  mesurés (§ « Direction A ») : **valeurs cibles des encres** ;
- `/Users/sjupin/work/iakagraph/etudes/IakaAgentsMonitor/lisibilite/index.html` et `README.md` — contexte,
  questions tranchées ; `direction-C-coup-doeil.html` — **seulement** pour la phrase d'état.

Exigences :

1. **Jetons** : toutes les tailles passent par l'échelle **`--sc-fs-body / -sec / -chip / -head / -title /
   -hero`** de Loki, et toutes les encres par `--sc-*` ; **aucune** valeur `font-size` en px hors du bloc de
   jetons (`grep` ⇒ 0).
2. **Plancher** : texte courant **≥ 14 px** ; texte secondaire (valeurs secondaires, étiquettes, chips,
   en-têtes) **≥ 13 px** ; **rien sous 12 px**, lettres de pastille comprises.
3. **Trois crans de taille : corps 14 / 16 / 18 px**, repris des crans `s / m / l` de Loki (`data-size`,
   l. 10-12 : sec 13 / 14,5 / 16 px, etc.) ; **réglage dans la barre du haut**, mémorisé comme le thème ;
   la fenêtre se ré-ajuste (A6/A7). **Cran par défaut : 16 px** (`m`, défaut des maquettes de Loki) —
   tranché par le décideur le 2026-09-30 ; 14 et 18 px restent des crans de réglage (un clic pour passer à
   14). Le plancher ci-dessus vaut pour le **plus petit** cran (14 px). Loki a mesuré qu'à 16 px la fenêtre
   de 480 px tient sans défilement horizontal : c'est vérifié à ce cran (`CA-10`).
4. **Contraste WCAG 2.x ≥ 4,5:1** pour **tout** texte, en **clair et en sombre**, sur ses fonds réels ;
   jamais l'exception « grand texte » ; seuls les éléments **décoratifs non textuels** (guides `├─`, filets)
   peuvent descendre à 3:1. `--sc-ink-faint` n'est plus employé pour du texte porteur d'information : ce
   texte passe en `--sc-ink-muted` et se distingue par la **forme** (italique, libellé), comme dans A.
5. **Mise en page à 480 px : deux lignes par agent** (A), **runner et modèle visibles** (pas de masquage
   derrière un clic). ⚠️ Ceci **amende A4 point 1** de `liste-des-agents.md` (le mode compact masquait
   « Modèle ») : le compact garde ses 5 colonnes de tête, le modèle passe en seconde ligne.
6. **Survie de F11** : rien d'autre de la liste F11 ne régresse.

### `D8` — Couleurs : **coloris actuels conservés, seulement accentués** — ✅ *décideur, 2026-09-30*

**Interdit** : changer de palette ou de teintes (accents, pastilles de persona, bandeaux de dépôt A7).
**Autorisé, et c'est la limite haute** : assombrir (clair) / éclaircir (sombre) **juste assez** pour
atteindre ≥ 4,5:1 avec marge, et **seulement** ces jetons :

| Jeton | Actuel (clair / sombre) | Cible (clair / sombre), `CONTRASTES.md` § A |
|---|---|---|
| `--sc-ink-muted` | `#6f6d64` / `#9a9a92` | `#57554d` / `#b4b3ab` |
| `--sc-ink-faint` | `#a3a199` / `#63666c` | `#66645b` / `#a1a099` |
| `--sc-ok`, `--sc-danger` | `#1a7f5a`, `#c1372c` / `#4ad393`, `#ef6b5e` (`styles.css:42,44,89,91`) | clair : `#176b4b`, `#b12f25` (assombris) ; sombre : **inchangés** (≥ 4,74:1 mesuré) |
| `--grp-*-fg` (bandeaux A7) | valeurs A7 (`styles.css:59-69`, `97-107`, `126-136`) — plus faibles : clair `grp-3/4/5` 4,78–4,85:1, sombre `grp-4` 4,82:1 | **même teinte**, assombrie en clair / éclaircie en sombre jusqu'à **≥ 5:1** (les autres `grp-*` ne bougent pas) |

Toute autre différence de valeur entre `direction-A-tableau.html` et `styles.css` (fonds, `--sc-ink`,
accent, pastilles `--sc-sp-*`…) **garde la valeur actuelle de l'app**. Règle de contrôle : pour chaque jeton
modifié, la **teinte** (angle HSL) reste à ± 5° de l'actuelle — seule la luminosité bouge.

### `D7` — Sobriété CPU : **budget et protocole**

**1re cause probable à vérifier (`M1`)** : le parcours complet de `~/.claude/projects` toutes les 5 s (`F4`)
quand le bus local est absent. `D2`/`D3` la suppriment (lecture O(sessions vivantes)).
**Budget, sur build de publication, fenêtre fermée puis ouverte, 10 min au repos (aucun agent actif)** :
durée d'un tick **≤ 50 ms au 95e centile** ; CPU **moyen ≤ 1 %**, **aucun échantillon > 10 %** (hors les
5 premières secondes). Protocole : `npm run tauri build`, lancer le `.app`, échantillonner `ps -o %cpu,rss -p
<pid>` chaque seconde pendant 600 s ; durée de tick journalisée derrière `IAKAAM_TRACE=1` ; consigner les
chiffres dans le commit. ✅ *Budget accepté par le décideur le 2026-09-30.*

## 4. Impacts sur les autres specs

- **`contract-vectors/`** : nouveau fichier **`local-sessions.json`** — normalisation d'un fichier de session
  (champs, `stuck_suspect`, `cwd` relatif) et règle `D2` (session vivante ⇒ ligne ; coordinateur
  synthétique ; fiche orpheline), vivacité **injectée** ; consommé par `fleet/` (Node) **et** l'app (Rust).
  Matrice de `specs/contrat-bus-iaka.md` § 6 : **ligne ajoutée** par ce cadrage.
- **`fusion-flotte-vue-distante.md`** : `D4` (« exactement comme aujourd'hui ») est **amendé** par la présente ;
  CA-R3 et CA-R4 mis à jour (sans cocher).
- **`agnostique-mqtt.md`** : Q3-A (bascule stricte) remplacée par l'union `D2` — **sous réserve** du décideur.

## 5. Périmètre

- **Inclus** : source locale Rust (`D1`, `D2`, `D3`), vecteurs `local-sessions.json` (Node + Rust), titre
  « Ce poste » et phrase d'état (`D4`), message vide (`D5`), lisibilité selon la direction **A** (`D6`) et
  accentuation des coloris (`D8`), instrumentation et mesure CPU (`D7`), tests, doc (`CLAUDE.md` du dépôt) ;
  **une ligne d'amendement** en tête d'`agnostique-mqtt.md` (Q3-A) et de `liste-des-agents.md` (A4 point 1).
- **Exclu** : toute **autre** retouche de couleur (nouvelle teinte, nouvelle palette, pastilles `--sc-sp-*`,
  fonds, accent) ; toute écriture dans `~/work/iakagraph/` (maquettes de Loki **lues seulement**) ; le PC et
  toute GUI Windows ; les **ordres** (boutons État toujours désactivés) ; le testament
  (`corr-hors-ligne-duree.md`) ; toute modification du démon, du publieur ou du contrat de bus hors § 6 ;
  `~/.claude/` (lecture seule).

## 6. Étapes (Gimli)

0. `M1`–`M3` ; `cargo tree -i libc`.
1. **Vecteurs d'abord, vus ROUGES** : `contract-vectors/local-sessions.json` (≥ 8 cas : session vivante
   inactive depuis 3 h ⇒ affichée ; pid mort ⇒ absente ; session sans fiche ⇒ coordinateur synthétique
   `status` repris ; fiche `main` présente ⇒ elle prime ; sous-agent bus + disque même clé ⇒ un seul, le plus
   récent ; fiche orpheline fraîche ⇒ non rattachée ; orpheline périmée ⇒ absente ; `stuck_suspect`). Test
   Node (fleet) et Rust qui les chargent.
2. **Source locale Rust** `src-tauri/src/sources/local_sessions.rs` (+ vivacité `libc`) et assemblage `D2`
   dans `watcher.rs` ; repli par session `D3` ; le parcours complet n'est plus appelé. Vecteurs verts.
3. **Titre « Ce poste »** (`D4`) : données posées par le watcher (node, compteurs, état du démon via la même
   fonction de libellé que « Autres machines ») ; rendu `render.ts`.
4. **Message vide** (`D5`).
5. **Mesure CPU** `D7` sur build de publication **avant/après** l'étape 2 (chiffres dans les commits).
6. **Lisibilité** (`D6`, `D8`) — direction **A** + phrase d'état de **C**, fichiers de Loki **lus, jamais
   modifiés** (chemins absolus en `D6`). Ordre : (a) **test de contraste d'abord**
   (`npm run test:contrast`, script Node sans dépendance qui lit les jetons de `styles.css`, clair **et**
   sombre, et un **registre versionné** des couples encre/fond réellement employés) ⇒ **rouge** sur les
   encres actuelles ; (b) jetons `D8` (accentuation seulement) ⇒ vert ; (c) échelle `--sc-fs-*` et crans
   14/16/18 dans la barre du haut ; (d) mise en page A (deux lignes par agent à 480 px, runner et modèle
   visibles), phrase d'état ; (e) une ligne d'amendement en tête de `liste-des-agents.md` : « A4 point 1
   amendé par `ce-poste-vue-locale-lisibilite.md` `D6`.5 ».
7. `cargo test`, `clippy -D warnings`, `npm run typecheck`, `npm run build`, `npm test` (fleet),
   `npm run test:contrast` ; `CLAUDE.md` mis à jour.

## 7. Critères d'acceptation

- [ ] **CA-1** — `local-sessions.json` vu **rouge** puis vert côté Node **et** Rust (trace `git log`).
- [ ] **CA-2** — **Contre-essai** : réintroduire la péremption à 90 s pour les sessions fait rougir le cas
      « session vivante inactive depuis 3 h » ; désactiver le dédoublonnage fait rougir le cas bus + disque.
- [ ] **CA-3** — Sur le Mac, avec les mêmes sessions ouvertes, **l'app et `iakafleet ls` listent les mêmes
      sessions et les mêmes agents** (hors libellés) — comparaison consignée.
- [ ] **CA-4** — **Sans broker ni démon** (bus local arrêté, démon arrêté par `launchctl bootout`, central
      non configuré) : « Ce poste » montre toutes les sessions vivantes ; titre `démon : non observé…` ; aucune
      erreur.
- [ ] **CA-5** — Session sans hooks (fiches absentes) ⇒ coordinateur synthétique + sous-agents par repli
      transcripts **de cette seule session** ; le watcher n'ouvre **aucun** fichier hors des sessions vivantes
      (test sur répertoire témoin contenant des transcripts morts).
- [ ] **CA-6** — Titre exact `Ce poste — <node> · N session(s) · M agent(s)` et libellé démon **identique** à
      celui d'une machine distante dans le même état (même fonction, test).
- [ ] **CA-7** — « Autres machines » vide : message `D5` avec l'hôte du broker, contraste ≥ 4,5:1.
- [ ] **CA-8** — `grep -nE 'font-size:\s*[0-9]' src/styles.css` hors bloc de jetons ⇒ **0** ; au **plus
      petit** cran (14 px) : texte courant **≥ 14 px**, texte secondaire **≥ 13 px**, **aucune** taille calculée
      < 12 px (mesure `getComputedStyle` sur la fenêtre, consignée) ; au cran par défaut (16 px), tailles
      conformes à l'échelle `m` de Loki.
- [ ] **CA-9** — `npm run test:contrast` : **tous** les couples du registre ≥ 4,5:1 en clair **et** en
      sombre, bandeaux `grp-*` ≥ 5:1 ; **contre-essai** : remettre `--sc-ink-faint` à `#a3a199` le fait rougir.
- [ ] **CA-9b** — **Coloris conservés** (`D8`) : seuls les jetons listés en `D8` diffèrent de `styles.css`
      d'origine (`git diff` des jetons) ; pour chacun, teinte HSL à ± 5° de l'actuelle (vérif par le même script).
- [ ] **CA-10** — Trois crans **14 / 16 / 18 px** dans la barre du haut, **défaut 16 px**, mémorisé au
      redémarrage, fenêtre ré-ajustée ; **à 480 px et au cran 16 px**, deux lignes par agent, runner et modèle
      visibles, aucun défilement
      horizontal ; phrase d'état en tête de « Ce poste ».
- [ ] **CA-11** — **Non-régression F11** : lettre/pastille, indentation, tri, compact/large, « Reste », bandeau
      de dépôt, État désactivé, ajustement de fenêtre, thèmes — vérifiés un par un (capture au gate).
- [ ] **CA-12** — Budget `D7` tenu sur build de publication (chiffres avant/après consignés).
- [ ] **CA-R1** — **Recette du décideur** sur le Mac : titre, sessions et agents complets, lisibilité jugée
      suffisante.

## 8. Estimation

| Poste | j-h |
|---|---:|
| Vecteurs `local-sessions` (Node + Rust) | 0,3 |
| Source locale Rust + assemblage + repli | 1,0 |
| Titre « Ce poste » + message vide | 0,3 |
| Mesure CPU (avant/après, instrumentation) | 0,3 |
| Lisibilité : test de contraste, jetons `D8`, échelle et 3 crans | 0,5 |
| Mise en page A (deux lignes à 480 px, runner/modèle visibles) + phrase d'état | 0,5 |
| Tests, doc, gate | 0,3 |
| **Total** | **≈ 3,2** (fourchette **2,6 – 4,2**) |

*Hausse de 0,4 j-h sur l'estimation initiale* : la direction A demande une **mise en page** (deux lignes par
agent, amendement de A4) et pas seulement des jetons.
**Complexité : moyenne. Risque : moyen** (portage d'une règle en Rust, format interne des fichiers de
session, réagencement du tableau V4 sans casser F11). **Inconnues** : `M1` (si la cause CPU est ailleurs,
+0,3 à +0,5) ; `libc` absent de l'arbre (+0,05). *Ordre de grandeur révisable.*

## 9. Décisions du décideur (2026-09-30) — plus rien à trancher

1. Source `D1` : **(i)** — acceptée.
2. Bascule stricte Q3-A **remplacée** par l'union `D2` — acceptée.
3. Détection par transcripts **gardée en repli par session** (`D3`) — acceptée.
4. Plancher **14 / 13 px**, rien sous 12 px ; **trois crans 14 / 16 / 18 px** dans la barre du haut, **défaut 16 px** (`D6`) —
   tranché (recommandation d'Odin, pas celle du cadrage initial).
5. Budget CPU `D7` — accepté.
6. Direction **A** + phrase d'état de **C** ; **coloris actuels conservés, un peu accentués** (`D8`) ; à 480 px
   deux lignes par agent ; runner et modèle visibles ; bandeaux A7 retouchés dans la limite `D8`.
