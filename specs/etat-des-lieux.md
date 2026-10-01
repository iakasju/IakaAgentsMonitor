# Etat des lieux - IakaAgentsMonitor

> Genere par iakaframe (CLI) le 2026-10-01 02:38 (motif: pause).
> A regenerer a chaque changement de version et a chaque pause/reprise.

## Etat courant

| Champ | Valeur |
|---|---|
| Version | v0.1.0 |
| Branche | feat/fusion-flotte |
| Dernier commit | 444518c docs(specs): recette du 2026-10-01 consignee (sans cocher) + lot de finition Ce poste |
| Arbre | propre |
| Fichiers (suivis + non ignores) | 196 |
| Note | Fusion flotte : Lot 0, demon, L-INSTALL Mac, vue Ce poste + lisibilite livres et gates PASS ; budget CPU tenu ; reste PC (/rc), finition, fusion main |

## Commits recents

| Hash | Date | Sujet |
|---|---|---|
| `444518c` | 2026-10-01 | docs(specs): recette du 2026-10-01 consignee (sans cocher) + lot de finition Ce poste |
| `ef3ed43` | 2026-09-30 | docs(specs): cran par defaut 16 px (decision du decideur) |
| `5fa5b69` | 2026-09-30 | docs(claude): CLAUDE.md -- Ce poste lu sur le disque, union D2, scripts de test de styles, trace CPU |
| `4375d2e` | 2026-09-30 | docs(specs): renvois d'amendement en tete d'agnostique-mqtt.md (Q3-A) et liste-des-agents.md (A4 point 1) |
| `67a6999` | 2026-09-30 | feat(monitor): deux lignes par agent, modele visible en compact (D6.5, amende A4 point 1) |
| `0873734` | 2026-09-30 | feat(styles): echelle --sc-fs-* et trois crans 14/16/18 px dans la barre du haut (D6.1-D6.3) |
| `d0df40a` | 2026-09-30 | feat(styles): accentuation des coloris (D8) -- test:contrast VERT (78 couples, plus faible 4,70:1) |
| `8142e67` | 2026-09-30 | test(styles): test de contraste WCAG et garde des tailles, vus ROUGES sur les encres et tailles actuelles |
| `2729931` | 2026-09-30 | feat(monitor): trace de duree de tick (IAKAAM_TRACE=1), micro-banc M1 et protocole de mesure CPU (D7) |
| `f83dcf3` | 2026-09-30 | feat(monitor): titre « Ce poste », phrase d'etat, etat du demon, message « Autres machines » vide (D4, D5) |

## Reprise du travail

- **Ce qui vient d'etre fait** (2026-09-30 -> 2026-10-01, branche `feat/fusion-flotte`, PASS Legolas a chaque lot, poussee, NON fusionnee dans main) :
  fusion d'iakaFleet dans IakaAgentsMonitor (`fleet/` = demon + CLI `iakafleet` importes par subtree) ; contrat de topics unique `iaka/<node>/{out,in}` ;
  Lot 0 du broker VPS (ACL + comptes `fleet-*`, rejeu avec secrets 0 FAIL, `naonedge-vps` v0.9.0) ; demon sous `node --permission` (G-EXEC-2) ;
  correctif de la duree hors ligne (`max(online.t, last_tick_at)`, verifie en reel par gel SIGSTOP) ; L-INSTALL Mac (service launchd + 7 hooks) ;
  lot « Ce poste, vue locale et lisibilite » (direction A de Loki, 16 px par defaut, crans 14/16/18, coloris seulement accentues) valide a l'ecran ;
  budget CPU du build de publication TENU (0,13 % moyen mesure avec `top`, tick p95 ~5 ms ; le verdict NON TENU du script `measure-cpu.sh` etait un artefact de quantification de `ps cputime`).
- **En cours / a reprendre** : rien en fabrication ; aucun agent lance. Instruction a fabriquer : `specs/instructions/finition-ce-poste.md` (~0,3 j-h).
- **Prochaine etape concrete** : retablir le lien avec le PC Windows (`/rc` a taper des DEUX cotes, meme compte), puis remotes -> VPS, Node a mesurer, `L-INSTALL (b)` ; en parallele, lot de finition
  (dont reecrire `scripts/measure-cpu.sh` avec `top`). Decisions du decideur : fusion de `feat/fusion-flotte` dans main apres recette, archivage d'iakaFleet.
- **Pieges connus** : `launchd` relance le demon en ~1 s (un `kill -9` ne laisse « hors ligne » qu'un instant : observer par abonne prealable ou par gel SIGSTOP) ; `!` du prompt n'a pas de tty pour le mot de passe maitre
  (lanceurs `.command` ouverts par `! open`) ; fermer la fenetre Terminal d'un `tauri dev` tue l'app ; le classifieur du mode auto bloque le lancement d'un agent « Production Deploy » ;
  ne jamais lister des processus avec `pgrep -fl`/`ps` complet (l'environnement apparait : un `IAKALOG_PASS` a ete expose, rotation a faire) ; disque ~8,6 Go libres (builds Rust lourds).

## Journal (versions & pauses)

| Date | Motif | Version | Branche | Note |
|---|---|---|---|---|
| 2026-10-01 02:38 | pause | v0.1.0 | feat/fusion-flotte | Fusion flotte : Lot 0, demon, L-INSTALL Mac, vue Ce poste + lisibilite livres et gates PASS ; budget CPU tenu ; reste PC (/rc), finition, fusion main |
| 2026-09-26 13:54 | version | v0.1.0 | main | onboarding initial |
