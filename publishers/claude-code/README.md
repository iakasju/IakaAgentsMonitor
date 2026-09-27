# Publieur Claude Code — contrat MQTT v1 IakaAgentsMonitor

Publie/efface des fiches d'agent sur le bus MQTT local (`iaka/agents/out/claude-code/<sessionId>/<agentId>`,
retenues, QoS 1), conformément à `specs/instructions/agnostique-mqtt.md`. Node ≥ 18, **aucune
dépendance**. Toujours code de sortie 0, jamais de sortie sur stdout : ce script ne ralentit et ne
bloque jamais une session Claude Code, même si le broker est absent ou lent (budget total 1,5 s).

## Ce que ce script fait

- Lit `hook_event_name` (et le reste du payload) sur `stdin`.
- Construit une fiche (`v`, `runner: "claude-code"`, `agentId`, `parentId`, `sessionId`, `model`,
  `persona`, `task`, `estimate`, horodatages, `state`) et la publie, ou publie un payload vide
  pour effacer une fiche retenue (fin d'agent).
- Événements gérés : `SessionStart`, `UserPromptSubmit`, `Stop`, `SessionEnd`, `SubagentStart`,
  `SubagentStop`, `PostToolUse`.
- Événements `SubagentStart`/`SubagentStop` à `agent_type` vide (fantômes connus, issue
  [#91910](https://github.com/anthropics/claude-code/issues/91910)) : ignorés **avant toute
  connexion**.
- Limite le débit des battements (`PostToolUse`) à une publication par agent toutes les 20 s.
- État local (fiche courante, date de dernière publication) dans
  `~/.cache/iaka-agents/<sessionId>/<agentId>.json`, effacé au complet à `SessionEnd`.

## Ce que ce script NE fait PAS

- Il ne lit ni n'écrit **rien** sous `~/.claude/` — c'est la configuration globale de Stéphane,
  hors du périmètre de cet agent.
- Il ne modifie ni iakaTokenCounter, ni iakahub, ni iakaframe.
- Il n'implémente **pas** le contrôle pause/reprise/arrêt (amendement A1, sous-lot séparé, non
  livré ici — les boutons « État » de l'app restent désactivés tant que ce sous-lot n'est pas
  arbitré et livré).

## Installation (geste de Stéphane — un agent ne le fait jamais à sa place)

1. Copier le script dans `~/.claude/` :

   ```bash
   cp publishers/claude-code/iaka-agents-publish.mjs ~/.claude/iaka-agents-publish.mjs
   ```

2. Ajouter l'extrait suivant à `~/.claude/settings.json`, **à côté** des entrées de hooks déjà
   présentes (`Stop`, `SubagentStop`, `UserPromptSubmit`, `PreToolUse`/`PostToolUse` existants) —
   **jamais à leur place**. Chaque bloc `hooks` d'un même événement peut porter plusieurs
   commandes : ajouter une entrée supplémentaire dans le tableau `"hooks"` de l'événement
   concerné, ou créer l'entrée si l'événement n'a encore aucun hook.

   ```json
   {
     "hooks": {
       "SessionStart": [
         {
           "matcher": "*",
           "hooks": [
             { "type": "command", "command": "node ~/.claude/iaka-agents-publish.mjs", "async": true }
           ]
         }
       ],
       "UserPromptSubmit": [
         {
           "matcher": "*",
           "hooks": [
             { "type": "command", "command": "node ~/.claude/iaka-agents-publish.mjs", "async": true }
           ]
         }
       ],
       "Stop": [
         {
           "matcher": "*",
           "hooks": [
             { "type": "command", "command": "node ~/.claude/iaka-agents-publish.mjs", "async": true }
           ]
         }
       ],
       "SessionEnd": [
         {
           "matcher": "*",
           "hooks": [
             { "type": "command", "command": "node ~/.claude/iaka-agents-publish.mjs", "async": false }
           ]
         }
       ],
       "SubagentStart": [
         {
           "matcher": "*",
           "hooks": [
             { "type": "command", "command": "node ~/.claude/iaka-agents-publish.mjs", "async": true }
           ]
         }
       ],
       "SubagentStop": [
         {
           "matcher": "*",
           "hooks": [
             { "type": "command", "command": "node ~/.claude/iaka-agents-publish.mjs", "async": true }
           ]
         }
       ],
       "PostToolUse": [
         {
           "matcher": "*",
           "hooks": [
             { "type": "command", "command": "node ~/.claude/iaka-agents-publish.mjs", "async": true }
           ]
         }
       ]
     }
   }
   ```

   > `SessionEnd` est le SEUL événement `async: false` (DA-M3) : la suppression de la fiche du
   > coordinateur doit partir avant la fermeture du processus. Tous les autres sont `async: true`
   > pour ne jamais ralentir la session.

3. Garder la tray **iakaTokenCounter** lancée (elle porte le broker `iakahub`,
   `127.0.0.1:1883`) pendant l'essai — sinon l'app IakaAgentsMonitor retombe automatiquement sur
   la lecture des fichiers (repli, Q3-A), sans erreur.

## Variables d'environnement

- `IAKAAM_MQTT_PORT` — port du broker (défaut `1883`), à faire correspondre à celui d'iakahub si
  surchargé.

## Tests

```bash
npm run test:publisher
```

`node --test`, avec un faux serveur MQTT en `node:net` (`test/fake-broker.mjs`) : aucune
dépendance, aucun broker réel nécessaire pour la suite de tests principale (un seul test lance un
vrai sous-processus pour vérifier le comportement broker absent).
