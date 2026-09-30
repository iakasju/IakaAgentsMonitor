# PROJET — IakaAgentsMonitor

> Espace Cowork (réflexion). Document de vision et de specs.
> **Aucun code n'est écrit ici** — seulement des décisions et des spécifications.

---

## Vision

IakaAgentsMonitor répond à une seule question : **« qui travaille en ce moment, et pour qui ? »**.
C'est un petit moniteur de barre de menus qui montre, d'un coup d'œil, les agents Claude Code en
cours sur le poste — leur persona iakaframe (lettre à la couleur de sa pastille), le projet sur
lequel ils tournent, la tâche qu'on leur a confiée, qui a délégué à qui, et depuis quand. Il est
**autonome** (aucun service, aucun bus, aucun compte), **en lecture seule** sur les fichiers que
Claude Code écrit déjà, et complète iakaTokenCounter sans le recouvrir : l'un mesure ce que
coûtent les agents, l'autre montre ce qu'ils font. D'abord le Mac, ensuite l'IdeaPad, puis, si le
besoin s'en fait sentir, une vue réunissant plusieurs machines.

Première feature : `specs/instructions/liste-des-agents.md` (validée le 26/09/2026). Maquettes : `docs/maquettes/visu-agents-5-versions.html` (version 1 retenue).

> **Remonté d'un cran le 2026-09-30 (décision du décideur, fusion avec iakaFleet)** : le Monitor
> garde sa vue locale et **voit en plus les autres machines et leurs sessions** par le broker MQTT
> du VPS (`mqtt.naonedge.com`, paramétrable). Il devient **le produit** ; le démon de machine et la
> CLI d'iakaFleet y entrent sous `fleet/`. Il **marche toujours seul** : aucun broker n'est
> obligatoire. Convention de bus unique : `specs/contrat-bus-iaka.md`. Instruction :
> `specs/instructions/fusion-flotte-vue-distante.md`.

## Objectifs

- <!-- objectif mesurable 1 -->
- <!-- objectif mesurable 2 -->

## Périmètre

**Dans le scope :**
- <!-- ... -->

**Hors scope (pour l'instant) :**
- <!-- ... -->

---

## Stack technique — décision

| Couche | Choix | Raison |
|---|---|---|
| Frontend | <!-- --> | <!-- --> |
| Backend | <!-- --> | <!-- --> |
| Données | <!-- --> | <!-- --> |
| Hébergement | <!-- --> | <!-- --> |

> Rappel méthode : self-hosted/open-source d'abord ; cloud en fallback justifié.

---

## Sources de données / dépendances externes

| Besoin | Source | Quota / coût | Stratégie en dev |
|---|---|---|---|
| <!-- --> | <!-- --> | <!-- --> | mock dans `specs/mock/` |

---

## Architecture

```
<!-- schéma ASCII des composants et de leurs échanges -->
```

---

## Backlog des features

Chaque feature reçoit son fichier dans `specs/instructions/` AVANT implémentation.

| Feature | Instruction | État |
|---|---|---|
| Liste des agents (A1–A7) | `specs/instructions/liste-des-agents.md` | validée 26/09 |
| Temps restant | `specs/instructions/temps-restant.md` | validée |
| Agnostique du runner par MQTT local | `specs/instructions/agnostique-mqtt.md` | lot de base codé (`a27a3c8`) ; A1 arbitré 27/09, non codé |
| **Fusion iakaFleet + vue distante (lecture seule)** | `specs/instructions/fusion-flotte-vue-distante.md` | **validée 30/09 (P1→P2 franchi), en attente de P2** |
| Ordres (point d'entrée `iaka/<node>/in/…`, converge A1 + Lot 6 BACKLOG + ordres iakaFleet) | à écrire | après la fusion |

---

## Décisions structurantes (journal)

> Trace courte des arbitrages importants — le « pourquoi » qui se perd sinon.

- **2026-09-30** — **Fusion iakaFleet → IakaAgentsMonitor** : « agentsmonitor remonte d'un cran et
  regarde les autres machines/sessions en distance avec le mqtt vers le mqtt de naonedge vps ».
  *Décideur, non rouvrable.*
- **2026-09-30** — **Cadrage de fusion VALIDÉ** : **un seul dépôt** (IakaAgentsMonitor) à trois
  pièces — app, publieur de hooks, `fleet/` (démon de machine + CLI, importé avec son historique).
  Convention de bus **unique** `iaka/<node>/{out,in}/…` sur le bus local comme sur le central. *Décideur.*
- **2026-09-30** — **Texte de tâche MASQUÉ PAR DÉFAUT sur le central**, activable machine par machine
  (même mécanisme que le masquage des noms de session, défaut inversé). *Décideur (Q1).*
- **2026-09-30** — **Groupement par machine MINIMAL** dans le tableau V4 ; pas de maquettes Loki, à
  rouvrir seulement si le tableau devient illisible avec deux machines. *Décideur (Q2).*
- **2026-09-30** — **Hooks du publieur + démon installés APRÈS le Lot 0 amendé**, Mac d'abord, PC
  quand le lien sera rétabli : lot explicite de la recette. *Décideur (Q3).*
- **2026-09-30** — Dépôt **iakaFleet archivé après l'import** (geste du décideur) ; **noms `iakafleet`
  conservés** (CLI, `~/.iakafleet/.env`, Vaultwarden `work/iakaFleet`). *Décideur (Q4).*
- **2026-09-30** — Règles héritées d'iakaFleet, gravées ici : ordre **remis à la session**, jamais
  exécuté comme un shell ; TLS + un identifiant MQTT par machine + ACL par branche ; secrets dans
  Vaultwarden ; confidentialité (pas d'identifiant de session ni de socket vers le central, masquage
  facultatif des noms) ; aucun centre obligatoire ; CLI d'abord, puis GUI (= le Monitor).
- **2026-09-30** — **« Ce poste » = toutes les sessions et tous les agents de la machine**, lus par l'app
  elle-même sur les fichiers du démon (`~/.claude/sessions`, `~/.cache/iaka-agents`) ; la **bascule stricte
  Q3-A** (bus **ou** fichiers) est **remplacée** par une union ; l'ancienne détection par transcripts reste
  en **repli par session** ; budget CPU au repos ≤ 1 % moyen, aucun pic > 10 % (build de publication).
  *Décideur, recommandations acceptées* — `specs/instructions/ce-poste-vue-locale-lisibilite.md`.
- **2026-09-30** — **Lisibilité** : direction **A « Tableau calme »** de Loki + **phrase d'état** de C en tête
  de « Ce poste » ; **coloris actuels conservés, seulement un peu accentués** (aucune nouvelle teinte) ;
  plancher **14 px courant / 13 px secondaire**, rien sous 12 px ; **trois crans 14 / 16 / 18 px** dans la barre
  du haut, **défaut 16 px** (plancher valant pour le cran 14) ; à 480 px **deux lignes par agent**, runner et modèle **visibles** (amende A4 point 1) ; bandeaux de
  dépôt A7 retouchés dans la même limite. *Décideur : « A mes en gardant les coloris actuelles juste un peu
  accentués. reco ok ».* Maquettes : `~/work/iakagraph/etudes/IakaAgentsMonitor/lisibilite/`.
