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

Moniteur de barre de menus macOS : d'un coup d'œil, qui travaille en ce moment (agents en cours,
tout runner), sur quel projet, quelle tâche leur a été confiée, qui a délégué à qui, et depuis
quand — **sur ce poste** (« Ce poste ») **et, si un broker central est configuré, sur les autres
machines** (« Autres machines », lecture seule). Décision du 2026-09-30 : iakaFleet a fusionné ici ;
le Monitor est **le produit**, en **trois pièces** : l'**app** (Tauri, `src-tauri/`, `src/`), le
**publieur de hooks** (`publishers/claude-code/`) et le **démon de machine + CLI** (`fleet/`, ex-iakaFleet,
sans GUI, un par machine). Instruction : `specs/instructions/fusion-flotte-vue-distante.md`.

**Convention de bus unique** `iaka/<node>/{out,in}/…` (`specs/contrat-bus-iaka.md`, contrat v2) : le publieur
écrit les fiches sur le bus local `iaka/<node>/out/agents/<runner>/<sessionId>/<agentId>` (retenues, QoS 1) ;
le démon relaie fiches, présence et sessions vers le central sous le même arbre (`sid` haché sur 12 hex,
`cwd` ramené au home, **texte de tâche masqué par défaut**, `IAKA_SHOW_TASKS=1` pour le publier ;
`IAKA_MASK_NAMES=1` masque les noms de session). `<node>` = `IAKA_NODE` sinon nom d'hôte normalisé
(mêmes règles dans les trois langages, prouvées par `contract-vectors/`). Vecteurs partagés exécutés par
`fleet/` (`node --test`), le publieur et l'app (`cargo test`, `src-tauri/tests/vectors.rs`).

**« Ce poste » lu sur le disque** (`specs/instructions/ce-poste-vue-locale-lisibilite.md`, D1-D4 ; remplace
la bascule stricte bus-ou-fichiers Q3-A d'`agnostique-mqtt.md`) : l'app lit **elle-même** les sources du démon,
sans broker ni démon — `~/.claude/sessions/<pid>.json` (vivacité `kill(pid,0)`, crate `libc` en dépendance
directe, **Unix seulement**) et `~/.cache/iaka-agents/<sessionId>/<agentId>.json` (`src-tauri/src/sources/local_sessions.rs`,
portage de `fleet/src/agent/sessions.js`/`cards.js`, vecteurs `contract-vectors/local-sessions.json`). **La session
fait foi** : pid vivant ⇒ affichée toujours ; coordinateur = fiche `main` sinon ligne synthétique (nom de session,
status, dépôt du `cwd`) ; sous-agents = fiches du disque **unies** à celles du bus local s'il est joint
(dédoublonnage, la plus récente gagne) ; fiche orpheline fraîche (90 s) ⇒ « agents non rattachés » ; session sans
aucune fiche ⇒ **repli par transcripts pour CETTE session seule** (`claude_code_files::cards_for_session`) — le
parcours complet de `~/.claude/projects` n'est plus appelé (il coûtait ~440 ms/tick en release, cause des pics CPU).
Les fiches d'un runner autre que Claude Code (bus local) restent leurs propres sessions. Titre
`Ce poste — <node> · N session(s) · M agent(s)` + `démon : <libellé>` + phrase d'état (`src-tauri/src/local_view.rs`).
L'app **s'abonne seulement** : aucun service,
aucun port en écoute ; connexions sortantes : `127.0.0.1:1883` (iakahub d'iakaTokenCounter, réutilisé sans
le modifier) et, **seulement si `~/.iakafleet/.env` porte des identifiants**, le broker central en **TLS**
(`mqtts://mqtt.naonedge.com:8883` par défaut, `IAKAFLEET_MQTT_URL`), abonnement `iaka/+/out/#`, jamais `in/`.
**Sans configuration le Monitor marche seul, rien ne change.** Central injoignable ⇒ la zone « Autres
machines » dit « broker injoignable : <motif> », jamais une liste vide. Aucun ordre n'est jamais exécuté
comme un shell (`fleet/` n'importe pas `child_process` : garde statique par liste blanche de modules `G-EXEC` **et** démon lancé
sous `node --permission` par les gabarits de service, sans `--allow-child-process`, `G-EXEC-2`). Complète iakaTokenCounter sans le recouvrir.

Stack : Tauri 2 (Rust) + TypeScript/Vite ; `fleet/` : Node ≥ 22.14 (modèle de permissions), JS sans build, dépendance `mqtt` (épinglée).
Client MQTT (`rumqttc`, TLS rustls déjà inclus), parseur TOML (`toml`) et `libc` (vivacité des pid) sont les
seules dépendances Rust non-Tauri ; aucun nom de runner ni de fournisseur de modèle hors de
`src-tauri/config/labels.default.toml`, `src-tauri/src/sources/claude_code_files.rs` et
`publishers/claude-code/` (vérifiable par grep, cf. instruction). Icône de barre de menus fixe
(mode template macOS) + titre = effectif d'agents ; fenêtre = tableau des agents, indenté par
délégation, colonnes triables, puis la section « Autres machines » (tableau V4 groupé par machine).

---

## Commandes à utiliser

```bash
npm run dev             # front seul (Vite, port 5180 — 5173 est pris par le worktree de
                         # dev d'iakaTokenCounter, ne jamais le réutiliser)
npm run tauri dev       # app complète (tray + fenêtre), rechargement à chaud
npm run build           # build front de prod
npm run tauri build     # bundle .app macOS
npm run typecheck       # tsc --noEmit
npm run test:publisher  # tests du publieur Claude Code (node --test, faux broker node:net)
npm run test:styles     # = test:contrast + test:fontsize (Node sans dépendance, ~0,1 s)
npm run test:contrast   # WCAG 2.x : registre scripts/contrast-registry.json (>= 4,5:1 tout texte, >= 5:1
                         # bandeaux, clair ET sombre), coloris conservés (seuls les jetons D8 changent, ± 5°)
npm run test:fontsize   # zéro font-size en dur hors bloc --sc-fs-*, plancher 14/13 px, crans 14/16/18 (défaut 16)
scripts/measure-cpu.sh  # protocole D7 sur un BUILD DE PUBLICATION (décideur) : 600 s, moyenne <= 1 %, pic <= 10 %

cd src-tauri
cargo test               # ~140 tests (fiche, config, roster, arbre, sources, bus, distante, vecteurs)
cargo clippy --all-targets -- -D warnings
cargo run --release --example tick_bench   # micro-banc : ancien tick (parcours complet) vs nouveau (O(sessions))

cd fleet                 # démon de machine + CLI (ex-iakaFleet)
npm ci && npm test       # node --test, aucune connexion externe (test/permission.test.js : démon sous --permission contre un faux broker sur 127.0.0.1)
npm run lint
node bin/iakafleet.js ls|watch [--json]     # CLI : machines, sessions, agents
node bin/iakafleet-agent.js                 # démon (en pratique lancé par un service : fleet/deploy/)
```

Variables d'environnement de l'app : `IAKAAM_MQTT_PORT` (port du broker local, défaut 1883),
`IAKAAM_LIVENESS_SECS` (péremption des **sous-agents**, défaut 90 ; une session vivante ne périme jamais),
`IAKAAM_TRACE=1` (durée de chaque tick sur stderr, budget p95 ≤ 50 ms), `IAKAFLEET_SESSIONS_DIR` (répertoire de
sessions, comme le démon). Taille du texte : trois crans 14 / 16 / 18 px dans la barre du haut (défaut 16,
mémorisé `iakaam-size`). Central / nœud / confidentialité
(`~/.iakafleet/.env`, cf. `fleet/docs/installation.md`) : `IAKA_NODE`, `IAKAFLEET_MQTT_URL`,
`IAKAFLEET_AGENT_USER`/`_PASS` ou `IAKAFLEET_CONSOLE_USER`/`_PASS`, `IAKA_MASK_NAMES`, `IAKA_SHOW_TASKS`.
Secrets : Vaultwarden `work/iakaFleet`, jamais dans le dépôt.

Aucun Makefile/script séparé : les commandes npm/cargo ci-dessus suffisent (app native de
bureau, pas de Docker — cf. `specs/instructions/liste-des-agents.md` § Périmètre).

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

Remote `origin` : **Forgejo du VPS NaonEdge** `https://git.naonedge.com/sjupin/IakaAgentsMonitor.git`,
**HTTPS + token** (l'ancien Forgejo LAN iakabox est mort ; le NAS reste un miroir secondaire sous le nom
`nas`). Token via `$FORGEJO_TOKEN` ou `.git/config` local — **jamais commité**. Création de dépôt par l'API
(description **ASCII**). Détail : skill `iakaframe-forgejo`.

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

- [x] Fusion iakaFleet + vue distante — `specs/instructions/fusion-flotte-vue-distante.md` : import `fleet/`,
      contrat v2, vecteurs, relais des fiches, CLI agents, source distante + zone « Autres machines »
      (livré en P2 hors broker réel ; **gate Legolas à passer**).
- [ ] Ce poste, vue locale et lisibilité — `specs/instructions/ce-poste-vue-locale-lisibilite.md` : source locale
      disque (D1-D3), titre + phrase d'état (D4), message vide (D5), direction A + crans 14/16/18 + coloris
      accentués (D6, D8), trace et protocole CPU (D7) — P2 remis au **gate Legolas** (aucun verdict) ; recette visuelle
      et budget CPU sur build de publication (`CA-12`, `CA-R1`) = décideur.
- [ ] Reste de la fusion : `M4` (TLS `rumqttc` vers le VPS, après le Lot 0 amendé), lot `L-INSTALL`
      (démon + hooks, Mac puis PC — geste du décideur), recette `CA-R*`, README de renvoi dans iakaFleet
      puis archivage (décideur), état des lieux v0.3.0.
- [ ] Ordres (`iaka/<node>/in/…`, converge A1 + Lot 6 BACKLOG + ordres iakaFleet) — instruction à écrire.
