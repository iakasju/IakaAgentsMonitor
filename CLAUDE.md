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

Agnostique du runner (`specs/instructions/agnostique-mqtt.md`) : les fiches d'agent arrivent soit par le
**bus MQTT local**, soit, **en repli strict** quand le broker local est absent, par l'**adaptateur Claude
Code** (lecture des transcripts dans `~/.claude/projects/`). L'app **s'abonne seulement** : aucun service,
aucun port en écoute ; connexions sortantes : `127.0.0.1:1883` (iakahub d'iakaTokenCounter, réutilisé sans
le modifier) et, **seulement si `~/.iakafleet/.env` porte des identifiants**, le broker central en **TLS**
(`mqtts://mqtt.naonedge.com:8883` par défaut, `IAKAFLEET_MQTT_URL`), abonnement `iaka/+/out/#`, jamais `in/`.
**Sans configuration le Monitor marche seul, rien ne change.** Central injoignable ⇒ la zone « Autres
machines » dit « broker injoignable : <motif> », jamais une liste vide. Aucun ordre n'est jamais exécuté
comme un shell (`fleet/` n'importe pas `child_process`). Complète iakaTokenCounter sans le recouvrir.

Stack : Tauri 2 (Rust) + TypeScript/Vite ; `fleet/` : Node ≥ 20, JS sans build, dépendance `mqtt` (épinglée).
Client MQTT (`rumqttc`, TLS rustls déjà inclus) et parseur TOML (`toml`) sont les deux seules dépendances
Rust non-Tauri ; aucun nom de runner ni de fournisseur de modèle hors de
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

cd src-tauri
cargo test               # ~130 tests (fiche, config, roster, arbre, sources, bus, distante, vecteurs)
cargo clippy --all-targets -- -D warnings

cd fleet                 # démon de machine + CLI (ex-iakaFleet)
npm ci && npm test       # node --test, sans réseau ni broker
npm run lint
node bin/iakafleet.js ls|watch [--json]     # CLI : machines, sessions, agents
node bin/iakafleet-agent.js                 # démon (en pratique lancé par un service : fleet/deploy/)
```

Variables d'environnement de l'app : `IAKAAM_MQTT_PORT` (port du broker local, défaut 1883),
`IAKAAM_LIVENESS_SECS` (seuil de péremption, défaut 90). Central / nœud / confidentialité
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
- [ ] Reste de la fusion : `M4` (TLS `rumqttc` vers le VPS, après le Lot 0 amendé), lot `L-INSTALL`
      (démon + hooks, Mac puis PC — geste du décideur), recette `CA-R*`, README de renvoi dans iakaFleet
      puis archivage (décideur), état des lieux v0.3.0.
- [ ] Ordres (`iaka/<node>/in/…`, converge A1 + Lot 6 BACKLOG + ordres iakaFleet) — instruction à écrire.
