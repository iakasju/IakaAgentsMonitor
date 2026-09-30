# IakaAgentsMonitor

Moniteur de barre de menus (macOS) : **qui travaille en ce moment, sur quel projet, pour qui** - les
agents Claude Code de ce poste (persona, tache, delegation, modele) et, si un broker central est
configure, **ceux des autres machines** (lecture seule). Il marche **seul** : aucun broker n'est obligatoire.

Trois pieces, un contrat de bus unique (`specs/contrat-bus-iaka.md`) :

| Piece | Ou | Role |
|---|---|---|
| App (Tauri) | `src-tauri/`, `src/` | vue locale « Ce poste » + vue distante « Autres machines » |
| Publieur de hooks | `publishers/claude-code/` | fiches d'agents, par hooks Claude Code |
| Demon de machine + CLI | `fleet/` | presence, sessions, relais des fiches vers le central ; CLI `iakafleet` |

Vecteurs de contrat partages : `contract-vectors/`. Vision : `specs/PROJET.md`. Instruction de fusion :
`specs/instructions/fusion-flotte-vue-distante.md`. Commandes : `CLAUDE.md`. Installation du demon et de
la CLI : `fleet/docs/installation.md`.
