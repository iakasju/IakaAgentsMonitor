# Étude : piloter les agents depuis l'app (changer le modèle, pause / reprise / arrêt)

> Rédigé par le cadrage (P1, Gandalf) le 2026-09-27. Étude, **pas de code**.
> Statut : **PROPOSÉE, en attente d'arbitrage de Stéphane** (QC1 à QC9 ci-dessous).
> Demandes de Stéphane (27/09) :
> (a) « Étudie s'il est possible de changer en temps réel le modèle d'un agent. »
> (b) « Il manque le statut en bouton (vivant ou mort) : on pourrait mettre running, paused, avec
> la possibilité de changer l'état au travers du MQTT. »
> Conclusion courte : **(a) impossible en temps réel** pour un agent déjà lancé ; faisable pour les
> **prochaines délégations** d'un persona (lot séparé, optionnel). **(b) faisable**, avec des
> limites nettes (la pause ne prend effet qu'**entre deux appels d'outil**) : proposée en
> **amendement A1** de `agnostique-mqtt.md`.
> Dépend de : `agnostique-mqtt.md` (arbitrée Q1-Q9 = A le 26/09, **pas encore réalisée**) : le bus,
> la fiche d'agent et le publieur par hooks sont le socle de tout ce qui suit.

---

## Problème

1. Stéphane voudrait basculer un agent sur un autre modèle sans attendre la fin de sa tâche (par
   exemple passer un agent coûteux sur un modèle moins cher, ou l'inverse quand il bloque).
2. La colonne « Statut » a été remplacée par la barre « Reste » (`temps-restant.md`, Q3-A) : l'app
   ne dit plus si un agent **tourne ou est suspendu**, et elle ne permet **aucune action**. Il veut
   un bouton d'état (running / paused) cliquable, relayé par le bus MQTT, pour suspendre ou relancer
   un agent, et pouvoir l'arrêter.

## Ce qui existe (vérifié en lecture le 2026-09-27)

| Élément | Constat |
|---|---|
| Hooks de Stéphane (`~/.claude/settings.json`, lecture seule) | `Stop`, `SubagentStop`, `UserPromptSubmit`, `PreToolUse` matcher `Task` (delegation-guard) et `Edit\|Write\|Bash\|NotebookEdit` (perimeter-guard, mode `warn`), `PostToolUse` `Task` et `TodoWrite`. **Aucun `PreToolUse` sur tous les outils, aucun `PreModelSwitch`**. `"model": "opus[1m]"` pour la session principale. |
| Client MQTT de hook existant | `~/.claude/delegation-guard.mjs` l. 261-300 : CONNECT + PUBLISH seulement (`node:net`). **Pas de SUBSCRIBE** : un hook qui lit un état retenu demande un client un peu plus complet. |
| Broker iakahub (`iakaTokenCounter/iakahub/rumqttd.toml`) | un seul listener **MQTT v4 en TCP**, `127.0.0.1`, **anonyme** (aucune section `auth`), **pas de WebSocket**, pas de v5. |
| App | `src/render.ts` l. 220-229 : colonnes Agent · Projet · Runner · Modèle · Tâche · Démarré · Actif · Reste ; plus de colonne d'état ; aucun contrôle. `estimate.rs` présent (lot `temps-restant` livré ou en cours). |
| Contrat v1 (`agnostique-mqtt.md`) | `iaka/agents/in/#` **réservé**, non utilisé ; `state` ∈ `running`, `idle`, valeur inconnue ⇒ traitée comme `running` (donc ajouter des valeurs est compatible, sans changer `v`). Q4-A : l'app est un **pur abonné**. |

## Faits externes vérifiés (doc officielle Claude Code, 27/09)

**Modèle**

- **Un sous-agent lancé garde son modèle** : le modèle est résolu **au lancement**, dans l'ordre
  (1) paramètre `model` de l'appel à l'outil Agent, (2) champ `model` du fichier d'agent
  (`inherit` = modèle de la session), (3) variable `CLAUDE_CODE_SUBAGENT_MODEL`, (4) modèle de la
  session. Le paramètre `model` **reste appliqué** quand le sous-agent est repris. Aucune commande
  documentée ne change le modèle d'un sous-agent en cours. (Sub-agents.)
- **Fichiers d'agents rechargés à chaud** : `~/.claude/agents/` et `.claude/agents/` sont surveillés ;
  une modification est prise en compte « en quelques secondes » et **la délégation suivante** utilise
  la nouvelle définition, sans redémarrage. (Sub-agents.)
- **Session principale** : `/model` **dans** la session ; de l'extérieur, seulement par le
  **SDK Agent** (`setModel()`, pour une session pilotée par le SDK, pas le terminal de Stéphane) ou
  par **Remote Control** (claude.ai / app mobile, via le relais du compte, pas d'interface locale).
  Changer `model` dans `settings.json` ou une variable d'environnement **n'agit qu'au lancement
  suivant**. (Model configuration.)
- **Les hooks ne peuvent pas changer de modèle** : `PreModelSwitch` peut seulement **bloquer ou
  faire confirmer** un changement demandé par l'utilisateur ou un client ; `PostModelSwitch` observe.
  (Hooks.)
- **Un hook `PreToolUse` peut réécrire les paramètres d'un outil** : `updatedInput` remplace
  `tool_input` avant exécution, **uniquement avec `permissionDecision: "allow"`**. (Hooks.) C'est le
  levier déterministe pour imposer le `model` d'une délégation.
- Valeurs de `model` admises : alias `sonnet`, `opus`, `haiku`, `fable`, ou identifiant complet.
  **Piège** : un alias de la même famille que la session (`opus` avec une session Opus) prend le
  modèle **exact** de la session, suffixe `[1m]` compris. (Sub-agents.)

**Pause / arrêt**

- `PreToolUse` reçoit `session_id`, `tool_name`, `tool_input`, et **`agent_id` / `agent_type`
  quand il se déclenche dans un sous-agent** : on sait donc **quel agent** va appeler un outil.
- **Délai d'un hook `command`** : 600 s par défaut, **aucun maximum documenté**. **Un hook qui
  dépasse son délai ne bloque pas** : l'appel d'outil continue (« don't count on a stalled hook to act
  as a gate »). Donc **fail-open** par construction.
- `permissionDecision: "deny"` + `permissionDecisionReason` : l'appel est refusé, le motif est
  **montré au modèle**.
- `"continue": false` + `stopReason` : **dans un sous-agent, arrête ce seul sous-agent et rend la
  main au parent** ; dans la session principale, arrête la conversation en cours.
- Les modifications de hooks dans les fichiers de réglages sont **reprises à chaud** par le
  surveillant de fichiers.
- Rien dans la doc ne permet à un hook d'**interrompre une génération en cours** : les hooks ne
  s'exécutent qu'aux événements (avant/après outil, fin de tour…). **Conclusion : la pause
  n'agit qu'au prochain appel d'outil** (fait déduit d'une absence, pas d'une phrase explicite).

**Broker**

- `rumqttd` : authentification par **couples identifiant / mot de passe** dans la configuration
  (`connections.auth`) ou par fonction externe ; **aucune autorisation par topic (pas d'ACL)**.
  (Configuration rumqttd.)

**Autres runners (non vérifié en détail, à confirmer au lot Codex)**

- Codex CLI : `/model` en cours de session ; un client « app-server » peut changer le modèle et
  l'effort de raisonnement. Mécanisme de pilotage externe **supposé**, non vérifié au niveau du
  protocole.
- Ollama : le modèle est un paramètre **de chaque requête** de l'API ; un runner maison
  (iakaframe/Ollama) peut donc honorer un changement **à sa requête suivante**. Sans vérification
  supplémentaire aujourd'hui.

## Réponses

### (a) Changer le modèle d'un agent « en temps réel »

| Cible | Faisable ? | Comment |
|---|---|---|
| Sous-agent **en cours** | **Non.** | Modèle figé au lancement, conservé à la reprise. Seul contournement : l'arrêter et le relancer sur un autre modèle — il **perd son contexte** de travail. |
| Session principale, de l'extérieur | **Non, pas depuis le bus.** | `/model` au clavier ; SDK `setModel()` ou Remote Control ne sont pas des voies locales utilisables par l'app. Aucun hook ne peut déclencher le changement. |
| **Prochaines délégations** d'un persona | **Oui.** | Deux voies, ci-dessous. |

Deux voies pour les délégations suivantes :

1. **Réécriture de l'appel par hook (recommandée si Stéphane veut la fonction).** Un hook
   `PreToolUse` sur l'outil de délégation (`Agent|Task`) lit sur le bus une **préférence de modèle
   par persona** (retenue, signée, voir sécurité) et, si elle existe pour le `subagent_type` visé,
   renvoie `allow` + `updatedInput` avec `model` remplacé. **Déterministe** : ne dépend pas de la
   bonne volonté du délégant. Prend effet à la délégation suivante, pas avant.
2. **Réécriture du champ `model` des fichiers d'agents** (`~/.claude/agents/<persona>.md`,
   générés par iakaframe). Rechargés à chaud, mais : c'est **la configuration globale de
   Stéphane** (aucun agent ni l'app n'y écrit) ; iakaframe les régénère et écraserait le réglage ;
   et le paramètre `model` d'un appel passe **devant** le fichier. **Déconseillé.**
3. (Écartée) Consigne lue sur le bus par le délégant qui choisirait lui-même `model` : repose sur
   le modèle qui suit une consigne, non déterministe, et coûte du contexte à chaque tour.

**Coût et limites de la voie 1.** Répondre `allow` court-circuite le parcours d'autorisation
normal **pour cet appel à l'outil Agent** (en pratique l'outil Agent ne demande pas d'autorisation,
à vérifier en recette). Un changement de modèle perd le cache de prompt (premier tour plus cher).
Piège de l'alias de même famille (voir plus haut) : l'app doit proposer des **identifiants complets**
lus dans `labels.toml`, pas des alias. **Agnosticisme** : sur le bus, `model` est une chaîne
opaque ; chaque adaptateur l'honore à sa façon (hook pour Claude Code, requête suivante pour
Ollama, à étudier pour Codex).

### (b) Bouton d'état et commande pause / reprise / arrêt

**Faisable**, par un hook `PreToolUse` sur **tous les outils** (« contrôleur ») :

- Il lit l'**état désiré** de son agent, retenu sur `iaka/agents/in/<runner>/<sessionId>/<agentId>`
  (`agentId` = `agent_id` du hook, `main` hors sous-agent).
- **Rien** (topic vide) ⇒ il sort aussitôt sans décision : l'outil s'exécute normalement.
- **`pause`** ⇒ il **garde la main** (le hook ne répond pas) en restant abonné, publie la fiche avec
  `state: "paused"` et rafraîchit `lastSeenAt` toutes les 20 s (sinon l'agent disparaîtrait de l'app
  après 90 s de pause). Dès que la commande est effacée (reprise) ⇒ fiche `running`, sortie sans
  décision, l'outil s'exécute.
- **`stop`** ⇒ fiche `state: "stopping"`, puis réponse `{"continue": false, "stopReason": "Arrêté
  par l'opérateur depuis IakaAgentsMonitor. Ne pas relancer sans consigne de l'utilisateur."}`.
  Pour un sous-agent, seul lui s'arrête et le parent reprend la main ; pour la session principale,
  le tour en cours s'arrête.

**Limites honnêtes.**

- **Pas d'interruption en pleine génération** : un agent qui écrit une longue réponse ou réfléchit
  sans appeler d'outil ne s'arrête qu'à son **prochain appel d'outil**. Un agent qui ne rappelle plus
  d'outil (il termine sa réponse) **finit sa tâche** malgré la pause. L'app doit donc afficher
  « pause demandée » tant que la fiche ne dit pas `paused`.
- **Durée de pause bornée** par le délai du hook (fail-open au dépassement). On règle le hook à
  **3 600 s** et le contrôleur se rend lui-même à 3 540 s avec un **refus** motivé (« pause
  prolongée par l'opérateur, refais exactement le même appel ») : le modèle réessaie, le hook
  repart pour une heure. Risque : le modèle peut changer de plan au lieu de réessayer.
- **Broker absent ou perdu pendant la pause ⇒ reprise** : jamais de blocage par défaut. De même,
  clé de signature absente ⇒ contrôleur inactif.
- **Coût sur chaque appel d'outil, dans toutes les sessions** : démarrage de Node + connexion +
  abonnement + courte attente du message retenu. Budget dur **250 ms**, au-delà sortie sans
  décision. Ordre de grandeur **supposé** : 60 à 120 ms par appel (le perimeter-guard paie déjà le
  démarrage de Node sur Edit/Write/Bash). **À mesurer en premier** ; si trop lent, repli sur un
  fichier miroir (QC2-B).
- **Ordre de livraison des messages retenus par rapport au SUBACK dans rumqttd : non vérifié.**
  Le contrôleur attend 30 ms après l'accusé d'abonnement avant de conclure « rien » ; à mesurer.
- Session principale **en attente de l'utilisateur** : aucun appel d'outil, la pause n'a rien à
  suspendre ; elle s'appliquera au premier outil du tour suivant.
- Délégations de l'outil `Workflow` : comportement des hooks **inconnu** (même inconnue que le lot
  de base).
- Plusieurs hooks `PreToolUse` peuvent viser le même appel (perimeter-guard, delegation-guard) :
  leur exécution en parallèle et la priorité du refus sont **supposées**, à vérifier en recette.

**Côté app.** Une colonne **« État »** étroite, en tête de ligne, visible aussi en mode compact :
bouton `running` / `paused` (clic = bascule), `idle` pour le coordinateur entre deux tours
(cliquable : la pause s'appliquera au tour suivant), `stopping` (inactif). Arrêt par un petit
bouton secondaire avec **confirmation dans la ligne** (« Arrêter Gimli ? Arrêter / Annuler »).
Boutons **désactivés** quand l'app lit les fichiers (bus absent), avec infobulle. Une commande
restée sans effet 30 s ⇒ le bouton revient à l'état réel et l'infobulle dit « non pris en compte
(contrôleur absent ?) ».

## Sécurité : ce que le canal de commande change

Aujourd'hui (R7 du lot de base), un processus local peut **lire** les tâches et **publier de
fausses fiches** : désagréable, mais passif. Un canal de **commande** permet en plus de
**suspendre ou arrêter** des agents. La dette s'aggrave : il faut une mitigation.

| Option | Protège contre | Limites |
|---|---|---|
| Activer l'authentification de rumqttd | clients sans mot de passe | **pas d'ACL par topic** (vérifié) ; casserait les clients anonymes d'iakaTokenCounter ; iakahub est **hors périmètre** (autre projet). Écartée pour ce lot. |
| Jeton partagé en clair dans la commande | rien de sérieux | tout abonné à `iaka/agents/in/#` **lit le jeton** et peut le rejouer. Écartée. |
| **Commande signée HMAC-SHA256**, clé 32 octets dans un fichier `0600` | tout processus qui atteint le port 1883 **sans pouvoir lire le dossier personnel** de Stéphane : autres comptes du Mac, conteneurs Docker (**supposé** joignables vers la boucle locale de l'hôte par `host.docker.internal`, non vérifié), outils mal configurés | un processus **du même compte** peut lire la clé — mais il peut déjà tuer les agents ou modifier `~/.claude/` : la frontière de confiance du poste est le compte, pas le bus. **Recommandée.** |
| Commande acceptée seulement depuis l'app (fichier local écrit par l'app, lu par le hook, sans bus) | idem ci-dessus, et aucune surface réseau | ne passe **pas** par le bus : contraire à la demande et à l'agnosticisme (chaque runner lirait un fichier). Gardée en **repli** (QC2-B). |

Vérifié : broker lié à `127.0.0.1`, anonyme, TCP v4 **sans WebSocket** (une page web ne peut pas
parler à ce port) ; rumqttd sans ACL. Supposé : accès des conteneurs Docker Desktop à la boucle
locale de l'hôte ; absence d'autres clients sur le bus en dehors d'iakaTokenCounter et de l'app.

Détails de la signature (proposés, voir amendement) : message signé = topic, commande,
`issuedAt`, `nonce` ; clé créée par l'app au premier lancement dans
`~/.config/iaka-agents/control.key` (dossier `0700`, fichier `0600`) ; commande non signée ou mal
signée ⇒ **ignorée** et journalisée ; anti-rejeu : le contrôleur ignore une commande dont
`issuedAt` est antérieur à la dernière reprise qu'il a vue pour cet agent.

## Décision retenue (proposée, sous réserve de QC1 à QC9)

- **(b)** : amendement **A1** de `agnostique-mqtt.md` (commande pause / reprise / arrêt par le bus,
  contrôleur `PreToolUse`, commande signée, colonne « État »), livré **après** le lot de base, en
  sous-lot séparé.
- **(a)** : **pas de changement en temps réel** (impossible). La préférence de modèle par persona
  pour les délégations suivantes est **décrite ici mais pas cadrée en détail** : lot séparé, à
  lancer seulement si Stéphane le veut (QC8), une fois A1 livré (il réutilise le contrôleur, la
  signature et le client MQTT).

## Périmètre

- Inclus (dans cette étude) : l'analyse ci-dessus ; l'amendement A1 écrit dans
  `agnostique-mqtt.md` ; les questions fermées.
- Exclu : tout code ; toute modification de `~/.claude/` (hooks, agents) ; toute modification
  d'iakahub / iakaTokenCounter ; le cadrage détaillé de la préférence de modèle (QC8) ; l'adaptateur
  Codex ; l'interruption en pleine génération (impossible par hook).

## Étapes d'implémentation

Voir `agnostique-mqtt.md` § « Amendement A1 ». Pour (a), aucune tant que QC8 n'est pas tranchée.

## Fichiers concernés

- `specs/instructions/controle-agents-etude.md` — **nouveau** (cette étude).
- `specs/instructions/agnostique-mqtt.md` — **amendement A1** ajouté.

## Risques

- **RC1 — Pause qui n'agit pas** (agent sans appel d'outil) : affichage « pause demandée »,
  retour à l'état réel après 30 s.
- **RC2 — Ralentissement de toutes les sessions** par un hook sur chaque outil : budget 250 ms,
  mesure en premier ; repli QC2-B.
- **RC3 — Le parent relance un sous-agent arrêté** : motif d'arrêt explicite ; à observer en
  recette (comportement supposé).
- **RC4 — Pause longue qui se relâche** (délai du hook) : refus motivé à 3 540 s, réessai du modèle
  non garanti.
- **RC5 — Sécurité** : commande signée ; dette résiduelle (même compte, rejeu limité) à consigner
  dans `specs/PROJET.md`.
- **RC6 — Formats de hooks récents et peu documentés** (`agent_id` dans `PreToolUse`,
  `continue:false` dans un sous-agent) : confinés au contrôleur, premiers points de la recette.

## Critères d'acceptation (de l'étude)

- [ ] Chaque affirmation de faisabilité est rattachée à une source ou marquée « supposé ».
- [ ] L'amendement A1 existe dans `agnostique-mqtt.md`, avec ses propres critères testables.
- [ ] Les questions QC1 à QC9 ont chacune une recommandation.

## Questions fermées (arbitrage de Stéphane)

**QC1 — Livrer la commande pause / reprise / arrêt ?**
- **A (recommandé)** — Oui, en sous-lot A1 **après** le lot de base `agnostique-mqtt` (étapes 1 à 12
  validées au gate), pour ne pas retarder ce dernier.
- B — Oui, fondu dans le lot de base (un seul gate, lot plus long et plus risqué).
- C — Non : seulement l'affichage de l'état, sans commande.

**QC2 — Où le hook lit-il l'état désiré ?**
- **A (recommandé)** — **Sur le bus** à chaque appel d'outil (connexion courte, budget 250 ms),
  conforme à la demande et agnostique.
- B — Dans un **fichier local** écrit par l'app (`~/.config/iaka-agents/control/…`), le bus ne
  servant qu'à refléter l'état : plus rapide, mais hors bus. À prendre si la mesure de QC2-A
  dépasse le budget.

**QC3 — Protection du canal de commande ?**
- **A (recommandé)** — Commande **signée HMAC**, clé `0600` créée par l'app.
- B — Aucune, dette assumée et consignée (tout processus local peut suspendre les agents).
- C — Authentification rumqttd : touche iakahub (autre projet) et casse ses clients anonymes.

**QC4 — Portée de l'arrêt ?**
- **A (recommandé)** — Sous-agents **et** coordinateur, toujours avec confirmation.
- B — Sous-agents seulement (pour la session principale, Stéphane a déjà Échap au clavier).

**QC5 — Pause au-delà d'une heure ?**
- **A (recommandé)** — Délai du hook 3 600 s, puis refus motivé « refais le même appel » (la pause
  se prolonge si le modèle réessaie).
- B — Délai du hook très long (24 h) : pas de relâche, mais un hook suspendu une journée entière,
  comportement jamais testé.

**QC6 — Libellés du bouton ?**
- **A (recommandé)** — Tels que proposés : `running` / `paused` (+ `idle`, `stopping`).
- B — En français : « en cours » / « en pause » (+ « au repos », « arrêt… »).

**QC7 — Maquette avant le code ?**
- **A (recommandé)** — Non : colonne étroite dans le style V4, capture d'écran au gate.
- B — Oui, maquette de Loki d'abord (+0,25 j).

**QC8 — Préférence de modèle par persona pour les délégations suivantes ?**
- **A (recommandé)** — Pas maintenant : on livre A1, puis Stéphane décide s'il veut ce lot
  (cadrage séparé, ~1 j).
- B — Oui, cadrer tout de suite (voie 1 : hook `PreToolUse` sur l'outil Agent + `updatedInput`,
  préférence par session).
- C — Jamais : on accepte que le modèle se change seulement au clavier.

**QC9 — Où vit le contrôleur ?**
- **A (recommandé)** — Même dossier que le publieur (`publishers/claude-code/`), fichier séparé
  `iaka-agents-control.mjs`, client MQTT partagé ; copié par Stéphane dans `~/.claude/`.
- B — Fusionné dans `iaka-agents-publish.mjs` (un seul fichier, mais un script tantôt bloquant,
  tantôt asynchrone).

## Estimation (ordre de grandeur, révisable)

- **A1 (b) : ~2,25 jours-homme** (fourchette 1,75 à 3,25) — contrat `in` + signature 0,25 ;
  contrôleur (client avec abonnement, pause, arrêt, fail-open) + tests 0,75 ; côté app (publication,
  clé, nettoyage des commandes) 0,4 ; colonne « État » + confirmation + états intermédiaires 0,4 ;
  intégration + recette 0,45.
- **(a) voie 1, si QC8-B : ~1 jour-homme** en plus.
- **Complexité / risque : moyen à élevé** — le hook bloquant est une nouveauté, et touche
  **toutes** les sessions de Stéphane.
- **Inconnues qui peuvent faire glisser** : latence réelle par appel d'outil (+0,5 j si repli
  QC2-B) ; ordre retenu/SUBACK dans rumqttd ; comportement réel de `continue:false` dans un
  sous-agent et réaction du parent ; hooks et outil `Workflow` ; interaction avec les autres hooks
  `PreToolUse`.

## Ce qui appartient à Stéphane

- Trancher **QC1 à QC9** (ou « go » sur les recommandations).
- **Installer le contrôleur** : copier le script dans `~/.claude/` et **ajouter** l'entrée
  `PreToolUse` (tous outils, `timeout: 3600`) à `~/.claude/settings.json`, à côté des hooks
  existants. Cette entrée ralentit et peut suspendre **toutes** ses sessions : aucun agent ne
  l'installe à sa place. Pour la retirer : supprimer l'entrée (reprise à chaud).
- Accepter et consigner la **dette de sécurité résiduelle** (même compte, rejeu limité).
- Garder la tray iakaTokenCounter lancée (elle porte le broker) pendant l'essai.

## Sources

- Lu le 2026-09-27 (lecture seule) : `~/.claude/settings.json` ; `~/.claude/delegation-guard.mjs`
  (l. 261-300) ; `iakaTokenCounter/iakahub/rumqttd.toml` ; `IakaAgentsMonitor/src/render.ts`
  (l. 204-229) ; `specs/instructions/agnostique-mqtt.md`, `temps-restant.md`.
- [Claude Code Docs — Hooks](https://code.claude.com/docs/en/hooks) : champs de `PreToolUse`,
  `updatedInput`, `deny`, `continue`/`stopReason` (sous-agent vs session), délais, fail-open au
  dépassement, `PreModelSwitch`/`PostModelSwitch`, rechargement à chaud.
- [Claude Code Docs — Subagents](https://code.claude.com/docs/en/sub-agents) : ordre de résolution
  du modèle, alias de même famille, `CLAUDE_CODE_SUBAGENT_MODEL(_FORCE)`, rechargement des
  fichiers d'agents.
- [Claude Code Docs — Model configuration](https://code.claude.com/docs/en/model-config) :
  `/model`, SDK `setModel()`, Remote Control, réglages pris au lancement seulement.
- [Claude Code Docs — Remote Control](https://code.claude.com/docs/en/remote-control) : relais via
  claude.ai / app mobile.
- [rumqttd — Configuration](https://rumqtt.bytebeam.io/docs/rumqttd/Configuration/) et
  [exemple external_auth](https://github.com/bytebeamio/rumqtt/blob/main/rumqttd/examples/external_auth.rs) :
  identifiant / mot de passe, pas d'ACL par topic.
- Codex (non vérifié au niveau protocole) :
  [Codex CLI — changement de modèle en session](https://codex.danielvaughan.com/2026/04/13/codex-cli-dynamic-session-control-mid-session-switching/),
  [openai/codex #1029](https://github.com/openai/codex/issues/1029).
