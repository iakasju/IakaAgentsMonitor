# Instruction : finition du lot « Ce poste » (observations du gate Legolas)

> Cadrée par **Gandalf** (P1) le **2026-10-01**. Exécution : **Gimli** (P2), **sans urgence**, sur
> `feat/fusion-flotte` après `ef3ed43`. Parent : `specs/instructions/ce-poste-vue-locale-lisibilite.md`
> (gate Legolas PASS, validé à l'écran par le décideur). Quatre observations **non bloquantes** du gate.

## 1. Problème

Le lot « Ce poste » est livré et validé, mais le gate a relevé une garde de chemin trop permissive, des
commentaires devenus faux, une règle protégée par un seul vecteur, et des tâches de coordinateur qui
affichent du XML brut.

## 2. Faits (relevés par Legolas, re-lus par le cadrage pour le n° 1)

| Id | Fait | Où |
|---|---|---|
| **F1** | `cards_for_session` n'écarte que `/` et `\` dans `session_id` (`session_id.contains(['/', '\\'])`) : `..` ou `.` passent, alors que le contrat n'admet pour un segment que `[A-Za-z0-9._-]{1,128}` | `src-tauri/src/sources/claude_code_files.rs:281-286` · `specs/contrat-bus-iaka.md` § 1 |
| **F2** | Commentaires périmés : anciens ratios 4,85 / 4,84 / 4,78:1 dans `styles.css` ; mention d'un `max-height` qui n'existe plus dans `render.ts` | `src/styles.css` ~l. 58-68 et ~143 · `src/render.ts` ~l. 720 |
| **F3** | L'épinglage des sessions vivantes (une session vivante ne périme jamais) n'est protégé **que** par le vecteur `local-sessions.json` ; aucun test unitaire de la bibliothèque | `src-tauri/src/sources/local_sessions.rs` |
| **F4** | Deux coordinateurs ont pour tâche du **texte XML brut** (`<agent-message …>`, `<task-notification>`), issu du premier message utilisateur lu par le publieur de hooks (DA-M6) — contenu injecté par le système, pas une consigne humaine. Sur le central, la tâche est déjà **masquée par défaut** | recette du 2026-10-01 · `publishers/claude-code/iaka-agents-publish.mjs` (tâche de `main`) |

## 3. Décisions

- **`D1` — Liste blanche, pas liste noire** : `cards_for_session` refuse tout `session_id` qui ne satisfait pas
  `^[A-Za-z0-9._-]{1,128}$` **ou** qui vaut `.` / `..` — même règle que les segments du contrat.
- **`D2`** — Commentaires corrigés pour dire ce que fait le code (ratios actuels, relevés par
  `npm run test:contrast` ; plus de mention de `max-height`).
- **`D3`** — Un test unitaire Rust dédié : session vivante inactive depuis 3 h ⇒ présente ; même session, pid mort
  ⇒ absente (le vecteur reste, le test double la garde).
- **`D4` — La tâche `main` ignore les messages injectés** : dans le **publieur**, le premier message utilisateur
  qui **commence par une balise** (`<` suivi d'un nom de balise) est **sauté** ; on prend le premier message
  humain suivant, sinon la tâche vaut `null` (affichée « — »). **Même règle** dans la ligne synthétique de
  l'app (`D2` du lot parent), qui lit `name` : si `name` commence par une balise, `null`. Aucun nettoyage de
  balise « au milieu » : on écarte, on ne réécrit pas.

## 4. Périmètre

- **Inclus** : `claude_code_files.rs` (`D1`), `styles.css` et `render.ts` (commentaires, `D2`),
  `local_sessions.rs` (tests, `D3`), `publishers/claude-code/iaka-agents-publish.mjs` et sa suite de tests,
  `local_view.rs` (`D4`).
- **Exclu** : toute autre modification de comportement ; les fiches déjà publiées (elles se renouvellent au
  battement) ; `~/.claude/` (le publieur installé est **recopié par le décideur**, cf. `L-INSTALL`).

## 5. Étapes (Gimli)

1. Tests d'abord, **vus rouges** : `session_id` = `..`, `.`, `a b`, 129 caractères ⇒ aucune lecture ; tâche `main`
   dont le premier message est `<task-notification>…` ⇒ message humain suivant, ou `null`.
2. `D1`, `D3`, `D4` ; puis `D2`.
3. `cargo test`, `clippy -D warnings`, `npm run test:publisher`, `npm run test:styles`, `npm run typecheck`.
4. Signaler au décideur qu'il doit **recopier** le publieur dans `~/.claude/` pour que `D4` agisse en réel.

## 6. Critères d'acceptation

- [ ] **CA-1** — Les tests de l'étape 1 vus rouges puis verts (trace `git log`).
- [ ] **CA-2** — **Contre-essai** : revenir à `contains(['/', '\\'])` fait rougir le cas `..`.
- [ ] **CA-3** — `grep -nE '4,8[45]|4,78|max-height' src/styles.css src/render.ts` ne renvoie plus de commentaire
      périmé (seules des valeurs réellement employées subsistent).
- [ ] **CA-4** — Test Rust de l'épinglage des sessions vivantes présent et vert ; le supprimer n'est pas couvert par
      le seul vecteur (les deux coexistent).
- [ ] **CA-5** — En réel, après recopie du publieur par le décideur : plus aucune tâche de coordinateur commençant
      par `<` dans « Ce poste ».
- [ ] **CA-6** — Aucun autre fichier modifié (`git diff --stat` limité au § 4).

## 7. Estimation

**≈ 0,3 j-h** (garde 0,05 · commentaires 0,05 · test 0,05 · tâche `main` publieur + app 0,15). Complexité et
risque **faibles**. *Révisable.*
