# Instruction (correctif) : durée « hors ligne » fausse après un testament

> Cadré par **Gandalf** (P1) le **2026-09-30** au soir, sur défaut **mesuré en recette réelle sur le
> Mac** et remonté par Odin. Exécution : **Gimli** (P2), branche `feat/fusion-flotte`. Parent :
> `specs/instructions/fusion-flotte-vue-distante.md` (§ 11, écart instruit). Contrat : `specs/contrat-bus-iaka.md`.

---

## 1. Problème

Quand une machine meurt, la console et l'app affichent « **hors ligne depuis <durée écoulée depuis le
DÉMARRAGE du démon>** » au lieu de la durée depuis sa disparition. Un démon qui a vécu 3 jours puis est
tué s'affiche « hors ligne depuis 3 j » alors qu'il est parti depuis quelques secondes. La cause est
structurelle : le testament MQTT est **déposé à la connexion** et ne peut pas connaître l'heure de la mort.

## 2. Faits

| Id | Fait | Preuve |
|---|---|---|
| **F1** | Le testament est construit à la connexion : `buildConnectOptions` pose `will.payload = encode(false, nowS)`, `nowS` = instant de **connexion** | `fleet/src/agent/publisher.js` (`buildConnectOptions`) — rapport Odin/Gimli |
| **F2** | 🛑 **Mesure réelle, Mac, 2026-09-30** : abonné console sur `iaka/mbp-de-sjupin/out/presence/online/current` pendant un `launchctl kill SIGKILL` du démon. Reçus dans l'ordre : retenu `{"v":true,"t":1790797788}` ; **testament** `{"v":false,"t":1790797786}` (`retain=0` à la livraison, reçu **à la seconde du kill**) ; `{"v":true,"t":1790797983}` du démon relancé par launchd. Le `t` du testament (…786) est **antérieur** au `true` du même démon (…788) : c'est l'heure de **démarrage** | recette du 2026-09-30, relayée par Odin |
| **F3** | Le testament a bien été **livré** : l'ACL le couvre, le piège eclipse-mosquitto#3552 est **écarté** en réel | F2 |
| **F4** | La règle d'état prend ce `t` pour « depuis » : `if (online.v === false) return { state: 'offline', since: online.t }` | `fleet/src/console/fleetModel.js:13` |
| **F5** | La CLI l'affiche : `` `${base} depuis ${formatDuration(nowS - m.since)}` `` | `fleet/src/console/render.js:20` |
| **F6** | Même règle en Rust : `Value::Bool(false) => MachineState { state: State::Offline, since: Some(online.t) }` ; libellé `hors ligne depuis …` | `src-tauri/src/fleet_model.rs:89,207` |
| **F7** | Le vecteur partagé fige le comportement fautif : cas « hors ligne (testament) » ⇒ `since` = `online.t` ; **aucun** cas ne combine `online=false` et `last_tick_at` | `contract-vectors/machine-state.json:10` |
| **F8** | `last_tick_at` est déjà publié **retenu**, republié à chaque battement (60 s) ; l'état `mute` s'en sert déjà comme « dernier battement » | `fleetModel.js:16-18` · contrat § 2 |
| **F9** | Un **arrêt propre** publie `online=false` **à l'instant de l'arrêt** (MVP Fleet `D6`) : son `t` est juste. Seul le **testament** ment | instruction MVP Fleet `D6` |
| **F10** | MQTT 3.1.1 ne permet pas de modifier le testament sans se **reconnecter** | spécification MQTT 3.1.1 § 3.1.2.5 |
| **F11** | La doc d'exploitation promet « **hors ligne depuis X** » | `fleet/docs/installation.md:84` |
| **M1** | **À mesurer par Gimli** : `last_tick_at` **reste retenu** après la mort du démon (aucun effacement à l'arrêt ni au testament) — abonné après un `kill -9`, lire le retenu | — |

## 3. Décision

### `D1` — « Depuis » d'une machine hors ligne = **dernier signe de vie connu** : `max(online.t, last_tick_at.v)`

- **Testament** : `online.t` = connexion (ancien), `last_tick_at` = dernier battement ⇒ le `max` rend le
  **dernier battement**, à ≤ 60 s de la mort (+ ≤ 45 s de détection keepalive). Honnête.
- **Arrêt propre** : `online.t` = instant d'arrêt ≥ dernier battement ⇒ le `max` rend l'**heure exacte**.
- **`last_tick_at` absent** : repli sur `online.t`, **marqué comme tel** (`basis: "connected"`), car on ne
  sait alors que l'heure de la dernière **connexion**.

La règle ne distingue pas testament et arrêt propre : elle n'en a pas besoin. Le résultat porte un
champ **`basis`** : `"last_seen"` (le `max` a pu utiliser un signe de vie postérieur à la connexion, ou un
arrêt propre) ou `"connected"` (repli).

*Écartées* : **reconnecter** périodiquement pour rafraîchir le testament (bascules et bruit sur le broker
pour gagner une précision que `last_tick_at` donne déjà) ; **MQTT 5 / délai de testament** (le démon et
l'app sont en 3.1.1 ; ne donne pas non plus l'heure de la mort).

### `D2` — Libellés honnêtes, identiques en CLI et dans l'app

| Cas | Libellé |
|---|---|
| `basis = "last_seen"` | **`hors ligne (dernier signe de vie il y a <durée>)`** |
| `basis = "connected"` | **`hors ligne (heure d'arrêt inconnue ; connecté il y a <durée>)`** |
| `since = null` | `hors ligne` |

Le mot « depuis » disparaît de l'état hors ligne : il affirmait une heure de départ qu'on ne connaît pas.

### `D3` — Le contrat documente le `t` du testament

`contrat-bus-iaka.md` § 2 gagne une règle : **le `t` d'un `online=false` venu du testament est l'instant
de CONNEXION du démon, pas celui de son départ** ; tout lecteur calcule « depuis » par la règle `D1` et
ne l'affiche jamais seul. La matrice § 6 est inchangée (`machine-state.json` : `fleet/` et app).

## 4. Périmètre

- **Inclus** : `contract-vectors/machine-state.json` (cas neufs + champ `basis`) ; `fleet/src/console/fleetModel.js`
  et `render.js` ; `src-tauri/src/fleet_model.rs` (règle + libellé) et les tests Rust de vecteurs ;
  `specs/contrat-bus-iaka.md` § 2 (`D3`, écrit par ce cadrage) ; `fleet/docs/installation.md` § « Lire les
  états » (tableau, l. 84).
- **Exclu** : le démon (`publisher.js`, testament inchangé) ; toute reconnexion ; MQTT 5 ; le front TS
  (sauf s'il recompose lui-même le libellé — `M2`) ; le Lot 0.

## 5. Étapes (Gimli)

0. `M1` ; `M2` : `grep -rn "hors ligne" src/ src-tauri/src fleet/src` — le libellé n'est-il construit qu'en
   `render.js` et `fleet_model.rs` ? Sinon, ajouter l'autre lieu au périmètre et le signaler.
1. **Vecteurs d'abord, vus ROUGES** (trace dans le message de commit), dans `machine-state.json` :
   - ajouter `"basis"` à l'`expected` de **tous** les cas (`null` sauf état `offline`) ;
   - cas existant « hors ligne (testament) » ⇒ `since: 2800, basis: "connected"` (pas de `last_tick_at`) ;
   - **neuf** « testament après longue vie » : `online {v:false,t:1000}`, `last_tick_at {v:9950}`, `nowS 10000`
     ⇒ `offline`, `since 9950`, `basis "last_seen"` — **c'est lui qui prouve le défaut** (rouge : 1000 aujourd'hui) ;
   - **neuf** « arrêt propre » : `online {v:false,t:9990}`, `last_tick_at {v:9950}` ⇒ `since 9990`, `basis "last_seen"` ;
   - **neuf** « `last_tick_at` illisible » : `online {v:false,t:2800}`, `last_tick_at {v:"x"}` ⇒ `since 2800`, `basis "connected"`.
   `npm test` (fleet) et `cargo test` doivent rougir sur ces cas.
2. `fleetModel.js` : règle `D1`, champ `basis` exposé (et dans `--json`). Vecteurs verts côté Node.
3. `render.js` : libellés `D2`. Tests de rendu sur les deux libellés.
4. `fleet_model.rs` : même règle, champ `basis`, libellés `D2`. Vecteurs verts côté Rust.
5. `fleet/docs/installation.md` l. 84 : ligne du tableau réécrite selon `D2`, avec une phrase : « la
   précision est d'environ une minute (période de battement) ».
6. `npm test`, `npm run lint` (fleet), `cargo test`, `cargo clippy --all-targets -- -D warnings`.

## 6. Critères d'acceptation

- [ ] **CA-1** — Les trois cas neufs de `machine-state.json` ont été **vus rouges** côté Node **et** Rust avant correction (trace `git log`).
- [ ] **CA-2** — Tous les vecteurs `machine-state` verts dans `fleet/` et l'app, champ `basis` compris.
- [ ] **CA-3** — **Contre-essai** : revenir temporairement à `since: online.t` dans `fleetModel.js` fait rougir le cas « testament après longue vie » (le vecteur mesure bien le défaut) ; idem en Rust.
- [ ] **CA-4** — Rendu : `hors ligne (dernier signe de vie il y a 50 s)` et `hors ligne (heure d'arrêt inconnue ; connecté il y a …)` exactement, en CLI et dans l'app ; plus aucune occurrence de « hors ligne depuis » dans le code (`grep` ⇒ 0).
- [ ] **CA-5** — `publisher.js` inchangé (`git diff --stat` vide) : le testament n'est pas touché.
- [ ] **CA-R1** — **Recette réelle (Mac)** : démon vivant ≥ 10 min, **abonné console lancé AVANT**, puis
      `launchctl kill SIGKILL` : pendant la fenêtre avant relance, `iakafleet ls --json` (ou la lecture du
      retenu `online` + `last_tick_at` à l'instant du testament) donne `since` = dernier battement, **pas**
      l'heure de démarrage ; libellé « dernier signe de vie il y a ≤ ~60 s ». Arrêt propre
      (`launchctl bootout`) : `since` = heure de l'arrêt, **état hors ligne stable** tant que le service
      n'est pas relancé.

## 7. Estimation

**≈ 0,3 j-h** (vecteurs 0,05 · Node 0,1 · Rust 0,1 · doc + recette 0,05). Complexité **faible**, risque
**faible**. Inconnue : `M2` (libellé recomposé ailleurs, +0,05). *Ordre de grandeur révisable.*
