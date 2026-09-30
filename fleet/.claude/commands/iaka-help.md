---
description: Carte des commandes : slash-commands du kit + verbes CLI + skills, avec description — inventaire vivant, jamais figé.
---

Affiche une **carte à jour** des commandes disponibles. **N'énumère RIEN de mémoire** — à chaque
appel, interroge les sources autoritatives :

1. **Slash-commands** : liste les fichiers `*.md` de `.claude/commands/` (projet) ET de
   `~/.claude/commands/` (global) ; pour chacun, affiche `/${nom}` + le champ `description` de son
   frontmatter.
2. **Verbes CLI** : `iakaframe --help` → extrais les commandes + leur ligne d'aide.
3. **Skills** : `iakaframe list skills` → liste les skills de la bibliothèque.

Rends une **arborescence** en 3 sections (Slash-commands / Skills / CLI), triée, une description par
entrée. Si `$ARGUMENTS` est fourni, **filtre** les entrées correspondantes.

$ARGUMENTS
