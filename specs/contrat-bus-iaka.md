# Contrat du bus `iaka/` — convention de topics unique du portefeuille (v2)

> Rédigé par Gandalf (P1 — cadrage) le **2026-09-30**, sur décision du décideur de **fusionner
> iakaFleet dans IakaAgentsMonitor**. **Source unique** de la convention : tout producteur ou
> consommateur (démon de machine, publieur de hooks, app Monitor, CLI, futurs clients) s'y réfère ;
> aucun autre document ne redéfinit un topic `iaka/`.
>
> **Remplace** : `iaka/agents/out/<runner>/<sessionId>/<agentId>` (Monitor, contrat v1,
> `specs/instructions/agnostique-mqtt.md`) et `iakafleet/<host>/…` (iakaFleet, contrat v1,
> `~/work/iakaFleet/specs/instructions/mvp-vue-flotte-lecture-seule.md` § D4).
> **Réconcilie** : la topologie `/<node>/<depot-projet>/<agent>/…` du Lot 2 de
> `~/work/BACKLOG.md` (« Bus MQTT portefeuille sur le VPS »).

---

## 1. Arbre

```
iaka/<node>/out/presence/<code>/current                 machine : état, versions, battement  {v,t}
iaka/<node>/out/sessions/<pid>/<code>/current            session Claude Code (fichier de session) {v,t}
iaka/<node>/out/agents/<runner>/<sid>/<agentId>          fiche d'agent (JSON v1 du Monitor)
iaka/<node>/out/events/…                                 RÉSERVÉ — flux d'événements (Lot 5 / télémétrie)
iaka/<node>/in/agents/<runner>/<sid>/<agentId>           RÉSERVÉ — ordres (instruction suivante, point d'entrée unique)
iaka/<node>/in/…                                         RÉSERVÉ
```

| Segment | Règle |
|---|---|
| `iaka` | racine du portefeuille. **Disjointe** de `iakaframe/#` (compte `iakaframe-publisher` et futur pont du VPS), `iakaboxlogs/#` (pont CouchDB) et `iakatokencounter/#` : rien n'y est aspiré par accident |
| `<node>` | identifiant de machine = **le « `<host>` » d'iakaFleet** : `IAKA_NODE` si défini, sinon `os.hostname()` en minuscules, suffixe `.local` retiré, tout caractère hors `[a-z0-9-]` remplacé par `-` ; vide ou contenant `+ # /` ⇒ refus. **Même fonction** dans les trois langages/lieux (démon Node, publieur de hooks, app Rust), prouvée par des **vecteurs de test partagés** (§ 6) |
| `out` / `in` | **sens** : `out` = ce que la machine émet (elle seule y écrit) ; `in` = ce qu'on lui adresse. C'est ce segment qui porte l'ACL |
| `<pid>` | pid de la session Claude Code (clé stable d'une session vivante ; `sessionId` est faux après `/clear`) |
| `<runner>`, `<agentId>` | inchangés du contrat v1 du Monitor (`[A-Za-z0-9._-]{1,128}`, `main` = coordinateur) |
| `<sid>` | **localement** (bus de la machine) : le `sessionId` brut. **Sur un broker central** : `sha256(sessionId)` en hexadécimal, **12 premiers caractères** — un identifiant de session ne quitte jamais la machine (décision de confidentialité du 2026-09-30) |

**Pourquoi le projet n'est PAS un segment de topic** (écart assumé avec le Lot 2) : une session change
de projet quand son `cwd` change ; un topic retenu doit être une **clé stable**, sinon chaque
changement de dossier laisse un retenu orphelin à effacer. Le projet est un **champ** (fiche `project`,
session `repo`) ; le filtrage par projet se fait chez le lecteur. Le Lot 2 est satisfait sur son
intention (**la machine en premier**), pas sur sa lettre. **Pas de `/` initial** : un topic qui commence
par `/` crée un premier niveau vide, source classique d'erreurs d'ACL.

## 2. Payloads

- **`presence/` et `sessions/`** : modèle **code/valeur** d'iakaTokenCounter —
  `{"v":<scalaire|null>,"t":<epoch s>}`, jamais d'objet ni de tableau ; **retain, QoS 1** ;
  effacement = **payload vide 0 octet, retained**. Codes : ceux d'iakaFleet MVP `D4` (`meta/*` devient
  `presence/*`, noms de codes inchangés), plus le masquage (§ 4) et deux codes neufs :
  `presence/cards_source_ok/current` (boolean : la source locale des fiches d'agents existe-t-elle ?
  **`false` écrit, jamais sous-entendu** — sans lui, « pas de hooks installés » et « aucun agent »
  seraient le même écran vide) et `sessions/<pid>/sid/current` (`sha256(sessionId)` sur 12 hex, § 1 : la
  **clé de rattachement** d'une fiche d'agent à sa session ; ajout compatible, v2 inchangée, § 7 —
  **ajouté en P2 par Gimli** : l'instruction D4 exige ce rattachement sans le nommer dans les codes).
- **`agents/`** : **exception déclarée** au modèle code/valeur — la **fiche d'agent JSON v1** du Monitor
  (`agnostique-mqtt.md` § « Fiche »), inchangée champ pour champ, **retain, QoS 1**, effacement = payload
  vide. Motif : une fiche est un **document** cohérent (persona, tâche, parent, modèle) que le Monitor
  valide en bloc ; la découper en 15 codes réécrirait `card.rs` et le publieur pour aucun gain de
  lecture. Règle de cohérence topic ↔ fiche : `sessionId` de la fiche **égal** au segment `<sid>`
  (donc haché, lui aussi, sur un broker central).

## 3. Propriété des branches et ACL (broker central)

| Branche | Écrit par | Lu par |
|---|---|---|
| `iaka/<node>/out/#` | **le seul démon de `<node>`** (compte `fleet-<node>`) | le démon de `<node>` (purge de ses anciens retenus) ; tout lecteur de flotte |
| `iaka/<node>/in/#` | *(instruction « ordres », pas avant)* | *(le démon de `<node>`, pas avant)* |

Comptes MQTT (Mosquitto, lignes explicites, pas de `pattern %u`) :

```
user fleet-console                       # lecteur pur (CLI/Monitor sur une machine sans démon)
topic read iaka/+/out/#

user fleet-<node>                        # un compte par machine
topic readwrite iaka/<node>/out/#        # readwrite : purge des retenus + testament (#3552)
topic read iaka/+/out/#                  # l'app Monitor de CETTE machine lit toute la flotte
```

Aucun compte n'a de droit sur `iaka/+/in/#` tant que l'instruction « ordres » n'est pas validée.

## 4. Confidentialité (décision du 2026-09-30)

Sur un broker **central**, jamais : `sessionId` en clair (haché, § 1), `bridgeSessionId`,
`messagingSocketPath`, chemin absolu (`cwd` ramené au home : `~/work/x`), contenu de conversation,
consigne de délégation. Deux réglages **par machine** (décideur, 2026-09-30) :

| Réglage | Défaut | Effet vers le central |
|---|---|---|
| `IAKA_MASK_NAMES=1` | **désactivé** (noms publiés) | le code `name` des sessions est publié `null` |
| `IAKA_SHOW_TASKS=1` | **désactivé** (tâches **masquées**) | le champ `task` des fiches est publié **tel quel** ; sans lui, `task` = `null` |

Le bus **local** (même machine) n'est pas concerné.

## 5. Brokers

| Broker | Rôle | Transport |
|---|---|---|
| **Central** — paramétrable, défaut `mqtts://mqtt.naonedge.com:8883` | vue **distante** multi-machines | TLS obligatoire, certificat vérifié, un compte par machine |
| **Local** — `127.0.0.1:1883` (iakahub d'iakaTokenCounter), facultatif | vue **locale** temps réel du Monitor | boucle locale, anonyme (dette R7 du Monitor, inchangée) |

Même arbre `iaka/<node>/…` sur les deux. **Aucun n'est obligatoire** : sans central, chaque Monitor
voit son poste ; sans local, le Monitor lit les fichiers ; le démon de machine publie vers le central
**sans passer par le bus local** (il lit les fichiers, § instruction de fusion `D3`).

## 6. Vecteurs de test partagés

`contract-vectors/` (racine du dépôt) : JSON de cas → résultats attendus pour (a) la normalisation de
`<node>`, (b) le hachage de `<sid>`, (c) la construction des topics, (d) la règle d'état de machine
(en ligne / muette / hors ligne / inconnue / sessions illisibles). **Le démon Node, le publieur de
hooks et l'app Rust passent tous les trois ces vecteurs** — c'est ce qui rend la convention unique
vérifiable, et non déclarative.

## 7. Versionnage

Ce contrat est la **v2** du bus. Un changement de forme = nouvelle version documentée ici. Ajout de
code ou de champ = compatible, sans changement de version.
