---
description: Retirer symétriquement ce qui a été ajouté (décomposabilité +/−) — pilote les verbes de retrait de la CLI (skill iakaframe-retrait).
---

Active le parcours de **retrait symétrique** (skill `iakaframe-retrait`) : la fenêtre
conversationnelle sur les gestes `−`, symétriques des ajouts `+`, qui **pilote** les commandes de
retrait de la CLI (source unique — RESTRICT / corbeille / cascade y vivent, **rien** de réimplémenté
ici). Restitue la sortie **verbatim**.

Défaire un ajout **déjà matérialisé** ≠ `reject` d'une proposition en attente (ça, c'est `/learning`) :

- **Détacher / attacher un skill ↔ persona** (cas emblématique) —
  `iakaframe detach <skillId> --persona <id>` et `iakaframe attach <skillId> --persona <id>` :
  mute le **seul** `skills:[]` (Option 1, le « titre du skill » est une vue, jamais une section du
  corps). Offre **détacher ET attacher au même niveau** ; réversible d'un geste.
- **Retirer team/method/binding/skill** — `iakaframe remove <team|method|binding|skill> <id>` :
  **RESTRICT par défaut** (refus + **liste des référents** si encore référencé ; oriente vers le
  `detach` d'un skill référencé) ; **cascade = geste humain explicite** (`--cascade --yes`), jamais
  silencieuse.
- **Retirer une entrée mémoire** — `iakaframe memory remove <profil|registre> "<contenu>"` (réutilisé).

**Non destructif** : tout retrait de fichier/dossier va en **corbeille** `<root>/.trash-<ts>/`
(restaurable). **Confirmation** proportionnée : légère pour `detach`/`attach`, **explicite** pour un
`remove` référencé ou une cascade.

> **Frontière.** **Rejeter** une proposition d'apprentissage encore **en attente**
> (`iakaframe review reject`) relève de `/learning` (skill `iakaframe-learning`) — un renvoi, pas une
> duplication.

$ARGUMENTS
