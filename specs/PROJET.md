# PROJET — iakaFleet

> Espace cadrage (réflexion). Document de vision et de décisions.
> **Aucun code n'est écrit ici** — seulement des décisions et des spécifications.
> Rédigé par Gandalf (P1 — cadrage) le **2026-09-30**, sur ordre de mission d'Odin portant les
> arbitrages du décideur (Stéphane) du même jour.

---

## Vision

**Voir, puis piloter, la flotte des sessions Claude Code** de Stéphane, locales et distantes
(aujourd'hui le Mac et le PC Windows `DESKTOP-DP5CQVK`), depuis n'importe laquelle de ses
machines.

Le déclencheur est l'opacité du **Remote Control** (`/rc`) :
- aucun **accusé de réception** d'un ordre envoyé ;
- une session d'un autre poste n'est visible **que si `/rc` est actif des deux côtés** ;
- on ne sait ni **sur quelle machine** tourne une session, ni son **état réel** ;
- un ordre envoyé d'une session à une autre peut être **refusé par le mode auto** de la session
  cible (blocage légitime constaté : « exploration de credentials »).

Résultat attendu, en une fois lu : **une vue fiable de qui tourne où, dans quel état, sur quel
dépôt/branche**, alimentée par un petit agent par machine qui publie sur MQTT ; puis, dans un
second temps, l'envoi d'ordres **avec accusé de réception**, remis à la session qui les applique
**sous ses propres permissions**.

## Objectifs

- **MVP (lecture seule)** — depuis n'importe quelle machine : liste des machines (en ligne /
  hors ligne, OS, versions), et pour chaque machine ses sessions Claude Code (état
  `idle`/`busy`/…, dépôt, branche, joignable par `/rc` ou non). Une machine qui meurt est vue
  **hors ligne** sans que personne ne l'ait dit (testament MQTT).
- **Ensuite** — envoyer un ordre à une session précise et **savoir** s'il a été reçu, remis,
  accepté ou refusé (accusé de réception traçable).
- **Enfin** — autorisations (qui peut ordonner quoi) et gestes destructifs, sous confirmation
  humaine.

## Périmètre

**Dans le scope (projet) :**
- un **agent de flotte** par machine (publie présence et état sur MQTT) ;
- une **console d'administration** (client MQTT) qui fonctionne **depuis le MQTT seul** ;
- le **contrat sur le fil** (topics + payloads) qui lie les deux ;
- les **gabarits de service** multi-OS (macOS, Windows, Linux).

**Hors scope (pour l'instant) :**
- tout **envoi d'ordre** avant l'instruction dédiée (cf. Backlog) ;
- toute **exécution** d'un ordre par l'outil (règle permanente, cf. Décisions) ;
- la **persistance** longue durée (historique, base) : le broker n'est pas un stockage ;
- une **interface graphique** (widget IakaCockpit, page WSS) : après le MVP ;
- la **supervision de l'hôte** (CPU, RAM, disque).

---

## Stack technique — décision

| Couche | Choix | Raison |
|---|---|---|
| Transport | **MQTT** (3.1.1), broker **paramétrable**, défaut `mqtts://mqtt.naonedge.com:8883` | Arbitrage décideur. Broker Mosquitto 2.1 du VPS déjà en service, TLS + ACL (`naonedge-vps/specs/instructions/mqtt-broker-vps.md`) |
| Agent de flotte | **Node ≥ 20, ESM**, une dépendance applicative : **`mqtt` (MQTT.js)** épinglée | Session MQTT **persistante** (keepalive, reconnexion, QoS 1, testament) : c'est exactement ce qu'on ne réécrit pas à la main. Même runtime que le CLI iakaframe et iakaHub |
| Console | **CLI Node** (même paquet) | Multi-OS sans build d'UI ; une GUI viendra après le MVP |
| Données | **aucune base** : messages `retained` sur le broker | Modèle code/valeur d'iakaTokenCounter : la dernière valeur connue suffit à une vue d'état |
| Hébergement | agent **sur chaque machine** (launchd / tâche planifiée / systemd user) ; broker **sur le VPS** | Aucun centre imposé : la console lit le broker, pas un démon central |

> Rappel méthode : self-hosted/open-source d'abord ; cloud en fallback justifié.

---

## Sources de données / dépendances externes

| Besoin | Source | Quota / coût | Stratégie en dev |
|---|---|---|---|
| État des sessions Claude Code | fichiers `~/.claude/sessions/<pid>.json` écrits par Claude Code (**non documentés officiellement**, cf. instruction MVP `F5`–`F8`) | aucun | jeux de fichiers figés dans `specs/mock/sessions/` |
| Broker MQTT | Mosquitto du VPS NaonEdge | aucun | broker **local** jetable en test (ou client MQTT simulé injecté) |
| Secrets (comptes MQTT) | **Vaultwarden** `work/iakaFleet`, item `iakaFleet/mqtt-<compte>` | — | `.env` local non commité |

---

## Architecture

```
  Machine A (Mac)                    Machine B (PC Windows)            N'importe quelle machine
 +---------------------------+     +---------------------------+      +--------------------------+
 | sessions Claude Code      |     | sessions Claude Code      |      | iakafleet (console CLI)  |
 |  ~/.claude/sessions/*.json|     |  ~/.claude/sessions/*.json|      |  compte fleet-console    |
 |          | lecture seule  |     |          | lecture seule  |      |  lecture seule           |
 | iakafleet-agent           |     | iakafleet-agent           |      +------------+-------------+
 |  compte fleet-<host>      |     |  compte fleet-<host>      |                   | subscribe
 +------------+--------------+     +------------+--------------+                   | iakafleet/#
              | publish retained QoS 1 + testament (LWT)                          |
              v                                  v                                 |
        +------------------------------------------------------------------------------+
        |   Broker MQTT (défaut mqtts://mqtt.naonedge.com:8883, paramétrable)          |
        |   racine iakafleet/<host>/...  — hors de iakaframe/# et de iakaboxlogs/#     |
        +------------------------------------------------------------------------------+

  iakaHub (démon Discord) : AUCUN lien au MVP. Couplage futur optionnel, jamais obligatoire.
```

---

## Backlog des features

Chaque feature reçoit son fichier dans `specs/instructions/` AVANT implémentation.

| Feature | Instruction | État |
|---|---|---|
| **MVP — vue de la flotte en lecture seule** | `specs/instructions/mvp-vue-flotte-lecture-seule.md` | **cadrée, au gate P1→P2** |
| Envoi d'ordres à une session + accusé de réception, trace des ordres | `specs/instructions/ordres-accuse-reception.md` | à cadrer (après le MVP) |
| Autorisations (qui ordonne quoi) + gestes destructifs sous confirmation humaine | `specs/instructions/autorisations-gestes-destructifs.md` | à cadrer (après les ordres) |
| Console graphique (widget IakaCockpit ou page WSS) | `specs/instructions/console-graphique.md` | à cadrer (optionnel) |
| Liaison optionnelle à iakaHub (ex. notifier Discord d'une machine hors ligne) | `specs/instructions/liaison-iakahub.md` | à cadrer (optionnel) |

---

## Décisions structurantes (journal)

> Trace courte des arbitrages importants — le « pourquoi » qui se perd sinon.

- **2026-09-30** — **MQTT** comme transport ; broker central **paramétrable**, celui de NaonEdge
  par défaut (`mqtt.naonedge.com`). *Décideur.*
- **2026-09-30** — **S'appuyer sur l'existant, ne rien réimplémenter** : broker VPS (cadré et
  vérifié, CA-12 « testé par le décideur » encore ouvert), contrat code/valeur d'iakaTokenCounter
  (topic = code, payload `{"v":<scalaire>,"t":<epoch s>}`, retained, QoS 1), main courante
  iakaboxlogs pour la trace future. *Décideur.*
- **2026-09-30** — **Racine de topic à part : `iakafleet/<host>/...`**, pour ne pas être aspirée
  par le pont iakaboxlogs (`iakaboxlogs/#`) ni par le futur pont du VPS (`iakaframe/#`). *Décideur.*
- **2026-09-30** — **MVP = vue en lecture seule d'abord.** Ordres avec accusé ensuite, puis
  autorisations et gestes destructifs. *Décideur.*
- **2026-09-30** — 🛑 **Règle permanente : un ordre reçu est REMIS À LA SESSION Claude, qui
  l'applique sous SES permissions ; il n'est JAMAIS exécuté comme un shell par l'outil.** Sinon
  l'outil devient une porte dérobée et un blanchisseur de permissions (*permission laundering*).
  Conséquence dès le MVP : l'agent de flotte **n'importe pas `child_process`**. *Décideur.*
- **2026-09-30** — **Pas de centre imposé.** La console fonctionne depuis le MQTT seul ; elle
  **peut** se lier à iakaHub pour une feature précise, jamais obligatoirement. iakaHub ne fait que
  passer des messages. L'outil tourne **sans iakabox** (morte depuis le 2026-08-19 ; l'ancien
  broker `192.168.2.11` a un mot de passe perdu). *Décideur.*
- **2026-09-30** — **Sécurité** : TLS, **un identifiant MQTT par machine** (secrets dans
  Vaultwarden, jamais en dur), ACL par topic, trace de ce qui est envoyé. *Décideur.*
- **2026-09-30** — *Proposé par le cadrage, soumis au gate P1→P2* : l'agent de flotte est un
  **paquet autonome dans ce dépôt**, pas une brique ajoutée à iakaHub au MVP (motifs mesurés dans
  l'instruction MVP, `D1`). Si le décideur retient iakaHub comme hôte, seule la § « Où vit
  l'agent » de l'instruction change.
