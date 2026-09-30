# Installation d'iakaFleet

iakaFleet a deux commandes : **`iakafleet-agent`** (une par machine, publie l'etat des sessions
Claude Code) et **`iakafleet`** (console, lit la flotte). Lecture seule : aucun ordre n'est envoye
ni execute (voir `specs/PROJET.md`).

## Prerequis

- Node >= 20, `npm install` dans le depot (une seule dependance : `mqtt`, epinglee).
- Un compte MQTT par machine (`fleet-<host>`) et un compte console (`fleet-console`), poses
  par le **Lot 0** (`naonedge-vps/specs/instructions/mqtt-acl-iakafleet.md`). Sans ce lot, le
  broker refuse (ou jette en silence) toute publication.
- Identifiants lus dans **Vaultwarden** (`work/iakaFleet`, items `iakaFleet/mqtt-fleet-<host>` et
  `iakaFleet/mqtt-fleet-console`). Jamais dans le depot.

## 1. Configuration : `~/.iakafleet/.env`

Meme chemin sous macOS, Windows et Linux. Les variables d'environnement priment. Modele : `.env.example`.

| Variable | Sens | Defaut |
|---|---|---|
| `IAKAFLEET_MQTT_URL` | broker ; `mqtt://` en clair accepte **uniquement** vers `127.0.0.1`/`localhost` | `mqtts://mqtt.naonedge.com:8883` |
| `IAKAFLEET_HOST` | nom de la machine dans les topics | `os.hostname()` normalise |
| `IAKAFLEET_AGENT_USER` / `_PASS` | compte de l'agent (`fleet-<host>`) | - |
| `IAKAFLEET_CONSOLE_USER` / `_PASS` | compte de la console (`fleet-console`) | - |
| `IAKAFLEET_SESSIONS_DIR` | repertoire des sessions Claude Code | `~/.claude/sessions` |
| `IAKAFLEET_SCAN_S` | periode de balayage | 5 |
| `IAKAFLEET_HEARTBEAT_S` | periode du battement | 60 |
| `IAKAFLEET_IAKAFRAME_PKG` | chemin explicite du `package.json` d'iakaframe | auto (PATH) |
| `IAKAFLEET_MASK_NAMES` | `1` = le nom des sessions est publie `null` (par machine) | non |

La verification du certificat TLS n'est pas debrayable.

## 2. Installer l'agent comme service

Le service est l'**unique** lanceur de l'agent (un seul client id `iakafleet-agent-<host>` par machine).
Substituer les placeholders (`__NODE__`, `__NODE_DIR__`, `__REPO__`, `__HOME__`).

- **macOS** : `deploy/launchd/com.naonedge.iakafleet-agent.plist` vers `~/Library/LaunchAgents/`,
  `mkdir -p ~/.iakafleet/logs`, puis `launchctl bootstrap gui/$(id -u) <plist>`.
- **Windows** : `powershell -NoProfile -File deploy\windows\Install-IakaFleetAgent.ps1`
  (tache planifiee au logon). **Non recette.** Desinstaller : `Uninstall-IakaFleetAgent.ps1`.
- **Linux** : `deploy/systemd/iakafleet-agent.service` (voir l'en-tete). **Non recette.**

## 3. Lancer la console

```bash
iakafleet ls           # instantane puis sortie
iakafleet watch        # rafraichi chaque seconde (Ctrl+C)
iakafleet ls --json    # sortie machine
```

Codes de sortie : `0` ok (y compris flotte vide), `2` broker injoignable / authentification refusee /
configuration invalide, `1` usage. **Une table vide n'est jamais le signe d'un echec** : un echec
s'ecrit sur stderr avec le code 2.

## 4. Lire les etats

| Affiche | Sens |
|---|---|
| **en ligne** | agent connecte, dernier battement recent (<= 3 x `heartbeat_s`) |
| **muette** | agent encore connecte au broker mais horloge arretee (battement perime) |
| **hors ligne depuis X** | testament recu ou arret propre : la machine est partie |
| **inconnue** | la machine a publie des codes mais jamais `online` |
| `! sessions illisibles` | l'agent ne peut pas lire le repertoire de sessions de Claude Code |
| `? (dernier connu : busy, il y a 2 h)` | session d'une machine non "en ligne" : derniere valeur connue, pas un fait |
| `busy ?` | session suspecte d'etre restee `busy` a tort (bug connu de Claude Code) |
| `rc` / `--` | session pilotee par Remote Control ou non |

Une machine **mise en veille passe "hors ligne"** : c'est la verite, pas une erreur.

## Limites connues

- Le fichier `~/.claude/sessions/<pid>.json` est un **format interne** de Claude Code, non documente :
  il peut changer sans preavis (l'agent est tolerant et publie `sessions_dir_ok=false` s'il est aveugle).
- Presence de ce repertoire sous Windows : **non mesuree** a ce jour.
- Le champ `rc` repose sur un `bridgeSessionId` non nul ; verifie sur un seul echantillon a ce jour.
