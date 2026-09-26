# Instruction : Temps restant — barre qui se colore selon l'estimation annoncée par le délégant

> Rédigé par le cadrage (P1, Gandalf) le 2026-09-26. Consommé par Loki (maquette, étape 0) puis
> Gimli (P2).
> **VALIDÉE par Stéphane le 26/09/2026** (« A partout ») : Q1 à Q7 = A. Reste l'étape 0 (validation de la maquette de Loki) avant tout code. Suite décidée le même jour : lot « agnostique via MQTT » (la couche source par outil deviendra l'adaptateur Claude Code publiant sur le bus local).
> Statut : **PROPOSÉE — en attente d'arbitrage de Stéphane** (Q1 à Q7 ci-dessous). Rien n'est à
> coder avant le « go » **et** la validation de la maquette de Loki (étape 0).
> Besoin du décideur (26/09) : afficher, dans le tableau V4, un **temps restant** sous forme de
> **barre qui se colore peu à peu** selon le temps prévu. Source choisie par Stéphane :
> **option 3 — une estimation annoncée au lancement par l'agent qui délègue** (pas d'historique,
> pas de liste de tâches).
> **Contrainte d'agnosticisme (Stéphane, 26/09, relayée par le coordinateur)** : la convention doit
> être **neutre vis-à-vis du runner** (Claude Code, Codex, Ollama…) et ne pas dépendre d'un champ
> propre à Claude Code comme `description`. Elle repose donc sur **une ligne normalisée dans le
> texte de la consigne** transmise au sous-agent (que tout runner reçoit), éventuellement doublée
> d'un suffixe dans le libellé court quand le runner en offre un. Dans l'app, la lecture passe par
> une **couche « source par runner »** (Claude Code seul aujourd'hui), jamais codée en dur dans le
> cœur.
> Dépend de : `specs/instructions/liste-des-agents.md` (amendements A1 à A3), livré.

---

## Problème

Le tableau dit **qui** travaille et **depuis quand**, mais pas **combien de temps il reste** :
impossible de savoir si un agent qui tourne depuis 12 minutes est à mi-course ou nettement en
retard. Aucun runner ne fournit cette durée prévue ; elle doit être **annoncée par celui qui
délègue**, au moment où il délègue, sous une forme que l'app sait lire quel que soit le runner.
Deux sujets distincts en découlent : une **convention de méthode** (comment on annonce) et un
**affichage** (comment l'app le montre).

## Ce qui existe (vérifié en lecture le 2026-09-26)

| Élément | Constat |
|---|---|
| Compagnon `agent-<id>.meta.json` (Claude Code) | 486 fichiers réels relevés sous `~/.claude/projects/*/*/subagents/`. Champs vus : `agentType`, `description`, `toolUseId`, `spawnDepth` (toujours) ; `parentAgentId` (profondeur ≥ 2) ; `requestShape`, `requestNonInteractive`, `model`, `stoppedByUser` (parfois). **Aucun champ de durée, aucune date.** Exemple (cet agent-ci) : `{"agentType":"gandalf","description":"Cadrer barre de temps restant","toolUseId":"toolu_01Max…","spawnDepth":1,…}` |
| `description` (Claude Code seul) | Longueur réelle 3 à 8 mots. **Une trentaine se terminent déjà par une parenthèse** : `Legolas passe 2 sur 3 (relance)`, `Gate qualité lot 6 (6·1, 6·3)`, `Cadrage lot 1c (ratification clavier + instruction socle)`… **Aucune** des 486 ne contient `~` ni un nombre suivi de `s`/`min`/`h`. |
| Consigne du sous-agent (Claude Code) | Transcript `agent-<id>.jsonl`, **ligne 1** = `"type":"user"`, `message.content` = **la consigne complète, en chaîne**, avec `timestamp` ISO. Consignes de 2 à plusieurs dizaines de Kio ; les lignes suivantes injectent des skills (`isMeta`, blocs commençant par `<command-message>`). |
| Code de l'app | `src-tauri/src/agents.rs` lit déjà chaque compagnon à chaque tick (`read_agent_meta`, l. 196) et expose `description` comme **tâche** (`build_children`, l. 727 et 762). Il lit **déjà la tête bornée (1 Mio) du transcript de chaque sous-agent vivant** à chaque tick pour le modèle (`model_label_for_transcript`, l. 361, `TRANSCRIPT_HEAD_CAP` l. 46) et sait extraire le texte d'un enregistrement utilisateur, chaîne ou blocs (`extract_user_text`, l. 377), en écartant les balises de service (`is_service_tag`, l. 398). « Démarré il y a » vient de la date de création du transcript (`created_of`, l. 222). Tick 5 s ; l'instantané change déjà à chaque tick. |
| Rendu | `src/render.ts` : colonnes Agent · Projet · Runner · Modèle · Tâche · Démarré · Actif · **Statut** (l. 140-149) ; Statut affiche toujours « vivant » (constante, non triable, l. 49-58). Runner = constante « Claude Code » (l. 198). |
| Styles | `src/styles.css` porte les jetons Studio clair `--sc-bg`… `--sc-accent-2` (clair l. 30-41, sombre l. 49-70) mais **pas** `--sc-ok` / `--sc-warn` / `--sc-danger`, présents dans la maquette (`docs/maquettes/visu-agents-5-versions.html`, l. 19 et 31). |
| Dépendances Rust | `tauri`, `serde`, `serde_json` seulement : **pas de crate `regex`** — le lecteur sera écrit à la main (grammaire minuscule). |

## Questions fermées (arbitrage de Stéphane)

**Q1 — Où le délégant écrit-il l'estimation ?**
- **A (recommandé, conforme à la contrainte d'agnosticisme) — Ligne normalisée en tête de la
  consigne, doublée du suffixe dans le libellé court quand le runner en a un.**
  Ligne : `Durée estimée : ~10 min` en **première ligne** de la consigne. Tout runner transmet une
  consigne texte à son sous-agent : la règle ne dépend d'aucun champ propre. Sous Claude Code, le
  délégant ajoute **aussi** `(~10 min)` à la fin de `description` : l'estimation devient visible
  dans le terminal, et sert de repli si la ligne manque. Bonus : le sous-agent connaît son budget.
  Coût côté app : nul en lecture disque (la tête du transcript est déjà lue pour le modèle).
- B — Suffixe dans `description` seul : le plus simple et le plus robuste, mais **propre à Claude
  Code** — écarté par la contrainte d'agnosticisme.
- C — Ligne dans la consigne seule, sans suffixe : neutre aussi, mais l'estimation disparaît du
  terminal de Claude Code et il n'y a plus de repli.

**Q2 — Fourchettes acceptées ?**
- **A (recommandé) — Oui**, `~10-15 min` : les modèles annoncent volontiers une fourchette ; la
  refuser ferait perdre l'estimation. Sens : **100 % = borne haute**, passage à l'orange **à la
  borne basse** (au lieu de 75 %).
- B — Valeur unique seulement : une fourchette donne un tiret.

**Q3 — Où placer la barre ?**
- **A (recommandé) — Remplacer la colonne Statut** par une colonne **« Reste »** : Statut vaut
  toujours « vivant » (un agent mort n'est jamais affiché), elle n'apporte rien ; la fenêtre de
  520 px ne supporte pas une 9e colonne. La cellule montre la barre + un texte court (`~4 min`,
  `+3 min`) ; la phrase complète (« ~4 min restantes », « +3 min de dépassement ») va dans
  l'infobulle et dans le libellé accessible. *Amende l'ordre de colonnes fixé par A3.*
- B — Ajouter une 9e colonne en gardant Statut : tableau trop large, défilement horizontal.
- C — Barre sous le texte de la tâche, dans la cellule Tâche : ligne plus haute, tri impossible.

**Q4 — Couleurs progressives ?**
- **A (recommandé) — Trois paliers nets** : vert (`--sc-ok`) sous 75 % (ou sous la borne basse),
  orange (`--sc-warn`) de 75 % à 100 % inclus, rouge (`--sc-danger`) au-delà. Lisible d'un coup
  d'œil, contraste vérifiable.
- B — Dégradé continu vert → rouge : joli, mais contraste non garanti sur les teintes
  intermédiaires et aucune frontière nette à lire.

**Q5 — Dépassement ?**
- **A (recommandé)** — Barre **pleine et rouge**, texte `+3 min` (en `--sc-danger`) : l'ampleur
  est portée par le texte.
- B — Barre qui déborde / seconde barre proportionnelle au dépassement : plus de code, peu lisible
  à 56 px.

**Q6 — Colonne triable ?**
- **A (recommandé)** — Oui, sur le temps restant en secondes (dépassement = valeur négative, donc
  en tête en tri croissant : « le plus en retard d'abord ») ; sans estimation = en dernier.
- B — Non triable.

**Q7 — Ordre des lots ?**
- **A (recommandé)** — Le **lot app** (cette instruction) peut partir dès le « go » et la maquette
  validée : sans estimation l'app affiche un tiret, rien ne casse. Le **lot méthode** (convention
  dans iakaframe) est cadré **à part, dans le dépôt iakaframe**, après validation de la
  convention par Stéphane. D'ici là, un délégant peut écrire la ligne à la main pour la recette.
- B — Attendre le lot méthode avant de coder l'app.

## Décision retenue (sous réserve de Q1 à Q7, valeurs recommandées)

**Convention (neutre)** : le délégant écrit, en **première ligne de la consigne**, une ligne
`Durée estimée : ~<valeur>`. **Complément propre au runner** : si le runner offre un libellé court
de délégation (Claude Code : `description`), le délégant y ajoute aussi `(~<valeur>)` en fin.

**App** : un **cœur neutre** (grammaire, calcul, mise en forme — aucune connaissance d'un runner)
reçoit, pour chaque sous-agent, deux textes fournis par une **source par runner** : la consigne et,
s'il existe, le libellé court. La seule source livrée est celle de Claude Code. Le cœur retient la
ligne de la consigne en priorité, le suffixe du libellé en repli, retire le suffixe du texte de la
tâche, calcule en Rust (D9) la part écoulée et peint une barre dans une colonne « Reste » qui
remplace « Statut ».

### Grammaire fermée de l'estimation

**Valeur** (commune aux deux formes) :

```
valeur := "~"? N unité | "~"? N sep M unité
N, M   := entier décimal ASCII, 1 à 3 chiffres, sans décimale ("90 min", pas "1.5 h") ; N < M
sep    := "-" (trait d'union) | "–" (tiret demi-cadratin), blancs facultatifs autour
unité  := "s" | "min" | "h"   (minuscules, rien d'autre : ni "mn", ni "mins", ni "heure")
          blanc facultatif entre le nombre et l'unité ; une seule unité pour les deux bornes
borne  : total (borne haute) entre 10 s et 12 h inclus
```

**Forme 1 — ligne de consigne (neutre, prioritaire).** Écrite en **première ligne** :
`Durée estimée : ~10 min` · `Durée estimée : ~30-45 min`.
Lue : la **première ligne non vide** de la consigne qui commence par l'étiquette, **cherchée parmi
les 5 premières lignes non vides** (tolérance à un titre ou à un badge placé avant) ; étiquette
`Durée estimée` ou `Duree estimee`, casse indifférente ; `:` avec blancs facultatifs ; puis la
valeur, puis fin de ligne (blancs finaux tolérés). Le tilde y est **facultatif** : l'étiquette
suffit à lever l'ambiguïté.

**Forme 2 — suffixe du libellé court (complément, repli).** Écrit : `<libellé> (~10 min)`.
Lu : toute fin du libellé (blancs finaux ôtés), précédée d'au moins un blanc ou constituant le
libellé entier ; parenthèses, **tilde obligatoire** (il écarte les parenthèses déjà en usage :
`(relance)`, `(Lot 4)`, `(6·1, 6·3)`) ; blancs facultatifs à l'intérieur. On n'examine que les
40 derniers caractères.

**Règles communes** : tout écart ⇒ **pas d'estimation** par cette forme ; une forme invalide ne
bloque pas l'autre ; si les deux sont valides et diffèrent, **la ligne de consigne l'emporte**. Le
suffixe valide est **toujours retiré** du texte de la tâche, même quand la ligne l'emporte ; un
suffixe invalide laisse le libellé **intact**.

### Couche « source par runner »

- **Contrat** (neutre) : pour un sous-agent, une source fournit
  `DelegationText { consigne: Option<String>, short_label: Option<String> }` et une date de
  début (`Option<i64>`). Le cœur `estimate.rs` ne connaît que ce contrat.
- **Source Claude Code** (seule livrée, dans `agents.rs` qui est déjà le module propre à Claude
  Code) : `consigne` = texte du **premier enregistrement `"type":"user"` non balise de service**
  de la tête bornée du transcript du sous-agent (réemploi d'`extract_user_text` et
  `is_service_tag`) ; `short_label` = `description` du compagnon ; début = création du transcript.
- **Ajouter un runner** (Codex, Ollama… hors de ce lot) = écrire une nouvelle source qui remplit
  ce contrat, **sans toucher au cœur**.

### Décisions arbitraires (mineures, révisables en une phrase)

- **DA-T1** Base du temps écoulé = **date de début fournie par la source** (Claude Code : création
  du transcript, même source que « Démarré il y a ») ; indisponible ⇒ pas de barre.
- **DA-T2** Texte court : restant ≥ 60 s ⇒ `~N min` arrondi **au supérieur** ; < 60 s ⇒ `< 1 min` ;
  ≥ 60 min ⇒ `~1 h 5 min` (même forme que « Démarré il y a ») ; dépassement ⇒ `+N min` arrondi au
  supérieur (donc `+1 min` dès la première seconde). Phrase longue : « ~4 min restantes »,
  « moins d'1 min restante », « +3 min de dépassement ».
- **DA-T3** Seuil orange = 75 % pour une valeur unique ; = borne basse pour une fourchette.
  100 % pile = orange ; rouge strictement au-delà.
- **DA-T4** Infobulle de la ligne enrichie de ` — estimé ~10 min, reste ~4 min` (ou
  `… , dépassement +3 min`).
- **DA-T5** Barre : 56 × 6 px, coins arrondis, piste `--sc-well` bordée `--sc-border`, largeur
  remplie = min(écoulé / estimé, 100 %). Texte de la cellule en police mono, chiffres tabulaires,
  `--sc-ink-muted` (vert/orange) ou `--sc-danger` (dépassement).
- **DA-T6** Session principale (Odin), ligne de débordement `+N`, agent sans estimation ⇒ cellule
  **`—`**, pas de barre.
- **DA-T7** La tête du transcript d'un sous-agent est lue **une seule fois par tick** et sert au
  modèle **et** à la consigne (pas de seconde lecture de 1 Mio). Consigne plus longue que la borne
  ⇒ enregistrement tronqué, JSON invalide ⇒ pas de ligne ⇒ repli sur le suffixe.

## Périmètre

- **Inclus** : maquette de la colonne « Reste » par Loki dans l'onglet V4 (étape 0) ; cœur neutre
  `estimate.rs` (grammaire des deux formes, priorité, calcul, mise en forme) avec ses tests ;
  contrat de source et source Claude Code ; lecture unique de la tête de transcript (DA-T7) ;
  retrait du suffixe du texte de la tâche ; champ d'estimation dans le nœud d'agent ; colonne
  « Reste » à la place de « Statut » ; tri ; jetons `--sc-ok` / `--sc-warn` / `--sc-danger` clair et
  sombre ; infobulle ; libellé accessible.
- **Exclu** : toute modification des fichiers de la méthode iakaframe (lot séparé, cf.
  « Lot méthode ») ; sources pour d'autres runners que Claude Code ; historique des durées,
  estimation automatique ; liste de tâches (todo) de l'agent ; estimation pour la session
  principale ; notification ou alerte au dépassement ; animation ; couleur dans l'icône de barre de
  menus ; Linux ; étiquette en d'autres langues que le français.

## Étapes d'implémentation

0. **Maquette (Loki, avant tout code).** Dans `docs/maquettes/visu-agents-5-versions.html`, onglet
   V4 (tableau + aperçu produit 520 × 360) : remplacer la colonne Statut par « Reste » et montrer
   **cinq états** — vert (~30 %), orange (~85 %), rouge (dépassement), sans estimation (`—`),
   session principale (`—`) — en **clair et en sombre**, avec l'infobulle. Loki mesure et note les
   contrastes (barre sur piste ≥ 3:1, texte ≥ 4,5:1) et ajuste les teintes si besoin **dans la
   charte Studio clair**. **Stéphane valide la maquette** ; Gimli ne démarre qu'après.
1. **Cœur neutre.** Nouveau `src-tauri/src/estimate.rs` (déclaré dans `lib.rs`), **sans aucune
   référence à Claude Code ni à un chemin de fichier** :
   `struct DelegationText { consigne: Option<String>, short_label: Option<String> }` ;
   `struct Estimate { lo_secs: Option<i64>, hi_secs: i64 }` ;
   `parse_value(&str) -> Option<Estimate>` ; `parse_consigne_line(&str) -> Option<Estimate>` ;
   `split_label_suffix(&str) -> (String, Option<Estimate>)` (libellé sans suffixe valide) ;
   `resolve(&DelegationText) -> (Option<String> /* tâche */, Option<Estimate>)` (priorité ligne >
   suffixe) ; `estimate_view(&Estimate, elapsed_secs) -> EstimateView` avec `fill_pct: u8`
   (0-100), `level: "ok" | "warn" | "over"`, `short_label`, `long_label`, `remaining_secs: i64`
   (négatif en dépassement). Écrit à la main, aucune nouvelle dépendance. Commit.
2. **Source Claude Code.** Dans `agents.rs` : une fonction qui, à partir de la tête bornée du
   transcript **déjà lue pour le modèle** (DA-T7 : factoriser la lecture pour qu'elle serve aux
   deux), rend la consigne ; construction du `DelegationText` (consigne + `description`) ; appel
   de `estimate::resolve` ; la tâche affichée devient la valeur rendue par `resolve` ; ajout à
   `AgentNode` d'`estimate: Option<EstimateView>` (camelCase) ; infobulle DA-T4. Coordinateur et
   ligne `+N` : `None`. Commit.
3. **Rendu (TS, peinture seule).** `src/types.ts` : type `EstimateView`. `src/render.ts` :
   colonne `statut` remplacée par `remaining` (« Reste »), triable (Q6-A : `remainingSecs`,
   absent = `+Infinity`) ; cellule = élément `role="progressbar"` avec `aria-valuemin=0`,
   `aria-valuemax=100`, `aria-valuenow=fillPct`, `aria-valuetext=longLabel`, plus le texte court ;
   `—` si `estimate` est nul. Supprimer `renderStatusBadge` devenu inutile. Aucune décision côté
   TS. Commit.
4. **Styles.** `src/styles.css` : ajouter `--sc-ok`, `--sc-warn`, `--sc-danger` (valeurs de la
   maquette, clair l. 19 / sombre l. 31, ou celles ajustées par Loki), dans les trois blocs de
   thème existants ; classes de la barre (DA-T5) et des trois paliers. Commit.
5. **Tests.** Au minimum :
   - **valeur** acceptée : `~10 min` → 600 s ; `10 min` ; `~45 s` ; `~2 h` ; `~10-15 min` ;
     `~10–15 min` ; `~ 10 - 15 min` ; `~10min` ; refusée : `~1.5 h`, `~0 min`, `~5 s` (< 10 s),
     `~13 h` (> 12 h), `~1000 min`, `~15-10 min`, `~10 mins`, `~10 Min`, `~45 min-1 h`, chaîne vide ;
   - **ligne de consigne** : acceptée en ligne 1 ; acceptée en ligne 3 après un titre et une ligne
     vide ; `DURÉE ESTIMÉE : ~10 min` et `Duree estimee: 10 min` acceptées ; refusée en 6e ligne non
     vide ; refusée si suivie d'autre texte (`Durée estimée : ~10 min environ`) ; refusée au milieu
     d'une phrase ; consigne absente ⇒ rien ;
   - **suffixe** accepté : `X (~10 min)` → tâche `X` ; `( ~ 10 min )` ; libellé réduit au seul
     suffixe → tâche vide ; refusé, tâche **inchangée** : `Legolas passe 2 sur 3 (relance)`,
     `Gate qualité lot 6 (6·1, 6·3)`, `Rédaction Partie II (Lot 4)`, `X (10 min)` (sans tilde),
     `X (~10 min) suite` ;
   - **corpus réel** : les ~30 descriptions à parenthèse finale relevées le 26/09, recopiées en
     données de test, donnent **toutes** « pas d'estimation » et une tâche inchangée ;
   - **priorité** : ligne seule ; suffixe seul ; les deux égaux ; les deux différents ⇒ la ligne
     l'emporte **et** le suffixe est retiré de la tâche ; ligne invalide + suffixe valide ⇒ suffixe ;
   - **paliers** : 0 %, 74 % vert ; 75 % et 100 % orange ; 101 % rouge ; fourchette 10-15 min à
     9 min vert, à 10 min orange, à 16 min rouge ;
   - **textes** : 239 s restantes → `~4 min` ; 30 s → `< 1 min` ; 3 900 s → `~1 h 5 min` ;
     170 s de dépassement → `+3 min` ;
   - **source Claude Code / arbre** : transcript dont la 1re ligne `user` porte la ligne ⇒ nœud avec
     `estimate` ; balises `<command-message>` en lignes suivantes ignorées ; compagnon avec suffixe
     et transcript sans ligne ⇒ `estimate` depuis le suffixe, tâche sans suffixe ; rien ⇒ `estimate`
     nul ; date de création absente ⇒ nul ; coordinateur ⇒ nul ; le modèle reste correct après la
     factorisation de la lecture (tests existants inchangés) ; deux instantanés identiques restent
     égaux.
   Commit.
6. **Vérification.** `cargo test`, `cargo clippy --all-targets -- -D warnings`,
   `npm run typecheck`, `npm run build`, `npm run tauri build`, puis gate qualité (Legolas) et
   essai réel par Stéphane.

## Fichiers concernés

- `docs/maquettes/visu-agents-5-versions.html` — onglet V4 : colonne « Reste » (Loki, étape 0).
- `src-tauri/src/estimate.rs` — **nouveau** : cœur neutre (contrat, grammaire, priorité, calcul,
  mise en forme) et ses tests.
- `src-tauri/src/lib.rs` — déclaration `mod estimate;`.
- `src-tauri/src/agents.rs` — source Claude Code (consigne + `description` + début), lecture
  unique de la tête de transcript, champ `estimate`, tâche sans suffixe, infobulle, tests.
- `src/types.ts` — type `EstimateView`, champ `estimate` d'`AgentNode`.
- `src/render.ts` — colonne « Reste » à la place de « Statut », tri, barre accessible.
- `src/styles.css` — jetons ok/warn/danger clair et sombre, styles de la barre.
- `specs/instructions/liste-des-agents.md` — **aucune modification par Gimli** ; l'amendement
  « colonne Statut → Reste » (A4) y sera reporté par le cadrage après le « go ».

## Lot méthode (séparé, à cadrer dans iakaframe — NON modifié ici)

La convention est un changement de méthode transverse : c'est Stéphane qui la valide, dans un lot
à part. La règle s'y écrit **en termes neutres** (« la consigne de délégation commence par
`Durée estimée : ~<valeur>` ») ; le complément « suffixe dans `description` » n'apparaît que dans
les fichiers propres à Claude Code. Fichiers qui devraient la porter (relevé en lecture) :

- **Neutres (règle + grammaire)** :
  - `~/work/iakaframe/methode-de-travail.md` — § « Workflows de sous-agents — activés par défaut »
    (l. 467) : nouvelle règle « toute délégation annonce sa durée estimée » + grammaire.
  - `~/work/iakaframe/library/skills/iakaframe-aragorn/SKILL.md` — gabarit « Ordre de mission »
    (l. 64-68) : ajouter `Durée estimée : ~…` en tête.
  - `~/work/iakaframe/library/skills/iakaframe-odin/SKILL.md` — Odin délègue aussi.
  - `~/work/iakaframe/library/personas/aragorn.md` et `~/work/iakaframe/library/personas/odin.md` —
    les deux délégants du roster.
  - `~/work/iakaframe/library/skills/iakaframe-fabrication/SKILL.md` — si Gimli délègue en
    worktrees parallèles.
- **Propres à un runner (application de la règle)** :
  - Claude Code : `~/work/iakaframe/kits/iakaframe-claude/global/CLAUDE.md` (source) puis
    `~/.claude/CLAUDE.md` (déployé) — ligne dans « Workflows de sous-agents » : ligne en tête de
    consigne **et** suffixe `(~…)` dans `description`.
  - Autres runners : `~/work/iakaframe/kits/iakaframe-codex/AGENTS.md`,
    `kits/iakaframe-ollama/AGENTS.md`, `kits/iakaframe-openwebui/AGENTS.md`,
    `kits/iakaframe-anythingllm/AGENTS.md` — ligne en tête de consigne seulement (à vérifier par le
    cadrage du lot méthode : ces runners délèguent-ils réellement à des sous-agents ?).

## Risques

- **R1 — L'estimation n'est pas écrite** (oubli du délégant, lot méthode pas encore livré).
  *Mitigation* : tiret, jamais d'erreur ; la règle de méthode (Q7) fait le reste.
- **R2 — Estimations fantaisistes** (un modèle annonce 5 min pour 40). Accepté : c'est
  précisément ce que la barre rend visible ; le rouge est une information, pas une erreur.
- **R3 — Faux positif** sur une parenthèse existante ou une phrase de consigne. *Mitigation* :
  tilde obligatoire + unités fermées pour le suffixe ; étiquette exacte, ligne entière et
  5 premières lignes pour la consigne ; test sur le corpus réel.
- **R4 — Formats Claude Code non contractuels** (compagnon, transcript ; déjà R1 de
  `liste-des-agents.md`). *Mitigation* : lecture défensive ; la couche source isole ce risque dans
  un seul endroit.
- **R5 — Délégations par l'outil `Workflow`** : non vérifié si leurs transcripts et compagnons ont
  la même forme. *Mitigation* : rien de lisible ⇒ tiret ; à observer en recette.
- **R6 — Consigne très longue** (> 1 Mio) : première ligne JSON tronquée, ligne perdue.
  *Mitigation* : repli sur le suffixe (d'où l'intérêt de Q1-A plutôt que Q1-C).
- **R7 — Contraste de l'orange clair** : `#b5790a` sur la piste claire est proche de 3:1 (estimé
  ~3,2:1). *Mitigation* : mesure par Loki à l'étape 0 et par Legolas au gate ; ajustement dans la
  charte si < 3:1.
- **R8 — Horloge** : l'écoulé dépend de la date de création du fichier ; un transcript copié ou
  restauré fausserait la barre. Accepté (même source que « Démarré il y a »).

## Critères d'acceptation

- [ ] **Maquette** : l'onglet V4 montre la colonne « Reste » dans ses cinq états, clair et sombre,
      et Stéphane l'a validée **avant** le premier commit de code.
- [ ] `cargo test` vert, incluant **tous** les tests de l'étape 5 et tous les tests existants sans
      modification de leurs assertions.
- [ ] `cargo clippy --all-targets -- -D warnings` sans avertissement ; `npm run typecheck`,
      `npm run build` OK ; `npm run tauri build` produit une `.app`.
- [ ] `Cargo.toml` inchangé (aucune nouvelle dépendance, pas de `regex`).
- [ ] **Neutralité** : `src-tauri/src/estimate.rs` ne contient aucune occurrence de `claude`,
      `meta.json`, `jsonl`, `description` ni de chemin de fichier (vérifiable par `grep -i`).
- [ ] Lecture de la tête de transcript d'un sous-agent : **une seule** par tick (revue de code).
- [ ] Essai réel, **ligne seule** : consigne commençant par `Durée estimée : ~2 min`, `description`
      sans suffixe ⇒ Reste verte `~2 min`, orange à partir de 1 min 30 s, rouge `+1 min` après
      2 min ; infobulle conforme à DA-T4.
- [ ] Essai réel, **suffixe seul** : `description` = `Test barre (~2 min)`, consigne sans ligne ⇒
      même comportement ; colonne Tâche `Test barre`.
- [ ] Essai réel, **fourchette** : `Durée estimée : ~1-3 min` ⇒ orange dès 1 min, rouge après 3 min.
- [ ] Un sous-agent sans estimation, la session principale et la ligne `+N` affichent `—`.
- [ ] La colonne Statut a disparu ; l'ordre est Agent · Projet · Runner · Modèle · Tâche ·
      Démarré · Actif · Reste ; un clic sur « Reste » trie (dépassements en tête en croissant,
      tirets en dernier), sans casser l'indentation.
- [ ] Contraste mesuré (clair **et** sombre) : remplissage de chaque palier ≥ 3:1 sur la piste ;
      texte de la cellule ≥ 4,5:1 sur le fond ; l'information n'est jamais portée par la seule
      couleur (texte toujours présent).
- [ ] Lecteur d'écran : la barre annonce la phrase longue (`aria-valuetext`).
- [ ] Aucun fichier de `~/work/iakaframe` ni `~/.claude/CLAUDE.md` modifié par ce lot.

## Estimation (ordre de grandeur, révisable)

- **Charge lot app : ~1,25 jour-homme** (fourchette 1 à 1,75 j) — maquette Loki 0,2 ; cœur neutre
  + tests 0,4 ; source Claude Code + factorisation de la lecture 0,25 ; rendu + styles 0,2 ;
  qualité + recette 0,2.
- **Lot méthode (hors de cette instruction) : ~0,4 j** (cadrage + une dizaine de fichiers de texte,
  dont les kits des autres runners).
- **Complexité / risque : faible à moyen.** Grammaire minuscule, pas de dépendance ; le point
  délicat est la factorisation de la lecture de tête sans régression sur la colonne Modèle.
- **Inconnues qui peuvent faire glisser** : aller-retour sur la maquette (teintes, place du texte
  dans 520 px) ; forme des transcripts des délégations par `Workflow` (R5) ; réponse B à Q1
  (−0,2 j, mais contraire à la contrainte d'agnosticisme).

## Ce qui appartient à Stéphane

- Trancher **Q1 à Q7** (ou « go » pour les recommandations).
- **Valider la maquette** de Loki avant le code.
- **Valider la convention de méthode** (texte exact de l'étiquette `Durée estimée :`, place en
  première ligne) et commander le lot méthode dans iakaframe : c'est un changement de la façon
  dont tous les agents délèguent, sur tous les runners.
- L'essai réel avec de vraies délégations estimées, et le réglage éventuel du seuil de 75 %.

## Sources

- Fichiers réels lus le 2026-09-26 : 486 `agent-*.meta.json` sous `~/.claude/projects/`
  (recherche de `description` à parenthèse finale, de `~` et d'unités de temps), transcript
  `-Users-sjupin-work-iakaos/5b57a341-…/subagents/agent-a5ba54b2df0194497.jsonl` (ligne 1 =
  consigne complète en chaîne, lignes 2+ = skills injectées en `isMeta`).
- Code lu : `src-tauri/src/agents.rs`, `src-tauri/src/lib.rs`, `src/render.ts`, `src/styles.css`,
  `src-tauri/Cargo.toml`, `docs/maquettes/visu-agents-5-versions.html` ; méthode :
  `~/work/iakaframe/methode-de-travail.md`, `library/skills/iakaframe-aragorn/SKILL.md`.
- Contraste des objets graphiques, 3:1 contre les couleurs adjacentes :
  [W3C — Understanding SC 1.4.11 Non-text Contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html).
- Rôle `progressbar` et `aria-valuetext` :
  [MDN — ARIA progressbar role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/progressbar_role).
- Sous-agents Claude Code : [Claude Code Docs — sous-agents](https://code.claude.com/docs/en/sub-agents).
