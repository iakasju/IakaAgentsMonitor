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
  /** Modèle de CET agent, déjà résolu en libellé lisible côté Rust (ex. « Opus 5.5 »),
   * `"—"` si indisponible (amendement A1, passe corrective post-gate Legolas du 26/09). */
  model: string;
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
}

/** Instantané complet des agents en cours (miroir Rust `agents::AgentsSnapshot`). `count` =
 * effectif total (coordinateurs compris), même au-delà de la troncature d'affichage. */
export interface AgentsSnapshot {
  count: number;
  sessions: SessionNode[];
  overflowSessions: number | null;
}
