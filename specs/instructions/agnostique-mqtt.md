# Instruction : IakaAgentsMonitor agnostique du runner, par le bus MQTT local

> **Q3-A remplacée par l'union `D2` de `ce-poste-vue-locale-lisibilite.md`** (2026-09-30) : la bascule stricte bus **ou** fichiers n'existe plus ; « Ce poste » lit les fichiers de session et de fiches du disque **et** le bus local s'il est joint, la détection par transcripts n'étant gardée qu'en repli **par session** (`D3`).

> **Note du 2026-09-30 (fusion iakaFleet — `specs/instructions/fusion-flotte-vue-distante.md`, étape 6)** :
> les topics de ce document sont **remplacés par le contrat v2** `specs/contrat-bus-iaka.md`.
> `iaka/agents/out/<runner>/<sessionId>/<agentId>` devient
> **`iaka/<node>/out/agents/<runner>/<sessionId>/<agentId>`** (le corps de la fiche est **inchangé**), et le
> topic cible de l'amendement A1 (**non codé**), `iaka/agents/in/<runner>/<sessionId>/<agentId>`, devient
> **`iaka/<node>/in/agents/<runner>/<sessionId>/<agentId>`** (point d'entrée unique des ordres, `D7` de la
> fusion). Les occurrences `iaka/agents/…` plus bas sont l'historique du contrat v1.

> Rédigé par le cadrage (P1, Gandalf) le 2026-09-26. Consommé par Gimli (P2).
> Statut : **VALIDÉE** (Q1 à Q9 = A), pas encore réalisée. L'amendement **A1** (27/09, commande
> pause / reprise / arrêt) est **PROPOSÉ, en attente d'arbitrage** (QC1 à QC9 de
> `controle-agents-etude.md`) : rien de A1 n'est à coder avant son « go ».
> **Arbitrage (26/09/2026)** : « go » de Stéphane sur les recommandations d'Odin — Q1 à Q9 = A (Q1 : réutiliser le broker iakahub d'iakaTokenCounter, repli fichiers quand il est absent ; passage à un Mosquitto permanent à reconsidérer quand le bus servira à plusieurs apps). Révisable en une phrase. Lancement après livraison de `temps-restant` (Q8).
> Décision du décideur (26/09, « go MQTT ») : rendre l'app **agnostique du runner** en passant par
> le **MQTT local**. Chaque runner publie une **fiche d'agent normalisée** ; l'app ne fait que
> **s'abonner** ; côté Claude Code, la publication se fait par **hooks** ; la lecture actuelle de
> `~/.claude/projects` devient l'**adaptateur Claude Code** et le **repli** quand le broker est
> absent ; Codex et Ollama : place prévue, non livrés.
> Dépend de : `liste-des-agents.md` (A1 à A3, livré) ; `temps-restant.md` (validé, **pas encore
> codé** : `src-tauri/src/estimate.rs` n'existe pas au 26/09). Ordre entre les deux : Q8.
> **Amende** `liste-des-agents.md` : le critère « `Cargo.toml` ne contient ni client MQTT » tombe
> (un client MQTT entre) ; « l'app n'ouvre aucun port en écoute » reste vrai.

---

## Problème

L'app sait aujourd'hui lire **un seul runner**, Claude Code, en fouillant ses fichiers internes
(transcripts, compagnons `.meta.json`), et elle porte en dur des noms qui ne la regardent pas :
familles de modèles (`opus|sonnet|haiku` dans `format_model_label`), runner (`"Claude Code"` dans
`render.ts`), roster figé de 9 personas (sans `charon`, pourtant dans l'équipe active). Chaque
nouveau runner (Codex, Ollama…) obligerait à recoder le cœur. Il faut inverser le sens : ce sont
les runners qui **annoncent** leurs agents sous une forme commune, et l'app qui **écoute**.

## Ce qui existe (vérifié en lecture le 2026-09-26)

| Élément | Constat |
|---|---|
| Broker local sur `127.0.0.1:1883` | **iakahub** d'iakaTokenCounter (`iakaTokenCounter/iakahub/`, Rust) : broker `rumqttd` **0.19** embarqué, **un seul listener MQTT v4 (3.1.1)**, **anonyme**, bind **boucle locale** uniquement (`src/broker.rs` l. 20-21, `rumqttd.toml`). Port surchargeable (`IAKATC_MQTT_PORT`), fail-fast si occupé. `max_payload_size = 20480` octets. |
| Cycle de vie de ce broker | **Pas un service système** : iakahub est lancé en **sidecar par l'app tray iakaTokenCounter** et **s'arrête avec elle** (`iakaTokenCounter/src-tauri/src/lib.rs` l. 48-58 et 108-115). Broker **en mémoire** : les messages retenus sont perdus à chaque redémarrage. |
| Homonyme trompeur | `~/Library/LaunchAgents/com.naonedge.iakahub.plist` lance **un autre projet**, `~/work/iakaHub` (Node, passerelle Discord) : **aucun MQTT** dedans (recherche `1883|mqtt|broker` vide hors `node_modules`). Ce n'est pas le broker. |
| Écoute effective à l'instant | **Non vérifiée** (le cadrage n'a pas d'accès shell) : à vérifier par `lsof -iTCP:1883 -sTCP:LISTEN` pendant que la tray iakaTokenCounter tourne. |
| Arbres de topics déjà présents | `iakatokencounter/#` (contrat `iakaTokenCounter/specs/contrat-mqtt-conso.md`, QoS 1, retenus, payload `{v,t}` en epoch s). Rien sous `iaka/`. |
| Limites du broker (lues dans le code `rumqttd` 0.19) | Message vide = **suppression du retenu** (`router/routing.rs` l. 1221-1222). **Last Will** géré, y compris retenu (l. 1086-1117, 1293-1295). **Expiration de message** : propriété **MQTT v5 seulement** (`router/logs.rs` l. 276-293) ⇒ **indisponible** sur le listener v4. **Rattrapage des retenus plafonné à 100 par filtre** pour un abonné QoS 1 (contrat conso § 4). |
| Hooks actuels de Stéphane (`~/.claude/settings.json`) | `Stop`, `SubagentStop` (identity-guard), `UserPromptSubmit` (identity-remind), `PreToolUse`/`PostToolUse` matcher `Task` (delegation-guard), `PreToolUse` Edit/Write/Bash (perimeter-guard), `PostToolUse` TodoWrite (plan-courante). **Aucun** `SubagentStart`, `SessionStart`, `SessionEnd`. |
| Publication MQTT sans dépendance, déjà écrite | `~/.claude/delegation-guard.mjs` l. 261-300 : client MQTT 3.1.1 minimal en `node:net` (CONNECT + PUBLISH), borné à 1,5 s, **fail-open**. Patron à reprendre. |
| Délégation observée par hook | Le matcher `Task` se déclenche toujours (15 lignes `ALLER` le 26/09 dans `~/.claude/iakaframe-delegations.log`) ; l'outil s'appelle désormais `Agent`, `Task` reste un alias. |
| Équipe active (roster) | `<projet>/.iakaframe` → `frame=iakaframe` ; `~/work/iakaframe/frames/iakaframe.md` → `teamId: iakaframe-8` ; `~/work/iakaframe/teams/iakaframe-8.md` → `personas: [odin, aragorn, gandalf, gimli, legolas, charon, helm, loki, nathalie, feanor]`, `coordinator: aragorn` ; `library/personas/<id>.md` → `name`, `pastille` (émoji). Même chaîne que la skill `iakastart` (§ 2). |
| Code de l'app | `agents.rs` : détection par fichiers, `ROSTER` figé (l. 88-98), `display_persona` figé (l. 103-117), `format_model_label` avec familles en dur (l. 332-356), arbre `build_session_node`/`build_children` (l. 597-794), tick 5 s, seuil `IAKAAM_LIVENESS_SECS`. `render.ts` : runner constant `"Claude Code"` (l. 198). `Cargo.toml` : `tauri`, `serde`, `serde_json`. |

## Faits externes vérifiés (web, 26/09)

- **Hooks Claude Code** : événements `SessionStart`, `SessionEnd`, `UserPromptSubmit`, `Stop`,
  `PreToolUse`, `PostToolUse`, **`SubagentStart`**, `SubagentStop`… `SubagentStart` reçoit
  seulement `session_id`, `transcript_path`, `cwd`, `agent_id`, `agent_type` : **ni consigne, ni
  description, ni modèle, ni parent** (demande d'ajout fermée « not planned », #69545).
  `SubagentStop` ajoute `last_assistant_message` et `agent_transcript_path`. Dans un hook déclenché
  **à l'intérieur** d'un sous-agent, `agent_id`/`agent_type` sont ceux de ce sous-agent. Hooks
  `command` : `async: true` possible, délai par défaut 600 s.
- **Piège connu** (#91910, ouvert, 2.1.259) : `SubagentStop` se déclenche aussi pour des appels
  internes (résumeurs), avec **`agent_type` vide** et un `agent_transcript_path` jamais créé —
  342 fantômes sur 381 dans une session mesurée. Il faut les **ignorer avant toute connexion**.
- **Client MQTT Rust** : `rumqttc` dernière version 0.25.1 (21/11/2025) ; iakaTokenCounter utilise
  0.24 contre ce même broker.

## Architecture retenue (sous réserve de Q1 à Q9, valeurs recommandées)

```
 runners                         bus local (iakahub, 127.0.0.1:1883, v4, anonyme)        app
 ───────                         ─────────────────────────────────────────────────       ───
 Claude Code ──hooks──► publieur  ──► iaka/agents/out/<runner>/<session>/<agent>  ──►  source « bus »  ─┐
 Codex (plus tard) ──► adaptateur ──►        (fiche JSON retenue, QoS 1)          │                    ├─► cœur neutre ─► tableau
 Ollama/iakaframe (plus tard) ──────►                                             │    (broker absent) │   (arbre, TTL,
                                                                                   └─  source « fichiers │    roster, libellés,
                                                                   Claude Code » (agents.rs actuel) ────┘    estimation)
```

- **Cœur neutre** (aucun nom de runner ni de fournisseur) : reçoit une liste de **fiches**, bâtit
  l'arbre de délégation par `parentId`, applique la péremption (`lastSeenAt`), résout persona
  (roster de l'équipe active) et libellés (fichier de configuration), calcule l'estimation
  (`estimate.rs` de `temps-restant.md`).
- **Sources** : `bus` (abonné MQTT) et `claude_code_files` (le code actuel de lecture de
  `~/.claude/projects`, réduit à produire des fiches). Q3 fixe quand la seconde sert.
- **Publieur Claude Code** : un script Node sans dépendance, appelé par hooks, qui publie et
  supprime les fiches. Seul endroit, avec l'adaptateur fichiers, où « Claude » a le droit d'être
  écrit.

## Contrat sur le fil — v1

### Topics

| Topic | Contenu | Retenu | QoS |
|---|---|---|---|
| `iaka/agents/out/<runner>/<sessionId>/<agentId>` | fiche d'agent (JSON ci-dessous) | **oui** | 1 |
| même topic, **payload vide (0 octet)** | suppression de la fiche (fin de l'agent) | oui | 1 |
| `iaka/agents/in/#` | **réservé** (commandes vers les agents), non utilisé en v1 | — | — |

- Convention héritée `iaka/<domaine>/{in,out}` : domaine `agents`, `out` = ce que les runners
  émettent. Arbre **disjoint** de `iakatokencounter/#` : aucune collision.
- Coordinateur de session : `agentId = "main"`.
- Segments `<runner>`, `<sessionId>`, `<agentId>` : `[A-Za-z0-9._-]{1,128}`. Tout autre caractère
  ⇒ le publieur **ne publie pas** (jamais de `+`, `#`, `/` dans un segment).
- Abonnement de l'app : `iaka/agents/out/+/+/+`, QoS 1.

### Fiche (payload JSON, UTF-8, ≤ 4 096 octets)

```json
{
  "v": 1,
  "runner": "claude-code",
  "runnerVersion": "2.1.270",
  "model": "claude-opus-5-5",
  "persona": "gandalf",
  "agentId": "a5ba54b2df0194497",
  "parentId": "main",
  "sessionId": "5b57a341-6954-4ccf-9361-6da5ac8ac839",
  "project": "iakaos",
  "cwd": "/Users/sjupin/work/iakaos",
  "task": "Cadrer MQTT agnostique (~10 min)",
  "estimate": "~10 min",
  "startedAt": 1790412000,
  "lastSeenAt": 1790412095,
  "state": "running"
}
```

| Champ | Type | Obligatoire | Règle |
|---|---|---|---|
| `v` | entier | oui | version **majeure** du contrat. L'app n'accepte que `1` ; autre valeur ⇒ fiche ignorée (journal). Ajout de champ = compatible, pas de changement de `v`. |
| `runner` | chaîne | oui | identifiant du runner (`claude-code`, `codex`, `ollama`…) ; **égal** au segment de topic. |
| `runnerVersion` | chaîne \| null | non | version du runner si connue. |
| `model` | chaîne \| null | non | identifiant **brut** du modèle ; le libellé vient du fichier de configuration. |
| `persona` | chaîne \| null | non | identifiant de persona (`gandalf`…) ; `null` pour un coordinateur ou un agent sans type. |
| `agentId` | chaîne | oui | égal au segment de topic ; `"main"` pour le coordinateur. |
| `parentId` | chaîne \| null | oui | `null` pour `main` ; `"main"` pour un enfant direct du coordinateur ; sinon `agentId` du parent. |
| `sessionId` | chaîne | oui | égal au segment de topic. |
| `project` | chaîne \| null | non | nom court du projet ; si absent, l'app le déduit de `cwd`. |
| `cwd` | chaîne \| null | non | ≤ 512 caractères ; sert au projet et à la résolution de l'équipe active. |
| `task` | chaîne \| null | non | libellé court tel que donné par le runner, **suffixe `(~…)` compris** ; ≤ 200 caractères. Jamais la consigne complète. |
| `estimate` | chaîne \| null | non | **valeur brute annoncée** (`~10 min`, `~10-15 min`), extraite sans validation par le publieur ; **validée par l'app** avec la grammaire de `temps-restant.md` (une seule grammaire, côté app). |
| `startedAt` | entier | oui | epoch **secondes** UTC (cohérent avec le contrat conso). |
| `lastSeenAt` | entier | oui | epoch secondes du dernier signe de vie. |
| `state` | chaîne | oui | `"running"` ou `"idle"` (coordinateur entre deux tours). Valeur inconnue ⇒ traitée comme `running`. |

Validation côté app : JSON invalide, champ obligatoire manquant, taille > 4 096 octets, ou
incohérence topic ↔ (`runner`, `sessionId`, `agentId`) ⇒ fiche **ignorée**, jamais de plantage.
Champs inconnus ignorés.

### Fin, plantage, expiration

- **Fin normale** : le publieur envoie un **payload vide retenu** sur le topic de la fiche
  (`SubagentStop` pour un sous-agent, `SessionEnd` pour `main`). Le broker efface le retenu.
- **Plantage** (processus tué, pas de `SubagentStop`) : la fiche reste retenue. **Expiration
  logique** côté app : fiche **masquée** si `now - lastSeenAt > IAKAAM_LIVENESS_SECS` (défaut 90,
  seuil existant). L'expiration native MQTT n'existe qu'en v5, absente d'iakahub (vérifié).
- **Last Will (LWT)** : **sans objet pour les hooks** — chaque appel ouvre une connexion courte et
  la ferme proprement, le testament ne partirait jamais. Réservé aux **publieurs de longue durée**
  futurs (adaptateur Codex) : topic `iaka/agents/out/_publisher/<publisherId>`, retenu, `online` à
  la connexion, testament retenu `offline`. **Non implémenté en v1**, documenté ici pour Codex.
- **Plafond de rattrapage** : au-delà de 100 fiches retenues sous le filtre, le broker tronque ce
  que reçoit un nouvel abonné. L'app **journalise** un avertissement à partir de 80 fiches reçues.

## Questions fermées (arbitrage de Stéphane)

**Q1 — Quel broker ?**
- **A (recommandé) — Réutiliser iakahub** (`127.0.0.1:1883`), sous l'arbre `iaka/agents/`.
  Justification de la réutilisation plutôt que l'isolation : iakahub est conçu comme le **bus
  commun du poste** (« backbone standalone »), pas comme la stack d'un projet ; les arbres sont
  disjoints ; la règle d'isolation par projet vise les stacks Docker, et un second broker
  obligerait chaque runner à connaître deux bus. **Limite assumée** : le broker ne vit que tant que
  la tray iakaTokenCounter tourne ; sinon, repli (Q3).
- B — Broker dédié embarqué dans IakaAgentsMonitor (`rumqttd` en processus, port propre, ex.
  `18830`) : isolé, mais l'app devient infrastructure, et Codex/iakaframe devront viser ce port.
- C — Mosquitto (Homebrew) en service `launchd` : bus permanent, indépendant d'iakaTokenCounter,
  mais une installation sur le Mac et un service de plus à tenir.

**Q2 — Signe de vie ?**
- **A (recommandé) — Battement par hook** : `PostToolUse` (tous outils), **asynchrone**, **limité à
  une publication toutes les 20 s par agent** (horodatage gardé dans un petit fichier d'état) ;
  plus `UserPromptSubmit`/`Stop` pour le coordinateur. Péremption à 90 s.
- B — Pas de battement : publication au début, suppression à la fin, péremption longue (30 min) :
  un agent planté reste affiché jusqu'à 30 min.
- C — Identité par le bus, vivacité par la date de modification des fichiers : casse
  l'agnosticisme.

**Q3 — Repli sur la lecture des fichiers ?**
- **A (recommandé, conforme à la décision) — Repli strict** : broker joignable ⇒ **bus seul** ;
  broker injoignable ⇒ **source fichiers seule**. Bascule au plus tard au tick suivant (5 s).
  Conséquence : broker présent mais hooks non installés ⇒ liste vide ; hooks et app se livrent
  donc **ensemble**.
- B — Fusion permanente, bus prioritaire (doublons éliminés par `runner`+`sessionId`+`agentId`) :
  plus tolérant pendant la transition, mais l'app lit toujours les fichiers de Claude Code.
- C — Bus seul, sans repli : l'app est vide dès que iakaTokenCounter est fermé.

**Q4 — Nettoyage des fiches d'agents plantés ?**
- **A (recommandé) — Aucun nettoyage par l'app** : elles sont masquées par la péremption, et le
  broker (en mémoire) les oublie à son prochain redémarrage. L'app reste un pur abonné.
- B — L'app efface les fiches périmées (payload vide retenu) : bus propre, mais l'app écrit sur le
  bus.
- C — Le publieur balaie les fiches de sa session au `SessionStart` : suppose de s'abonner depuis un
  hook, plus lourd.

**Q5 — Transport de l'estimation ?**
- **A (recommandé) — Texte brut** : le publieur extrait bêtement la valeur (ligne `Durée estimée :`
  parmi les 5 premières lignes non vides de la consigne, sinon suffixe `(~…)` du libellé) et la
  met dans `estimate` ; l'app la **valide** avec sa grammaire. Une seule grammaire, la consigne ne
  circule jamais sur le bus.
- B — Le publieur envoie une valeur déjà calculée (`{loSecs, hiSecs}`) : grammaire dupliquée en
  JavaScript.
- C — Le publieur envoie la tête de la consigne (≤ 1 Kio) : texte potentiellement sensible sur un
  bus non authentifié.

**Q6 — Où vit le script de publication ?**
- **A (recommandé) — Dans ce dépôt**, `publishers/claude-code/iaka-agents-publish.mjs`, versionné
  avec le contrat ; **copié** par Stéphane dans `~/.claude/`. Migration possible vers le kit
  iakaframe plus tard.
- B — Tout de suite dans le kit iakaframe (`kits/iakaframe-claude/global/hooks/`) : lot à cadrer
  dans le dépôt iakaframe.

**Q7 — Roster des personas ?**
- **A (recommandé) — Lu par agent depuis l'équipe active de son projet** : `<cwd>/.iakaframe`
  (`frame=`) → `<racine iakaframe>/frames/<frame>.md` (`teamId`) → `teams/<teamId>.md`
  (`personas`) → `library/personas/<id>.md` (`name`, `pastille`). Pointeur absent ⇒ frame
  `iakaframe`. Racine iakaframe configurable (défaut `~/work/iakaframe`), relue au plus toutes les
  60 s. Couleur par pastille et lettres imposées (Gimli `I`) dans le fichier de configuration.
  Réservoir illisible ⇒ tout persona en gris avec sa lettre (jamais de plantage).
- B — Un seul roster, celui de la frame par défaut : plus simple, faux pour un projet sur une autre
  frame.
- C — Roster publié sur le bus par iakaframe : n'existe pas, lot iakaframe à part.

**Q8 — Ordre avec « temps restant » ?**
- **A (recommandé)** — Livrer d'abord `temps-restant.md` (validé, attend la maquette de Loki) ;
  ce lot vient ensuite et réutilise `estimate.rs` tel quel.
- B — Ce lot d'abord ; `temps-restant.md` est alors ré-aligné sur les fiches (sa « source Claude
  Code » disparaît au profit du champ `estimate`).

**Q9 — Format du fichier de configuration ?**
- **A (recommandé) — TOML** (crate `toml`), lisible et commentable, comme `models.toml` ailleurs
  chez Stéphane.
- B — JSON : aucune dépendance nouvelle, mais pas de commentaires.

## Décisions arbitraires (mineures, révisables en une phrase)

- **DA-M1** Client MQTT : `rumqttc` **0.24** (même version qu'iakaTokenCounter, éprouvée contre ce
  broker) ; passage à 0.25 dans un lot à part.
- **DA-M2** Connexion de l'app : hôte **forcé à `127.0.0.1`** (aucune variable pour le changer),
  port `IAKAAM_MQTT_PORT` (défaut 1883), identifiant client `iakaagentsmonitor-<pid>`, session
  propre, QoS 1, reconnexion avec attente croissante bornée à 30 s.
- **DA-M3** Publieur : connexion courte `node:net` (patron de `delegation-guard.mjs`), budget total
  **1,5 s**, **code de sortie toujours 0**, aucune sortie sur stdout, hooks en `async: true` sauf
  `SessionEnd` (synchrone, borné par le même budget, pour que la suppression parte avant la
  fermeture).
- **DA-M4** État du publieur : `~/.cache/iaka-agents/<sessionId>/<agentId>.json` (fiche courante +
  date de dernière publication) ; dossier de session supprimé au `SessionEnd`.
- **DA-M5** Enrichissement : `SubagentStart` ne donne ni tâche ni parent ; le publieur lit le
  compagnon `agent-<id>.meta.json` (`description`, `parentAgentId`, `model`) et la première ligne
  utilisateur du transcript du sous-agent (≤ 64 Kio) **à chaque battement tant que `task` est
  vide**, au plus 5 fois. Parent absent du compagnon ⇒ `"main"`.
- **DA-M6** Coordinateur : fiche `main` publiée au `SessionStart` ; `task` = début du premier
  `UserPromptSubmit` (60 caractères, comme A3) ; `state` = `idle` au `Stop`, `running` au
  `UserPromptSubmit`. Persona affiché pour `main` : clé `main_persona` de la configuration
  (défaut `odin`, convention actuelle).
- **DA-M7** Événements `SubagentStart`/`SubagentStop` à `agent_type` vide : **ignorés avant toute
  connexion** (fantômes #91910).
- **DA-M8** Fichier de configuration : `labels.default.toml` embarqué (`include_str!`), surchargé
  par `~/Library/Application Support/com.iakaframe.iakaagentsmonitor/labels.toml` s'il existe.
  Contenu : `[runners]` (id → libellé), `[[models]]` (`prefix`, `label` ; **préfixe le plus long**
  gagne ; aucun ⇒ identifiant brut), `[pastilles]` (émoji → couleur de fond et de texte),
  `[letters]` (persona → lettre imposée), `main_persona`, `iakaframe_root`.

## Périmètre

- **Inclus** : contrat v1 ci-dessus ; module de fiche (lecture, validation) ; source bus (abonné
  MQTT dans un fil dédié) ; transformation de la lecture de fichiers actuelle en source qui produit
  des fiches ; cœur d'arbre neutre construit à partir des fiches (mêmes règles d'affichage qu'avant :
  remontée d'ancêtre mort, bornes 6/8, tri) ; bascule de repli (Q3) ; roster lu depuis l'équipe
  active (Q7) ; fichier de configuration des libellés (Q9, DA-M8) ; colonne Runner alimentée par la
  fiche ; script de publication Claude Code et ses tests ; **extrait de configuration des hooks**
  prêt à coller (fourni, **non appliqué**) ; tests d'intégration contre un broker éphémère.
- **Exclu** : **toute modification de `~/.claude/settings.json` ou de `~/.claude/`** (geste de
  Stéphane) ; toute modification d'iakaTokenCounter, d'iakahub, d'iakaHub, d'iakaframe ; adaptateur
  Codex ; journal Ollama/iakaframe ; testament (LWT) ; authentification ou TLS du bus ; plusieurs
  machines ; persistance ; nettoyage des fiches par l'app (sauf Q4-B) ; commandes vers les agents
  (`iaka/agents/in`) ; nouvelle présentation (le tableau V4 reste tel quel, à la colonne Runner
  près) ; Linux.

## Étapes d'implémentation

1. **Fiche.** Nouveau `src-tauri/src/card.rs` : `struct AgentCard` (champs du contrat, camelCase),
   `parse_card(topic: &str, payload: &[u8]) -> Result<Option<AgentCard>, CardError>` (`Ok(None)` =
   payload vide = suppression), bornes de taille, cohérence topic/fiche, `v == 1`. Aucun nom de
   runner. Tests. Commit.
2. **Configuration.** `src-tauri/config/labels.default.toml` + `src-tauri/src/labels.rs` :
   chargement (défaut embarqué, surcharge utilisateur), `runner_label`, `model_label` (préfixe le
   plus long), couleurs de pastille, lettres imposées. Les correspondances d'aujourd'hui
   (`format_model_label`, `ROSTER`, `display_persona`, `"Claude Code"`) passent **dans ce fichier**.
   Ajout de la crate `toml` (Q9-A). Tests (reprise des cas de `format_model_label` comme données).
   Commit.
3. **Roster.** `src-tauri/src/roster.rs` : résolution Q7-A avec cache 60 s ; persona hors équipe ⇒
   gris + sa lettre ; nom affiché = `name` du persona. Tests sur un faux réservoir en dossier
   temporaire (pointeur présent, absent, frame inconnue, persona hors équipe, pastille variable
   `—`). Commit.
4. **Cœur neutre.** `src-tauri/src/tree.rs` : `build_snapshot(cards, now, liveness, &Labels,
   &Roster) -> AgentsSnapshot` — reprise de `build_session_node`/`build_children`/`snapshot_at`
   avec `parentId` et `lastSeenAt` au lieu des fichiers ; `estimate` validé par `estimate.rs`
   (après Q8-A) ; `task` sans suffixe valide. `AgentNode` gagne `runner` (libellé). Les tests
   d'arbre existants sont **transposés sur des fiches**, mêmes assertions. Commit.
5. **Source fichiers.** `agents.rs` devient `src-tauri/src/sources/claude_code_files.rs` : la
   découverte et la lecture actuelles produisent des `AgentCard` (`runner = "claude-code"`,
   `lastSeenAt` = date de modification, `startedAt` = date de création, `parentId` depuis le
   compagnon, `"main"` à défaut). Plus d'arbre ni de libellé ici. Tests de lecture existants
   conservés. Commit.
6. **Source bus.** `src-tauri/src/sources/bus.rs` : fil dédié `rumqttc` (DA-M1, DA-M2), table
   `topic → fiche` mise à jour à chaque message, suppression sur payload vide, drapeau
   `connected`. Avertissement journalisé à 80 fiches. Commit.
7. **Bascule + watcher.** `run_watcher` (tick 5 s) : prend les fiches de la source active selon
   Q3-A, appelle `tree::build_snapshot`, émet `tray://agents` si changement. Infobulle de l'icône :
   `N agent(s) en cours — source : bus` ou `— source : fichiers (bus absent)`. Commit.
8. **Rendu.** `src/types.ts` : champ `runner` ; `src/render.ts` : la colonne Runner affiche
   `node.runner` (plus de constante). Rien d'autre ne change à l'écran. Commit.
9. **Publieur Claude Code.** `publishers/claude-code/iaka-agents-publish.mjs` (Node ≥ 18, aucune
   dépendance) : un seul script pour `SessionStart`, `UserPromptSubmit`, `Stop`, `SessionEnd`,
   `SubagentStart`, `SubagentStop`, `PostToolUse` (lit `hook_event_name` sur stdin), règles DA-M3 à
   DA-M7, extraction de l'estimation (Q5-A), limitation de débit (Q2-A). En-tête : renvoi à ce
   contrat. `publishers/claude-code/README.md` : **extrait `hooks` à coller** dans
   `~/.claude/settings.json` (entrées **ajoutées** à côté des hooks existants, jamais à leur place)
   et commande de copie. Commit.
10. **Tests du publieur.** `publishers/claude-code/test/*.test.mjs` (`node --test`), avec un **faux
    serveur MQTT** en `node:net` qui enregistre CONNECT/PUBLISH : chaque événement → bon topic,
    bon drapeau retenu, bonne fiche ; `SubagentStop` → payload vide retenu ; `agent_type` vide →
    **aucune connexion** ; deux `PostToolUse` à 5 s d'écart → une seule publication ; broker absent
    → sortie code 0 en < 2 s ; compagnon absent puis présent → `task` rempli au battement suivant.
    Script `npm run test:publisher`. Commit.
11. **Intégration.** Test Rust contre un **broker éphémère** : `rumqttd` 0.19 en
    `dev-dependencies`, lancé en processus sur un port libre (patron des tests d'iakahub) :
    fiche retenue publiée avant l'abonnement → reçue ; payload vide → disparue ; fiche périmée
    (91 s) → masquée ; broker arrêté → bascule sur la source fichiers. Commit.
12. **Vérification.** `cargo test`, `cargo clippy --all-targets -- -D warnings`,
    `npm run typecheck`, `npm run build`, `npm run test:publisher`, `npm run tauri build`, puis
    gate qualité (Legolas), puis **installation des hooks par Stéphane** et essai réel.

## Fichiers concernés

- `src-tauri/src/card.rs`, `labels.rs`, `roster.rs`, `tree.rs` — **nouveaux** (cœur neutre).
- `src-tauri/src/sources/mod.rs`, `sources/bus.rs` — **nouveaux**.
- `src-tauri/src/agents.rs` → `src-tauri/src/sources/claude_code_files.rs` — **déplacé et réduit**
  (lecture seule, produit des fiches).
- `src-tauri/src/claude_paths.rs` — inchangé (propre à l'adaptateur), `bucket_project` réemployé
  par le cœur pour déduire le projet de `cwd` (à déplacer dans le cœur s'il ne contient aucun nom
  de runner).
- `src-tauri/src/lib.rs`, `state.rs`, `tray.rs` — déclarations, drapeau de source, infobulle.
- `src-tauri/config/labels.default.toml` — **nouveau**.
- `src-tauri/Cargo.toml` — `rumqttc = "0.24"`, `toml` ; `rumqttd = "0.19"` en `dev-dependencies`.
- `src/types.ts`, `src/render.ts` — champ et colonne Runner.
- `publishers/claude-code/iaka-agents-publish.mjs`, `README.md`, `test/` — **nouveaux**.
- `package.json` — script `test:publisher`.
- `CLAUDE.md` du projet — section « Commandes » : `npm run test:publisher` ; une ligne sur le bus.

## Risques

- **R1 — Broker absent** dès que la tray iakaTokenCounter est fermée. *Mitigation* : repli fichiers
  (Q3) ; si c'est trop fréquent, Q1-C dans un lot à part.
- **R2 — Identifiants différents** entre le hook (`agent_id`) et les fichiers (`agent-<id>`) : non
  prouvé qu'ils soient identiques. *Mitigation* : premier point de la recette ; si différents,
  l'enrichissement DA-M5 passe par `agent_transcript_path`/le compagnon le plus récent — à
  re-cadrer.
- **R3 — Compagnon pas encore écrit au `SubagentStart`**. *Mitigation* : DA-M5 (relecture aux
  battements suivants).
- **R4 — Fantômes `SubagentStop`** (#91910). *Mitigation* : DA-M7.
- **R5 — Coût du hook `PostToolUse` sur tous les outils** : un démarrage de Node par appel
  d'outil, dans toutes les sessions. *Mitigation* : asynchrone (ne ralentit pas la session),
  sortie immédiate si publication récente (lecture d'un fichier, pas de réseau).
- **R6 — Agent qui réfléchit plus de 90 s sans appeler d'outil** : disparaît temporairement
  (même défaut qu'aujourd'hui avec la date de modification). *Mitigation* : `IAKAAM_LIVENESS_SECS`.
- **R7 — Sécurité (dette assumée, à consigner)** : bus **anonyme**, en clair. Tout processus local
  peut **lire** les tâches des agents et **publier de fausses fiches**. Bornes : écoute **limitée à
  `127.0.0.1`** côté broker (vérifié), connexion **forcée à `127.0.0.1`** côté app (DA-M2), la
  consigne ne circule jamais (Q5-A), tâche ≤ 200 caractères, fiches validées et affichées en
  texte brut (jamais en HTML). À reprendre si le bus sort un jour du poste (authentification, TLS).
- **R8 — Plafond de 100 retenus par filtre** : sans objet pour quelques dizaines d'agents ;
  avertissement à 80 (étape 6).
- **R9 — Réservoir iakaframe déplacé ou renommé** : roster en gris. *Mitigation* :
  `iakaframe_root` configurable.
- **R10 — Formats non contractuels de Claude Code** (hooks récents, compagnons) : confinés au
  publieur et à la source fichiers.

## Critères d'acceptation

- [ ] `cargo test` vert : tests de fiche, configuration, roster, arbre transposé (mêmes assertions
      qu'avant : remontée d'ancêtre mort, bornes 6/8, liveness 0/89/90/91 s, instantanés égaux),
      source fichiers, intégration contre broker éphémère.
- [ ] `cargo clippy --all-targets -- -D warnings` sans avertissement ; `npm run typecheck`,
      `npm run build`, `npm run test:publisher` verts ; `npm run tauri build` produit une `.app`.
- [ ] **Neutralité** : `grep -riE 'claude|anthropic|opus|sonnet|haiku|codex|openai|ollama'` sur
      `card.rs`, `labels.rs`, `roster.rs`, `tree.rs`, `estimate.rs`, `sources/bus.rs`, `src/*.ts`
      ne renvoie **rien**. Ces noms n'apparaissent que dans `sources/claude_code_files.rs`,
      `claude_paths.rs`, `publishers/claude-code/` et `config/labels.default.toml`.
- [ ] Plus aucune table de personas codée en dur : ajouter un persona dans l'équipe active d'un
      projet (réservoir de test) le fait apparaître avec son nom et sa pastille, sans recompiler.
- [ ] Changer un libellé de modèle dans `labels.toml` (surcharge utilisateur) change l'affichage au
      redémarrage, sans recompiler.
- [ ] Pendant que l'app tourne : `lsof -iTCP -sTCP:LISTEN -p <pid>` vide (aucun port ouvert) ;
      sa seule connexion est vers `127.0.0.1:1883`.
- [ ] Publication à la main (`mosquitto_sub`/`mosquitto_pub` ou test) d'une fiche `runner:"essai"`
      valide ⇒ ligne affichée en ≤ 5 s avec le runner « essai » (brut, pas de libellé) ; payload
      vide ⇒ ligne disparue en ≤ 5 s ; `lastSeenAt` vieux de 91 s ⇒ ligne masquée.
- [ ] Fiche invalide (JSON cassé, `v: 2`, topic incohérent, > 4 096 octets) ⇒ ignorée, app
      intacte.
- [ ] Broker arrêté (tray iakaTokenCounter fermée) ⇒ en ≤ 5 s la liste revient par les fichiers,
      infobulle « source : fichiers (bus absent) » ; broker relancé ⇒ retour au bus.
- [ ] Publieur : broker absent ⇒ le hook sort en code 0 en < 2 s, la session Claude Code n'est ni
      bloquée ni ralentie ; événement à `agent_type` vide ⇒ aucune connexion.
- [ ] **Essai réel (Stéphane, après installation des hooks)** : une session dans un projet X avec
      2 à 3 délégations dont une imbriquée ⇒ mêmes lignes, même indentation, mêmes personas, tâches
      et modèles qu'avec la lecture de fichiers ; `SubagentStop` ⇒ la ligne disparaît en ≤ 5 s ;
      une délégation dont la consigne commence par `Durée estimée : ~2 min` ⇒ barre « Reste »
      (après Q8-A).
- [ ] Aucun fichier de `~/.claude/`, d'iakaTokenCounter, d'iakahub, d'iakaHub ni d'iakaframe
      modifié par ce lot ; aucun secret dans le dépôt.

## Estimation (ordre de grandeur, révisable)

- **Charge : ~3,25 jours-homme** (fourchette 2,5 à 4,5 j) — fiche + contrat 0,3 ; configuration
  0,25 ; roster 0,4 ; cœur d'arbre sur fiches 0,5 ; source fichiers réduite 0,3 ; source bus 0,4 ;
  publieur 0,5 ; tests d'intégration 0,3 ; qualité + recette 0,3.
- **Complexité / risque : moyen.** Le code d'arbre existe et est testé ; la nouveauté est le
  découpage en sources et le publieur, qui dépend d'événements Claude Code récents et mal
  documentés.
- **Inconnues qui peuvent faire glisser** : R2 (identifiants hook ≠ fichiers : +0,5 j) ; champs
  réels de `SessionStart` (modèle présent ou non) ; comportement des hooks pour les délégations de
  l'outil `Workflow` ; réponse B à Q1 (+0,5 j) ou C (+0,3 j, installation) ; Q7-B (−0,2 j).

## Ce qui appartient à Stéphane

- Trancher **Q1 à Q9** (ou « go » pour les recommandations).
- **Installer les hooks** : copier `publishers/claude-code/iaka-agents-publish.mjs` dans
  `~/.claude/` et **ajouter** l'extrait fourni à `~/.claude/settings.json`. C'est sa configuration
  globale, qui touche **toutes** ses sessions : aucun agent ne le fait à sa place.
- Garder la tray iakaTokenCounter lancée pendant l'essai (elle porte le broker), ou décider plus
  tard d'un bus permanent (Q1-C).
- Accepter la **dette de sécurité** R7 (bus local non authentifié), à consigner dans
  `specs/PROJET.md`.
- L'essai réel, et plus tard : adaptateur Codex, journal Ollama/iakaframe, migration du publieur
  dans le kit iakaframe.

---

## Amendement A1 (27/09/2026) — commande pause / reprise / arrêt par le bus

> **PROPOSÉ, en attente d'arbitrage** : QC1 à QC9 de `specs/instructions/controle-agents-etude.md`
> (faits vérifiés, limites, sécurité et sources y sont détaillés). Rédigé aux valeurs recommandées
> (toutes A). Sous-lot **séparé**, lancé seulement après le gate du lot de base (étapes 1 à 12).
> Le changement de modèle d'un agent en cours est **impossible** (étude, § a) et **n'est pas** dans
> A1.

### Ce que A1 change dans le lot de base

- **Topic `iaka/agents/in/…`** : n'est plus réservé (tableau « Topics » et ligne « Exclu : commandes
  vers les agents » levés **pour A1 seulement**).
- **Q4-A amendée** : l'app n'est plus un pur abonné ; elle **publie**, mais **uniquement** sous
  `iaka/agents/in/`, **jamais** sous `iaka/agents/out/`.
- **`state`** gagne `paused` et `stopping` (compatible : valeur inconnue ⇒ `running` pour une app
  plus ancienne ; `v` reste 1).
- Critère « l'app n'ouvre aucun port en écoute » : **inchangé**.

### Contrat de commande — v1

| Topic | Contenu | Retenu | QoS |
|---|---|---|---|
| `iaka/agents/in/<runner>/<sessionId>/<agentId>` | commande signée (JSON ci-dessous) = **état désiré** | **oui** | 1 |
| même topic, **payload vide** | plus de commande = **reprise** (`running`) | oui | 1 |

```json
{ "v": 1, "cmd": "pause", "issuedAt": 1790499000, "nonce": "9f1c2a7e5b3d4c61", "sig": "<hex>" }
```

- `cmd` ∈ `pause`, `stop`. La reprise n'est **pas** une commande : c'est l'effacement du retenu.
  Valeur inconnue ⇒ ignorée.
- `sig` = HMAC-SHA256, en hexadécimal minuscule, de
  `topic + "\n" + cmd + "\n" + issuedAt + "\n" + nonce`, avec la clé de 32 octets lue dans
  `~/.config/iaka-agents/control.key` (dossier `0700`, fichier `0600`, **créé par l'app** au
  premier lancement s'il n'existe pas, jamais écrasé).
- Payload ≤ 512 octets ; segments soumis aux mêmes règles que `out`.

### Décisions arbitraires (mineures, révisables en une phrase)

- **DA-C1** Contrôleur : `publishers/claude-code/iaka-agents-control.mjs` (QC9-A), Node ≥ 18 sans
  dépendance ; client MQTT minimal **partagé** avec le publieur (`publishers/claude-code/mqtt-lite.mjs`,
  CONNECT, PUBLISH, SUBSCRIBE, réception PUBLISH, PINGREQ). Hook `PreToolUse`, **tous outils**,
  **synchrone**, `timeout: 3600`.
- **DA-C2** Chemin normal : clé absente ⇒ sortie immédiate sans décision (contrôleur inactif, **sans
  connexion**). Sinon connexion, abonnement QoS 1 au **topic exact** de l'agent, attente du retenu
  jusqu'à **30 ms après le SUBACK** ; **budget total 250 ms** ; toute erreur ou dépassement ⇒ sortie
  code 0 **sans décision** (l'outil s'exécute). Jamais de sortie sur stdout hors décision JSON.
- **DA-C3** Vérification : signature fausse ou absente, JSON invalide, `cmd` inconnue ⇒ **ignorée**
  (journal `~/.cache/iaka-agents/control.log`, une ligne, sans le payload). Anti-rejeu : le
  contrôleur note dans `~/.cache/iaka-agents/<sessionId>/<agentId>.json` la date de la dernière
  reprise observée ; une commande dont `issuedAt` est antérieur ou égal ⇒ ignorée.
- **DA-C4** Pause : fiche publiée avec `state: "paused"` ; connexion **gardée ouverte** ; fiche
  republiée (`lastSeenAt`) toutes les **20 s** ; payload vide reçu ⇒ fiche `running`, sortie sans
  décision ; `stop` reçu ⇒ DA-C5 ; broker perdu ⇒ sortie sans décision (reprise) ; à **3 540 s**
  ⇒ `permissionDecision: "deny"`, motif « Pause prolongée par l'opérateur : refais exactement le
  même appel. » (QC5-A).
- **DA-C5** Arrêt : fiche `state: "stopping"`, puis
  `{"continue": false, "stopReason": "Arrêté par l'opérateur depuis IakaAgentsMonitor. Ne pas relancer sans consigne de l'utilisateur."}`.
  Sous-agents et coordinateur (QC4-A).
- **DA-C6** App, publication : même connexion que l'abonné (DA-M2) ; commande QoS 1 retenue ;
  reprise = payload vide retenu. Quand une fiche disparaît (payload vide ou péremption), l'app
  **efface** la commande retenue de cet agent si elle en avait publié une.
- **DA-C7** App, colonne **« État »** : première colonne, non triable, visible en compact, étroite ;
  bouton `running` / `paused` (clic = bascule), `idle` cliquable (pause au tour suivant),
  `stopping` inactif ; libellés tels quels (QC6-A). États intermédiaires « pause demandée » /
  « reprise demandée » tant que la fiche ne reflète pas l'état ; sans effet après **30 s** ⇒ retour
  à l'état réel + infobulle « non pris en compte (contrôleur absent ?) ». Arrêt : petit bouton
  secondaire, **confirmation dans la ligne** (« Arrêter <agent> ? Arrêter / Annuler »), sans
  nouveau plugin. Source fichiers active (bus absent) ⇒ boutons **désactivés**, infobulle « bus
  absent : commande indisponible ». Pas de maquette (QC7-A), jetons V4 existants.
- **DA-C8** Crates Rust : `hmac`, `sha2`, `getrandom` (clé et `nonce`) — versions stables du jour
  à fixer par Gimli (non vérifiées au cadrage).

### Étapes d'implémentation (A1)

13. **Contrat.** Mettre à jour la section « Contrat sur le fil » (topic `in`, valeurs `paused`,
    `stopping`) ; nouveau `src-tauri/src/control.rs` : clé (création `0600`, lecture), signature,
    construction du payload, **aucun nom de runner**. Tests (vecteurs de signature partagés avec
    le contrôleur Node). Commit.
14. **Client MQTT partagé.** `publishers/claude-code/mqtt-lite.mjs` (abonnement + réception) ; le
    publieur l'utilise à la place de son code de connexion. Tests existants du publieur verts.
    Commit.
15. **Contrôleur.** `iaka-agents-control.mjs` selon DA-C1 à DA-C5. Tests `node --test` avec le faux
    serveur MQTT (étendu à SUBSCRIBE) : pas de commande ⇒ sortie sans décision ; `pause` puis
    effacement ⇒ fiche `paused` puis `running`, sortie ; `pause` puis `stop` ⇒ `continue:false` ;
    signature fausse ⇒ ignorée ; rejeu ⇒ ignoré ; broker absent ⇒ code 0 < 300 ms ; clé absente ⇒
    aucune connexion ; broker coupé en pause ⇒ sortie sans décision ; relâche à 3 540 s (horloge
    injectée) ⇒ `deny`. Commit.
16. **App, publication.** Commande Tauri `set_agent_control(runner, sessionId, agentId, cmd)`
    (`pause`, `resume`, `stop`) ⇒ publication DA-C6 ; effacement à la disparition de la fiche.
    Commit.
17. **App, colonne « État ».** `src/render.ts`, `src/types.ts` : DA-C7. Commit.
18. **Intégration.** Broker éphémère : commande signée publiée par l'app ⇒ reçue et validée par le
    contrôleur lancé en processus ; fiche `paused` ⇒ bouton `paused`. **Mesure de latence** du
    chemin normal (50 appels) consignée dans le rapport. Commit.
19. **Vérification.** Mêmes contrôles qu'à l'étape 12 ; gate Legolas ; installation par Stéphane
    (extrait `PreToolUse` ajouté au `README.md` du publieur, **non appliqué**) ; essai réel.

### Fichiers concernés (A1)

- `src-tauri/src/control.rs` — **nouveau** ; `sources/bus.rs` — publication ; `lib.rs` — commande
  Tauri ; `Cargo.toml` — `hmac`, `sha2`, `getrandom`.
- `src/render.ts`, `src/types.ts` — colonne « État », confirmation, états intermédiaires.
- `publishers/claude-code/iaka-agents-control.mjs`, `mqtt-lite.mjs` — **nouveaux** ;
  `iaka-agents-publish.mjs` — utilise `mqtt-lite.mjs` ; `README.md` — extrait `PreToolUse` ;
  `test/` — tests du contrôleur.
- `specs/PROJET.md` — dette de sécurité résiduelle (écrite par Gimli sur consigne de ce lot).

### Risques (A1)

- **RC1 — La pause n'agit qu'au prochain appel d'outil** (aucune interruption en pleine
  génération) : états intermédiaires et retour à 30 s.
- **RC2 — Ralentissement de toutes les sessions** : budget 250 ms, mesure étape 18 ; au-delà d'une
  médiane de 150 ms ajoutés, arrêt et re-cadrage vers QC2-B (fichier local).
- **RC3 — Le parent relance un sous-agent arrêté** : motif explicite ; observé en recette.
- **RC4 — Relâche de la pause** au délai du hook : refus motivé à 3 540 s ; réessai non garanti.
- **RC5 — Sécurité résiduelle** : un processus du même compte peut lire la clé ; rejeu limité à la
  même cible et bloqué après reprise. À consigner.
- **RC6 — Champs de hook récents** (`agent_id` dans `PreToolUse`, `continue:false` dans un
  sous-agent) : premiers points de la recette ; si `agent_id` manque, contrôleur limité à `main`.

### Critères d'acceptation (A1)

- [ ] `cargo test`, `cargo clippy --all-targets -- -D warnings`, `npm run typecheck`,
      `npm run build`, `npm run test:publisher` verts (tests du contrôleur compris).
- [ ] **Neutralité** : `control.rs` passe le même `grep` que les autres modules du cœur.
- [ ] Clé créée par l'app : `stat` donne `0600` (fichier) et `0700` (dossier) ; relancer l'app ne
      la change pas.
- [ ] Commande non signée publiée à la main sur `iaka/agents/in/…` ⇒ **aucun effet**, une ligne dans
      `control.log`.
- [ ] Contrôleur, broker absent ⇒ code 0 en < 300 ms, aucune décision ; clé absente ⇒ aucune
      connexion (faux serveur : zéro CONNECT).
- [ ] Mesure étape 18 : médiane ajoutée par appel d'outil ≤ 150 ms, consignée.
- [ ] Bus absent ⇒ boutons désactivés avec infobulle ; aucune commande publiée.
- [ ] **Essai réel (Stéphane, après installation)** : un sous-agent qui enchaîne des lectures ⇒ clic
      `running` ⇒ au plus un appel d'outil plus tard, bouton `paused` et plus aucun appel ; clic ⇒
      reprise, l'agent termine normalement ; pendant une pause de 2 min, la ligne **reste affichée** ;
      arrêt confirmé ⇒ la ligne passe `stopping` puis disparaît, le coordinateur reçoit le motif ;
      tray iakaTokenCounter fermée pendant une pause ⇒ l'agent repart.
- [ ] Aucun fichier de `~/.claude/`, d'iakaTokenCounter ni d'iakahub modifié par A1 ; aucune clé
      dans le dépôt.

### Estimation (A1)

- **~2,25 jours-homme** (fourchette 1,75 à 3,25) ; complexité / risque **moyen à élevé** (hook
  bloquant sur toutes les sessions). Inconnues : latence réelle (+0,5 j si QC2-B), ordre
  retenu/SUBACK dans rumqttd, `continue:false` dans un sous-agent, outil `Workflow`, cohabitation
  avec perimeter-guard et delegation-guard.

### Ce qui appartient à Stéphane (A1)

- Trancher QC1 à QC9.
- **Installer** le contrôleur (copie dans `~/.claude/` + entrée `PreToolUse` tous outils,
  `timeout: 3600`, **ajoutée** à côté des hooks existants) ; le retirer = supprimer l'entrée.
- Accepter la dette résiduelle RC5.

> **Arbitrage de l'amendement A1 (Stéphane, 27/09/2026, question par question)** : QC1 = A (après le lot MQTT de base) · QC2 = A (état lu sur le bus) · QC3 = A **renforcé** : signature HMAC avec anti-rejeu (horodatage + nonce à usage unique) et identifiant de clé `kid` dans le message signé, pour préparer un MQTT central sur le VPS pilotant plusieurs machines (ce jour-là s'ajouteront TLS, authentification broker et ACL par topic — Mosquitto ; clés partagées via Vaultwarden) · QC4 = A (arrêt sous-agents et session, avec confirmation) · **QC5 = B : délai maximal de pause 10 min**, puis reprise automatique ; **exigence ajoutée** : retour visuel dans l'app de l'attente d'effet — état « pause demandée » (animé) tant que le hook n'a pas pris effet au prochain appel d'outil, puis « paused » avec le temps restant avant reprise automatique · QC6 = A (running/paused) · QC7 = A (capture au gate) · **QC8 = B : le choix du modèle des prochaines délégations d'un persona est inclus dans ce lot** (+~1 j-h) · QC9 = A (script de contrôle séparé, client MQTT partagé).

## Sources

- Code lu le 2026-09-26 (lecture seule) : `IakaAgentsMonitor/src-tauri/src/agents.rs`,
  `src/render.ts`, `src-tauri/Cargo.toml` ; `iakaTokenCounter/iakahub/README.md`,
  `iakahub/rumqttd.toml`, `iakahub/src/broker.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/config.rs`,
  `specs/contrat-mqtt-conso.md` ; `rumqttd-0.19.0/src/router/routing.rs` (l. 1086-1117,
  1221-1225, 1455-1467) et `router/logs.rs` (l. 258-293) ; `~/.claude/settings.json`,
  `~/.claude/delegation-guard.mjs`, `~/.claude/iakaframe-delegations.log` ;
  `~/Library/LaunchAgents/com.naonedge.iakahub.plist` ; `~/work/iakaframe/frames/iakaframe.md`,
  `teams/iakaframe-8.md`, `library/personas/gandalf.md`, skill `iakastart` § 2.
- Hooks Claude Code (événements, champs, `async`) :
  [Claude Code Docs — Hooks](https://code.claude.com/docs/en/hooks).
- `SubagentStart` sans consigne ni modèle :
  [anthropics/claude-code #69545](https://github.com/anthropics/claude-code/issues/69545).
- `SubagentStop` fantômes à `agent_type` vide :
  [anthropics/claude-code #91910](https://github.com/anthropics/claude-code/issues/91910).
- Outil `Agent` (ex-`Task`, alias conservé) :
  [Claude Code Hooks (2026)](https://www.morphllm.com/claude-code-hooks).
- `rumqttc` 0.25.1 (21/11/2025) : [crates.io — rumqttc](https://crates.io/crates/rumqttc).
- Sémantique des messages retenus :
  [HiveMQ — Retained Messages](https://www.hivemq.com/blog/mqtt-essentials-part-8-retained-messages/).
