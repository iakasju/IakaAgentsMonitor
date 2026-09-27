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
quand. Agnostique du runner (`specs/instructions/agnostique-mqtt.md`) : les fiches d'agent
arrivent soit par le **bus MQTT local** (`iaka/agents/out/<runner>/<sessionId>/<agentId>`,
retenues, QoS 1 — tout runner qui publie sur ce topic apparaît), soit, **en repli strict** quand
le broker est absent, par l'**adaptateur Claude Code** (lecture des transcripts déjà écrits
localement dans `~/.claude/projects/`). L'app **s'abonne seulement** : aucun service, aucun port
en écoute côté app, sa seule connexion sortante va vers `127.0.0.1:1883` (broker **iakahub**
d'iakaTokenCounter, réutilisé sans le modifier). Complète iakaTokenCounter (qui mesure la conso)
sans le recouvrir. Cf. `specs/instructions/liste-des-agents.md`.

Stack : Tauri 2 (Rust) + TypeScript/Vite. Client MQTT (`rumqttc`) et parseur TOML (`toml`) sont les
deux seules dépendances Rust non-Tauri ; aucun nom de runner ni de fournisseur de modèle hors de
`src-tauri/config/labels.default.toml`, `src-tauri/src/sources/claude_code_files.rs` et
`publishers/claude-code/` (vérifiable par grep, cf. instruction). Icône de barre de menus fixe
(mode template macOS) + titre = effectif d'agents ; fenêtre = tableau des agents, indenté par
délégation, colonnes triables.

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
cargo test               # ~100 tests (fiche, config, roster, arbre, sources, integration bus)
cargo clippy --all-targets -- -D warnings
```

Variables d'environnement : `IAKAAM_MQTT_PORT` (port du broker, défaut 1883),
`IAKAAM_LIVENESS_SECS` (seuil de péremption, défaut 90).

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

## Dépôt git : Forgejo (iakabox)

Remote par défaut : **Forgejo LAN** `http://192.168.2.11:3001/<user>/<repo>.git`,
**HTTP + token** (SSH inutilisable). Token via `$env:FORGEJO_TOKEN` ou `.git/config`
local — **jamais commité**. Voir `iakabox-usage.html` (iakaframe) pour clone/push,
création de dépôt (API, description **ASCII**) et rotation de token.

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

- [ ] <!-- feature → specs/instructions/feature-xxx.md -->
