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
| <!-- --> | `specs/instructions/feature-xxx.md` | à spécifier |

---

## Décisions structurantes (journal)

> Trace courte des arbitrages importants — le « pourquoi » qui se perd sinon.

- **AAAA-MM-JJ** — <!-- décision + raison -->
