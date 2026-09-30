// Types du contrat de rendu entre le backend Rust (arbre de délégation déjà résolu) et la
// webview. Le backend sérialise en camelCase (serde rename_all). La webview ne fait AUCUN calcul
// métier (D9) : elle reçoit un instantané via la commande `get_running_agents` et les mises à
// jour via l'événement `tray://agents` ; le tri de tableau (amendement A1) ne fait que comparer
// les champs déjà résolus, sans réinterpréter la donnée.

/** Sprite d'un agent : lettre dans un carré arrondi à la couleur de sa pastille (miroir Rust
 * `agents::Sprite`). Point d'extension UNIQUE (amendement A1, 26/09) : Loki proposera des icônes
 * pixel art 4×4, choisies par Stéphane — pas encore implémentées ici. Type hors roster -> fond
 * gris + sa propre première lettre. */
export interface Sprite {
  letter: string;
  bg: string;
  fg: string;
}

/** Estimation de temps restant, déjà mise en forme côté Rust (miroir `estimate::EstimateView`,
 * specs/instructions/temps-restant.md). Rien à calculer côté webview : `fillPct`/`level`
 * pilotent la barre, `shortLabel` le texte de cellule, `longLabel` l'annonce accessible
 * (`aria-valuetext`), `remainingSecs` sert uniquement au tri (négatif = dépassement). */
export interface EstimateView {
  fillPct: number;
  level: "ok" | "warn" | "over";
  shortLabel: string;
  longLabel: string;
  remainingSecs: number;
}

/** Nœud d'agent déjà résolu pour l'affichage (miroir Rust `agents::AgentNode`). Un nœud marqueur
 * de débordement porte `overflow` non-null et n'a pas d'enfants. Les champs `*Secs` sont bruts
 * (tri de tableau uniquement) ; `*Ago` sont déjà formatés côté Rust (D9). */
export interface AgentNode {
  sprite: Sprite;
  persona: string;
  task: string;
  activeAgo: string;
  activeSecs: number;
  startedAgo: string;
  startedSecs: number | null;
  /** Modèle de CET agent, déjà résolu en libellé lisible côté Rust (fichier de configuration des
   * libellés), `"—"` si indisponible (amendement A1, passe corrective post-gate Legolas du
   * 26/09). */
  model: string;
  /** Libellé du runner de CET agent, déjà résolu côté Rust (`labels::Labels::runner_label`,
   * specs/instructions/agnostique-mqtt.md, étape 8) — plus aucune constante figée ici : toute la
   * neutralité vis-à-vis du runner tient dans ce champ. */
  runner: string;
  /** État d'exécution (`running` | `idle`, contrat § Fiche) ; vide pour le marqueur de débordement. */
  state: string;
  /** Temps restant estimé (colonne « Reste »), `null` pour le coordinateur, le marqueur de
   * débordement, ou tout agent sans estimation annoncée — cellule à tiret, jamais de barre
   * inventée. */
  estimate: EstimateView | null;
  tooltip: string;
  children: AgentNode[];
  overflow: number | null;
}

/** Une session vivante affichée dans le tableau (miroir Rust `agents::SessionNode`). */
export interface SessionNode {
  project: string;
  coordinator: AgentNode;
  /** Index de couleur stable du dépôt (amendement A7, point 3), déjà résolu côté Rust
   * (`group_color_index`, hash du nom de projet) : `render.ts` ne fait que traduire cet index en
   * classe CSS (`agents-group-row--c<index>`), les valeurs de couleur vivent dans `styles.css`. */
  groupColor: number;
}

/** Instantané complet des agents en cours (miroir Rust `agents::AgentsSnapshot`). `count` =
 * effectif total (coordinateurs compris), même au-delà de la troncature d'affichage. */
export interface AgentsSnapshot {
  count: number;
  sessions: SessionNode[];
  overflowSessions: number | null;
  /** Agents dont la session n'est plus vivante mais dont la fiche est fraîche (D2.5 de
   * `ce-poste-vue-locale-lisibilite.md`) : « agents non rattachés », jamais perdus. */
  unattached: AgentNode[];
  /** Titre, état du démon et phrase d'état de « Ce poste » (D4), déjà composés côté Rust. */
  header: LocalHeader | null;
}

/** En-tête de « Ce poste » (miroir Rust `local_view::LocalHeader`) : la webview ne fait que peindre. */
export interface LocalHeader {
  /** `Ce poste — <node> · N session(s) · M agent(s)`. */
  title: string;
  /** `démon : <libellé>` — mêmes mots que la vue distante. */
  daemon: string;
  /** « 2 agents travaillent · 1 session en cours sur 4 ». */
  phrase: string;
  /** Avertissement de source locale dégradée (l'absence ne se déguise jamais en vide). */
  notice: string | null;
  sessions: number;
  agents: number;
  activeAgents: number;
  busySessions: number;
}

// ---- Vue « Autres machines » (central) : miroir Rust `sources::remote::RemoteView` -----------
// Tout est déjà résolu côté Rust (libellés, états, rattachement des agents) : la webview ne fait
// que peindre. Sans central configuré, `status` = "not_configured" et la section est absente.

/** Un agent d'une autre machine. `taskMasked` : la machine ne publie pas le texte de tâche. */
export interface FleetAgent {
  sid: string;
  agentId: string;
  parentId: string | null;
  persona: string | null;
  model: string | null;
  project: string | null;
  task: string | null;
  taskMasked: boolean;
  stateLabel: string;
  activeAgo: string;
  startedAgo: string;
  live: boolean;
}

export interface FleetSession {
  pid: number;
  statusLabel: string;
  name: string | null;
  repo: string | null;
  branch: string | null;
  cwd: string | null;
  live: boolean;
  agents: FleetAgent[];
}

export interface FleetMachine {
  node: string;
  state: "online" | "mute" | "offline" | "unknown";
  os: string | null;
  stateLabel: string;
  alerts: string[];
  sessions: FleetSession[];
  unattachedAgents: FleetAgent[];
}

export interface RemoteView {
  status: "not_configured" | "connecting" | "connected" | "unreachable";
  reason: string | null;
  machines: FleetMachine[];
  ownNode: string | null;
  /** Hôte du broker central (message « Aucune autre machine n'a publié sur <hôte> »). */
  brokerHost: string | null;
}
