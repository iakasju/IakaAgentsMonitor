# Instruction : Liste des agents en cours — MVP tray macOS

> Rédigé par le cadrage (P1, Gandalf) le 2026-09-26. Consommé par Gimli (P2).
> Statut : **VALIDÉE par Stéphane le 26/09/2026** (« go ») : Q1 à Q6 = A. Maquette de référence : `docs/maquettes/visu-agents-5-versions.html`, version 1 (fenêtre de barre de menus), remplacée par le tableau V4 (amendement A1). Les recommandations ci-dessous étaient les valeurs par défaut, retenues par ce « go ».
> **Amendement A1 (Stéphane, 26/09/2026)** : l'affichage par défaut n'est plus la popover V1 mais le **tableau V4** de la maquette (une ligne par agent, colonnes triables, runner et modèle), **avec une indentation qui représente la délégation** (enfant sous son parent). Clic sur l'icône de barre de menus → petite fenêtre avec ce tableau. **Icônes des agents** : pixel art en **4×4 caractères**, proposées par Loki et choisies par Stéphane **avant** implémentation ; en attendant, lettre dans un carré à la couleur de la pastille.
> **Décision A2 (Stéphane, 26/09/2026)** : pour l'instant, l'icône d'un agent reste un **carré aux bords arrondis à la couleur de sa pastille, avec la lettre du persona** (déjà en place, `renderSpriteIcon`). Les essais de pixel art (4×4, 2×2, sprites 16×32, têtes) sont jugés insuffisants ; piste suivante à décider plus tard : portraits libres (pack CC0) ou générateur dédié (Retro Diffusion).
> **Amendement A3 (Stéphane, 26/09/2026, « go »)** : aligner l'app sur la maquette V4 de Loki — thèmes clair et sombre (suit macOS + bascule manuelle), runner et modèle en étiquettes encadrées (police mono), colonne Statut (« vivant »), guides de délégation `├─ └─ │`, indicateur de tri ▲/▼, ordre des colonnes Agent · Projet · Runner · Modèle · Tâche · Démarré · Actif · Statut, noms de persona avec majuscule, polices de la charte Studio clair, fenêtre compacte 520×360 ouverte directement au clic gauche sur l'icône (menu au clic droit : Afficher, Quitter), contraste AA des lettres sur pastille (fg sombre si nécessaire) et lettre distincte pour Gimli (≠ Gandalf), tâche de la session principale = début du premier message utilisateur (tronqué) au lieu de « coordinateur ».
> **Amendement A4 (Stéphane, 26/09/2026, « go »)** — passe corrective après comparaison visuelle d'Odin : (1) la fenêtre compacte 520×360 n'affiche que les 5 colonnes de l'aperçu de la maquette (Agent · Projet · Runner · Actif · Reste), un bouton « agrandir » bascule vers le tableau complet à 8 colonnes (fenêtre élargie), choix mémorisé ; (2) lettre de Legolas = « E » (≠ Loki « L ») ; (3) repère de dépassement `.reste-mark` visible (non rogné par `overflow: hidden`) ; (4) tâche du coordinateur inchangée.
> **Amendement A5 (27/09/2026, « continue » de Stéphane sur la proposition d'Odin)** — retouches après comparaison visuelle : (1) « Actif il y a » formaté comme « Démarré » au-delà de 60 s (« 6 min », jamais « 371 s ») dans les deux modes ; (2) session hors projet iakaframe : afficher le nom du dossier de travail (ex. « work ») au lieu de « hors projet » ; (3) pas de focus clavier visible sur le bouton ⤢ à l'ouverture (focus visible seulement au clavier, `:focus-visible`).
> **Amendement A6 (27/09/2026, maquette validée par Stéphane « A6 ok », commit b614f5b)** — affichage seul, sans effet réel : (1) largeur de fenêtre ajustée au contenu, bornée 480–1200 px, seule la Tâche en ellipse, ancrage sous l'icône conservé ; (2) groupement par dépôt : ligne de séparation pleine largeur au nom du projet, arbre de délégation dessous, colonne Projet retirée des deux modes ; (3) colonne « État » : pastille-bouton running/paused (bascule locale en attendant MQTT + A1 d'agnostique-mqtt), état stopping non cliquable, bouton ■ avec confirmation inline avant stopping ; colonne Reste conservée. Le bouton n'agit sur aucun agent tant que le lot MQTT et son amendement A1 ne sont pas livrés : l'état affiché est toujours « running » côté données, la bascule reste visuelle et doit être signalée comme telle (infobulle « commande disponible avec le bus MQTT »).
> Besoin du décideur, verbatim (26/09) : « la liste des agents, je la veux dans IakaAgentsMonitor ».

---

## Problème

Quand plusieurs sessions Claude Code tournent en parallèle sur plusieurs projets, Stéphane ne
voit pas d'un coup d'œil **qui travaille en ce moment** : quels agents (personas iakaframe), sur
quel projet, qui a délégué à qui, depuis quand. Cette vue a été écrite le 20/09 **dans
iakaTokenCounter** (branche non fusionnée), où elle n'a pas sa place : iakaTokenCounter mesure la
**consommation**, pas l'**activité**. Le besoin est de donner à cette vue son propre produit,
IakaAgentsMonitor, projet neuf et vide à ce jour.

## Ce qui existe (lecture seule, relevé du 2026-09-26)

| Élément | Où | État |
|---|---|---|
| Module de détection complet | `iakaTokenCounter` branche `worktree-agent-a73ae73650ed5e328`, commit `1c9fd0e`, fichier `src-tauri/src/agents.rs` (1041 lignes dont ~400 de tests) | **écrit et testé**, jamais fusionné dans `main` |
| Instruction d'origine | `iakaTokenCounter/specs/instructions/feature-agents-en-cours.md` (présente **sur `main`** et sur la branche) | fait foi pour le comportement repris ici (D1 à D9) |
| Rendu de la popover | branche D7 : `src/render.ts` (l. 251-317, `renderAgents`), `src/types.ts` (l. 140+), `src/main.ts`, `src/styles.css` (l. 60-120), `index.html` | écrit, peinture seule |
| Compteur dans l'icône | branche D7 : `src-tauri/src/icon.rs` (canvas 54×18, rasterisation `resvg`) | **lié à l'icône des jetons**, non réutilisable tel quel |
| Utilitaires réemployés | `iakaTokenCounter/iakatc-core/src/measure/claude.rs` : `claude_projects_dir` (l. 453), `claude_transcript_files` (l. 99), `project_of` (l. 64), `bucket_project` (l. 84), `PORTFOLIO_ROOTS` (l. 76) | ~45 lignes, génériques |
| Tuyauterie tray / popover | branche D7 : `src-tauri/src/tray.rs` (clic gauche = popover, menu clic droit avec entrée « open »), `lib.rs` (`ActivationPolicy::Accessory`), `Info.plist` (`LSUIElement`) | patron à recopier |
| IakaAgentsMonitor | `~/work/IakaAgentsMonitor` | structure iakaframe seule, **aucun code** ; `CLAUDE.md` et `specs/PROJET.md` encore au gabarit |

### Comment la liste fonctionne aujourd'hui (constat)

- **Détection : par fichiers, ni processus ni `sysinfo`.** Toutes les 5 s, un fil dédié parcourt
  `~/.claude/projects/` et ne fait que des `stat()` (aucun transcript relu). Un agent est
  « vivant » si son transcript a été écrit il y a **90 s au plus** (seuil surchargeable).
- **Identité : le fichier compagnon `agent-<id>.meta.json`** que Claude Code écrit à côté de chaque
  transcript de sous-agent : `agentType` (le persona : gandalf, gimli…), `description` (la tâche
  confiée), `parentAgentId` et `spawnDepth` (la délégation, exacte à toute profondeur). Le
  coordinateur de session n'a pas de compagnon : il est affiché par convention comme Odin.
- **Projet** : lu dans le `cwd` de la première ligne du transcript de session (lecture bornée à
  1 Mio), réduit au nom de dossier ; `work`/`Desktop` tombent dans « hors projet ».
- **Affichage** : une ligne par session vivante, `projet  O ( G  L ( N ) )` — lettre du persona
  dans un carré arrondi à la couleur de sa pastille (table figée de 9 personas, gris pour tout type
  hors roster), infobulle `persona — tâche — actif il y a N s`. Bornes : 6 sessions, 8 enfants par
  parent, marqueur `+N`. Compteur d'agents dans l'icône de barre de menus.
- **Circulation : AUCUNE sur le bus MQTT.** La liste vit entièrement dans l'app tray (événement
  interne Tauri `tray://agents`). Le contrat MQTT d'iakaTokenCounter
  (`specs/contrat-mqtt-conso.md`) ne connaît que `coordinator`/`subagent` et des jetons ; il
  n'expose ni persona ni liveness.
- **Générique vs lié aux jetons** : tout `agents.rs` est générique **sauf** deux points —
  `run_watcher` relit le magasin des réservoirs pour recomposer l'icône (`store.cards()`), et
  l'icône partage son canvas avec les jauges de quota. Les utilitaires pris à `iakatc-core` sont
  génériques (aucun calcul de jetons).
- **Dette laissée sur `main` d'iakaTokenCounter** : l'instruction `feature-agents-en-cours.md` et
  un renvoi dans `feature-tray-visuals.md` (§ D2, « canvas étendu à 54 × 18 ») sont fusionnés alors
  que le code ne l'est pas (`icon.rs` de `main` est toujours en 40 × 18).

## Questions fermées (arbitrage de Stéphane)

**Q1 — Déplacer ou copier ?**
- **A (recommandé) — Déplacer.** IakaAgentsMonitor devient le seul foyer de la liste.
  iakaTokenCounter ne garde que les jetons : la branche D7 **n'est pas fusionnée**, elle est
  **conservée intacte** (ni supprimée ni réécrite) comme source historique. Rien à retirer du code
  d'iakaTokenCounter (il n'a jamais été fusionné) ; seules les deux traces de spec sur son `main`
  sont à annoter « transféré vers IakaAgentsMonitor » — **par un cadrage dans le projet
  iakaTokenCounter**, hors de la présente instruction.
- B — Copier : fusionner aussi D7 dans iakaTokenCounter. Double maintenance, et la branche doit
  être rebasée sur un `main` qui a bougé (`rollups.rs`, `quota_history.rs`).
- C — Fusionner D7 maintenant, retirer plus tard : deux chantiers pour zéro gain.

**Q2 — Forme du MVP ?**
- **A (recommandé) — App de barre de menus Tauri 2 autonome** (même stack qu'iakaTokenCounter) :
  le code existant s'y transpose presque tel quel, et c'est la forme « d'un coup d'œil » voulue au
  départ.
- B — Commande terminal (`iakaagents`, affichage texte rafraîchi) : moins chère (~0,5 j), mais il
  faut ouvrir un terminal pour voir.
- C — Page web locale : exige un serveur et un port, sans gain pour un seul poste.
- D — iakaHub / Discord : suppose un transport multi-machines, hors MVP.

**Q3 — Réutilisation ?**
- **A (recommandé) — Recopier** `agents.rs` et les ~45 lignes d'utilitaires d'`iakatc-core`
  dans IakaAgentsMonitor, avec un commentaire de provenance (dépôt, branche, commit `1c9fd0e`).
  App autonome, aucun service à lancer, aucun port.
- B — Dépendre d'`iakatc-core` en dépendance git : il faudrait un jeton Forgejo au build (dépôt
  privé), on tirerait tout le code de mesure de conso pour 45 lignes, et chaque évolution
  d'iakaTokenCounter pourrait casser ce projet.
- C — S'abonner au bus MQTT : **la liste n'y circule pas**. Il faudrait d'abord ajouter un
  producteur dans `iakatc-daemon` (modifier iakaTokenCounter) et exiger que ce démon tourne ; le
  bus écoute en plus sur `127.0.0.1`, il n'apporte donc rien au multi-machines.

**Q4 — Informations par agent dans le MVP ?**
- **A (recommandé)** — persona (lettre + couleur de pastille), projet, tâche confiée
  (`description`), parenté de délégation, **dernière activité** (« actif il y a N s ») et
  **démarré il y a** (date de création du transcript, déjà lue par le code pour le tri).
- B — A + nom de la machine : sans intérêt tant qu'une seule machine est surveillée.
- C — Sans « démarré il y a » (reprise stricte de D7).
- Non disponible sans lire le contenu des transcripts, donc **exclu** : un « état » plus fin que
  vivant/terminé (en réflexion, en attente d'outil…).

**Q5 — Icône de barre de menus ?**
- **A (recommandé)** — Icône fixe monochrome (modèle macOS) + **nombre d'agents en texte** à
  côté (`set_title` de Tauri). Aucune rasterisation SVG, aucune dépendance `resvg`.
- B — Reprendre le compteur dessiné (carré gris) d'`icon.rs` : plus de code pour un rendu proche.

**Q6 — Plateformes ?**
- **A (recommandé)** — **macOS seul** dans ce lot ; l'IdeaPad (Linux) fait l'objet d'un lot 2
  cadré à part (sur Linux, le clic sur l'icône n'émet aucun événement avec le moteur
  AppIndicator : il faudra passer par le menu, cf. Sources).
- B — Les deux maintenant : l'environnement de bureau de l'IdeaPad n'est pas connu, risque de
  glissement.

## Décision retenue (sous réserve de Q1 à Q6, valeurs recommandées)

App de barre de menus **Tauri 2** autonome, **macOS**, qui **transpose** le module de détection de
la branche D7 d'iakaTokenCounter. Détection par fichiers (`stat()` + compagnons `.meta.json`),
tick 5 s, liveness 90 s, table de personas figée, tout le calcul en Rust et le TypeScript qui ne
fait que peindre (D9 d'origine). Pas de bus, pas de service, pas de port, pas de Docker (app
native de bureau, comme iakaTokenCounter). La liste **quitte** iakaTokenCounter (branche D7
conservée, non fusionnée).

Décisions arbitraires (mineures, révisables en une phrase) :
- **DA-1** Variable de seuil renommée `IAKAAM_LIVENESS_SECS` (défaut 90).
- **DA-2** Le titre de l'icône affiche **toujours** le nombre, `0` compris : la largeur de l'item ne
  bascule pas à chaque apparition d'agent.
- **DA-3** Identifiant d'app `com.iakaframe.iakaagentsmonitor` (même famille que
  `com.iakaframe.iakatokencounter`).
- **DA-4** Au repos (aucun agent), la popover affiche une ligne « Aucun agent en cours » (dans
  iakaTokenCounter la section était masquée ; ici la liste *est* l'app, une fenêtre vide serait
  ambiguë).

## Périmètre

- **Inclus** : projet Tauri 2 + TypeScript/Vite à la racine du dépôt ; module `agents.rs`
  transposé avec ses tests ; utilitaires recopiés ; icône de barre de menus avec compteur texte ;
  popover (clic gauche) listant les sessions et leurs agents ; menu clic droit « Afficher » /
  « Quitter » ; ajout de « démarré il y a » dans l'infobulle ; app sans icône de Dock ;
  complétion des sections « Ce qu'est ce projet » et « Commandes » du `CLAUDE.md` du projet.
- **Exclu** : toute modification du dépôt iakaTokenCounter (code, specs, branches) ; Linux /
  IdeaPad ; plusieurs machines ; bus MQTT ; nom de machine ; jetons, coût, historique ; Codex
  (aucune donnée de persona ni de délégation) ; vrai pixel art ; couleur par phase de
  gimli/legolas ; clic sur un agent pour ouvrir sa session ; notifications ; lancement au
  démarrage de session ; signature / notarisation / installeur ; icône soignée (provisoire
  acceptée).

## Étapes d'implémentation

1. **Squelette.** Créer l'app Tauri 2 (dernière 2.x, 2.11.6 au 26/09) + Vite + TypeScript à la
   racine : `package.json` (scripts `dev`, `build`, `typecheck`, `tauri`), `src-tauri/`
   (`Cargo.toml`, `tauri.conf.json` avec `productName` « IakaAgentsMonitor », identifiant DA-3,
   une fenêtre `popover` cachée, sans décoration), `Info.plist` avec `LSUIElement`. Commit.
2. **Utilitaires.** `src-tauri/src/claude_paths.rs` : recopie de `claude_projects_dir`,
   `claude_transcript_files`, `project_of`, `bucket_project`, `PORTFOLIO_ROOTS` et du seau
   « hors projet », commentaire de provenance, **et leurs tests** repris d'`iakatc-core` s'ils
   existent (sinon : cwd normal, racine `work`, chemin avec `/` final). Commit.
3. **Module agents.** `src-tauri/src/agents.rs` : recopie du fichier de la branche D7 (`1c9fd0e`),
   avec trois adaptations seulement : (a) import depuis `crate::claude_paths` ; (b) `run_watcher`
   ne relit **plus** de magasin de réservoirs, il met à jour l'icône avec le seul effectif ;
   (c) horloge et seuil lus localement (`IAKAAM_LIVENESS_SECS`, DA-1). **Tous les tests
   d'origine** passent sans modification de leurs assertions. Commit.
4. **« Démarré il y a » (Q4-A).** Ajouter à l'infobulle d'un sous-agent ` — démarré il y a <durée>`
   à partir de la date de création du transcript déjà lue (`created`) ; durée en `s` sous 60 s,
   en `min` au-delà, en `h min` au-delà de 60 min ; date indisponible ⇒ mention omise. La durée
   est **formatée en Rust** (D9). Tests : 45 s, 5 min, 1 h 12 min, date absente. Commit.
5. **Tray.** `src-tauri/src/tray.rs` + `lib.rs` : icône fixe en mode modèle macOS, titre = effectif
   (DA-2), infobulle « N agent(s) en cours », clic gauche = afficher/masquer la popover, menu
   « Afficher » / « Quitter », `ActivationPolicy::Accessory`, commande
   `get_running_agents`, watcher lancé au démarrage. Commit.
6. **Front.** `index.html`, `src/main.ts`, `src/render.ts`, `src/types.ts`, `src/styles.css` :
   recopie de la partie agents de la branche D7 (peinture seule), + état vide DA-4. Aucune
   décision côté TS. Commit.
7. **CLAUDE.md du projet.** Remplir « Ce qu'est ce projet » (2-3 lignes) et « Commandes »
   (`npm run tauri dev`, `npm run build`, `npm run typecheck`, `cargo test` / `cargo clippy` dans
   `src-tauri`). Commit.
8. **Vérification.** `cargo test`, `cargo clippy --all-targets -- -D warnings`,
   `npm run typecheck`, `npm run build`, `npm run tauri build` (bundle `.app`), puis remise au
   gate qualité (Legolas) et essai réel par Stéphane.

## Fichiers concernés

- `package.json`, `tsconfig.json`, `vite.config.ts`, `index.html` — **nouveaux**.
- `src/main.ts`, `src/render.ts`, `src/types.ts`, `src/styles.css` — **nouveaux** (rendu).
- `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/Info.plist`,
  `src-tauri/build.rs`, `src-tauri/capabilities/default.json`, `src-tauri/icons/` — **nouveaux**.
- `src-tauri/src/main.rs`, `lib.rs`, `tray.rs`, `state.rs`, `claude_paths.rs`, `agents.rs` —
  **nouveaux**.
- `CLAUDE.md` — sections « Ce qu'est ce projet » et « Commandes » complétées.
- `.gitignore` — déjà prêt (`node_modules/`, `target/`, `dist/`, `.env`), à ne pas affaiblir.

## Risques

- **R1 — Le format `.meta.json` n'est pas une API publiée** de Claude Code. Il est confirmé à ce
  jour par plusieurs outils tiers (cf. Sources). *Mitigation* : lecture entièrement défensive
  héritée de D7 (compagnon absent ou invalide ⇒ agent gris « inconnu », jamais ignoré ni
  plantage).
- **R2 — Fantômes / clignotements** (seuil 90 s). *Mitigation* : `IAKAAM_LIVENESS_SECS` pour
  ajuster en recette sans recompiler.
- **R3 — Divergence avec la branche D7** si quelqu'un la reprend un jour. *Mitigation* : Q1-A
  (un seul foyer) + commentaire de provenance.
- **R4 — Toolchain Rust + Node sur le Mac.** Déjà en place pour iakaTokenCounter (même stack) ;
  aucun nouvel outil requis. Si ce n'était plus le cas, remonter à Stéphane avant d'installer.
- **R5 — Largeur de l'item de barre de menus** qui change entre 1 et 2 chiffres. Accepté (DA-2).

## Critères d'acceptation

- [ ] `cargo test` vert dans `src-tauri`, incluant **tous** les tests repris d'`agents.rs` (roster,
      unicité des couples lettre/couleur, compagnon, liveness 0/89/90/91 s, arbre, bornes 6/8,
      projet, instantanés égaux) et les nouveaux tests de `claude_paths.rs` et de « démarré il y a ».
- [ ] `cargo clippy --all-targets -- -D warnings` sans avertissement.
- [ ] `npm run typecheck` et `npm run build` OK ; `npm run tauri build` produit une `.app`.
- [ ] `Cargo.toml` ne contient **ni** `iakatc-core`, **ni** client MQTT, **ni** `resvg` ;
      l'app n'ouvre **aucun** port (vérifiable par `lsof -iTCP -sTCP:LISTEN` pendant qu'elle tourne).
- [ ] Au lancement, **aucune icône dans le Dock** ; une icône apparaît dans la barre de menus.
- [ ] Aucun agent vivant : titre `0`, popover « Aucun agent en cours ».
- [ ] **Essai réel (Stéphane)** : une session Claude Code active dans un projet X, avec 2 à 3
      délégations dont une imbriquée ⇒ la popover affiche une ligne `X  O ( … ( … ) )` conforme à
      la délégation réelle, le titre donne le bon effectif (coordinateur compris), l'infobulle de
      chaque agent donne persona, tâche, « actif il y a », « démarré il y a ».
- [ ] Apparition d'un agent en **≤ 5 s** après sa première écriture ; disparition en **≤ 90 s**
      après la dernière.
- [ ] Un type hors roster (ex. `general-purpose`) s'affiche en **gris** avec sa propre lettre.
- [ ] Aucun fichier du dépôt iakaTokenCounter modifié ; aucun secret dans le dépôt.

## Estimation (ordre de grandeur, révisable)

- **Charge : ~1,5 jour-homme** (fourchette 1 à 2 j) — squelette 0,3 ; transposition + tests 0,4 ;
  tray + popover 0,3 ; « démarré il y a » 0,1 ; CLAUDE.md 0,1 ; qualité + recette 0,3.
- **Complexité / risque : faible à moyen.** Le cœur est déjà écrit et testé ; le risque tient au
  format non contractuel de Claude Code et aux réglages de fenêtre popover sous macOS.
- **Inconnues qui peuvent faire glisser** : réponses différentes de la recommandation à Q2/Q5
  (option B de Q5 : +0,5 j ; Q6-B : +1 à 2 j) ; changement de format des `.meta.json` d'ici
  l'implémentation ; comportement de la popover sur plusieurs écrans.

## Ce qui appartient à Stéphane

- Trancher **Q1 à Q6** (ou dire « go » pour les recommandations).
- Décider du sort de la branche D7 dans iakaTokenCounter (recommandé : conservée, non fusionnée,
  jamais supprimée côté IA) et commander, **dans ce projet-là**, l'annotation des deux traces de
  spec sur son `main`.
- L'**essai réel** avec une vraie salve de délégations, et le réglage final du seuil de 90 s.
- Plus tard : le lot IdeaPad (Linux, environnement de bureau à préciser), le multi-machines,
  le lancement automatique à l'ouverture de session.

## Vision (à reporter dans `specs/PROJET.md`)

IakaAgentsMonitor répond à une seule question : **« qui travaille en ce moment, et pour qui ? »**.
C'est un petit moniteur de barre de menus qui montre, d'un coup d'œil, les agents Claude Code en
cours sur le poste — leur persona iakaframe (lettre à la couleur de sa pastille), le projet sur
lequel ils tournent, la tâche qu'on leur a confiée, qui a délégué à qui, et depuis quand. Il est
**autonome** (aucun service, aucun bus, aucun compte), **en lecture seule** sur les fichiers que
Claude Code écrit déjà, et complète iakaTokenCounter sans le recouvrir : l'un mesure ce que
coûtent les agents, l'autre montre ce qu'ils font. D'abord le Mac, ensuite l'IdeaPad, puis, si le
besoin s'en fait sentir, une vue réunissant plusieurs machines.

## Sources

- Code et instruction d'origine, lus en lecture seule le 2026-09-26 : `iakaTokenCounter`, branche
  `worktree-agent-a73ae73650ed5e328` (`1c9fd0e`), `src-tauri/src/agents.rs`,
  `specs/instructions/feature-agents-en-cours.md`, `specs/contrat-mqtt-conso.md`.
- Format des compagnons de sous-agents (`agentType`, `description`, `spawnDepth`,
  `parentAgentId`) : [princess-pi/wtft #137](https://github.com/princess-pi/wtft/issues/137),
  [kamp-us/phoenix #8404](https://github.com/kamp-us/phoenix/issues/8404),
  [meirka8/volute PR #82](https://github.com/meirka8/volute/pull/82),
  [Claude Code Docs — sous-agents](https://code.claude.com/docs/en/sub-agents).
- Tauri 2 (version 2.11.6 au 26/09 ; `set_title` : affiché sous macOS, sous Linux seulement avec
  une icône, non supporté sous Windows) :
  [docs.rs — tauri::tray::TrayIcon](https://docs.rs/tauri/latest/tauri/tray/struct.TrayIcon.html),
  [Tauri — System Tray](https://v2.tauri.app/learn/system-tray/).
- Clic sur l'icône non émis sous Linux (AppIndicator) :
  [tauri-apps/tray-icon #104](https://github.com/tauri-apps/tray-icon/issues/104),
  [tauri-apps/tray-icon README](https://github.com/tauri-apps/tray-icon).
