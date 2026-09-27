// Rendu du tableau des agents en cours (amendement A1 du 26/09/2026, remplace la popover V1 de
// la maquette de référence ; amendement A3 du 26/09/2026, alignement visuel sur l'onglet V4 de
// Loki) : une ligne par agent, colonnes triables, INDENTÉE par profondeur de délégation (chaque
// enfant sous son parent, décalé d'un niveau) avec des guides ASCII (├─ └─ │). Amendement A6
// (27/09/2026) : GROUPEMENT PAR DÉPÔT — une ligne de séparation pleine largeur porte le nom du
// projet, l'arbre de délégation de ce dépôt en dessous ; la colonne « Projet » disparaît (portée
// par la ligne de groupe) ; le tri ne s'applique plus qu'À L'INTÉRIEUR de chaque groupe (l'ordre
// des groupes reste celui, déjà déterministe, renvoyé par le backend — activité décroissante).
// Toute la logique (roster, liveness, arbre, formatage des durées, capitalisation des personas,
// contraste) est déjà résolue côté Rust (D9) : ce module ne fait que peindre et trier — le tri ne
// réinterprète aucune donnée, il compare des champs déjà résolus.
//
// Référence de maquette : docs/maquettes/visu-agents-5-versions.html, onglet V4 (tableau dense +
// aperçu produit, groupé par dépôt), adaptée ici avec indentation hiérarchique au lieu d'une
// liste plate.

import type { AgentNode, AgentsSnapshot, SessionNode } from "./types";

/** Colonnes triables du tableau. "runner" et "status" n'en font PAS partie : "runner" est une
 * constante pour ce lot (Q6-A : macOS + Claude Code seul) ; "status" (colonne « État ») n'a
 * aujourd'hui qu'un seul état réel possible (running, amendement A6 point 3) — rien à trier.
 * "remaining" (colonne « Reste ») a un comparateur dédié (cf. `compareNodes`) : les tirets (sans
 * estimation) restent TOUJOURS en dernier, quel que soit le sens du tri
 * (specs/instructions/temps-restant.md, Q6-A). */
type SortCol = "agent" | "task" | "active" | "started" | "model" | "remaining";

interface SortState {
  col: SortCol;
  dir: 1 | -1;
}

// Tri par défaut : activité la plus récente (déjà l'ordre transmis par Rust côté sessions), pas
// de ré-échange nécessaire tant que l'utilisateur n'a pas cliqué un en-tête.
let sort: SortState | null = null;
let lastSnapshot: AgentsSnapshot | null = null;

/** Mode d'affichage (amendement A4, point 1) : compact = 5 colonnes dans la fenêtre 520×360
 * (Agent · Projet · Runner · Actif · Reste, comme l'aperçu produit de la maquette V4) ; complet =
 * les 8 colonnes dans la fenêtre élargie (~1000×420). Par défaut compact, comme la fenêtre créée
 * par Tauri. Le choix mémorisé (localStorage) et le redimensionnement de la fenêtre sont pilotés
 * par main.ts ; ce module ne fait que peindre le bon jeu de colonnes. */
let compact = true;

/** Bascule le mode d'affichage et re-peint immédiatement le dernier instantané connu (si un
 * rendu a déjà eu lieu) — même logique que le re-tri sur clic d'en-tête. */
export function setCompactMode(value: boolean): void {
  compact = value;
  if (lastSnapshot) renderAgents(lastSnapshot);
}

/** Point d'extension UNIQUE pour l'icône d'un agent (amendement A1, décision A2 : carré arrondi
 * couleur pastille + lettre, en attendant une piste de portraits jugée suffisante). */
function renderSpriteIcon(node: AgentNode): HTMLElement {
  const el = document.createElement("span");
  el.className = "sprite";
  el.style.background = node.sprite.bg;
  el.style.color = node.sprite.fg;
  el.textContent = node.sprite.letter;
  return el;
}

/** Étiquette encadrée (chip, amendement A3 point 2) : Runner et Modèle. */
function renderChip(text: string): HTMLElement {
  const el = document.createElement("span");
  el.className = "chip";
  el.textContent = text;
  return el;
}

/** Infobulle de la colonne « État », amendement A6 point 3 : tant que le lot bus MQTT (et son
 * amendement A1 d'agnosticisme) n'est pas livré, AUCUN bouton ne pilote quoi que ce soit — jamais
 * de bascule purement visuelle qui ferait croire à une action. */
const STATUS_MQTT_TOOLTIP = "commande disponible avec le bus MQTT";

/** Colonne « État » (amendement A6, point 3) : pastille « running » — seul état réel aujourd'hui,
 * l'app n'a aucune autre source d'information sur l'exécution — accompagnée d'un bouton bascule
 * et d'un bouton ■ (arrêt), tous deux DÉSACTIVÉS avec l'infobulle `STATUS_MQTT_TOOLTIP`. Rien
 * n'est cliquable : ni état local inventé, ni effet simulé. */
function renderStatusCell(): HTMLElement {
  const wrap = document.createElement("span");
  wrap.className = "status-cell";

  const toggle = document.createElement("button");
  toggle.type = "button";
  toggle.className = "status-btn";
  toggle.dataset.state = "running";
  toggle.disabled = true;
  toggle.title = STATUS_MQTT_TOOLTIP;
  toggle.setAttribute("aria-label", STATUS_MQTT_TOOLTIP);
  const dot = document.createElement("span");
  dot.className = "status-dot";
  toggle.appendChild(dot);
  toggle.appendChild(document.createTextNode("running"));

  const stop = document.createElement("button");
  stop.type = "button";
  stop.className = "status-stop-btn";
  stop.disabled = true;
  stop.title = STATUS_MQTT_TOOLTIP;
  stop.setAttribute("aria-label", STATUS_MQTT_TOOLTIP);
  stop.textContent = "■";

  wrap.append(toggle, stop);
  return wrap;
}

/** Barre « Reste » (remplace la colonne Statut, specs/instructions/temps-restant.md) : piste
 * accessible (`role="progressbar"`, `aria-valuetext` = phrase longue déjà composée en Rust),
 * remplissage coloré par palier, texte court à côté. Sans estimation (session principale, ligne
 * de débordement, délégation sans durée annoncée) : un tiret, jamais de barre inventée (DA-T6). */
function renderRemainingCell(node: AgentNode): HTMLElement {
  const est = node.estimate;
  if (!est) {
    const dash = document.createElement("span");
    dash.className = "reste-dash";
    dash.title = "pas de durée estimée annoncée (session principale ou délégation sans durée)";
    dash.textContent = "—";
    return dash;
  }
  const wrap = document.createElement("span");
  wrap.className = "reste";

  const track = document.createElement("span");
  track.className = "reste-track";
  track.setAttribute("role", "progressbar");
  track.setAttribute("aria-valuemin", "0");
  track.setAttribute("aria-valuemax", "100");
  track.setAttribute("aria-valuenow", String(Math.round(est.fillPct)));
  track.setAttribute("aria-valuetext", est.longLabel);

  const fill = document.createElement("span");
  fill.className = "reste-fill" + (est.fillPct >= 100 ? " full" : "");
  fill.style.width = `${est.fillPct}%`;
  fill.style.background =
    est.level === "ok" ? "var(--sc-ok)" : est.level === "warn" ? "var(--sc-warn)" : "var(--sc-danger)";
  track.appendChild(fill);
  if (est.level === "over") {
    const mark = document.createElement("span");
    mark.className = "reste-mark";
    track.appendChild(mark);
  }

  const txt = document.createElement("span");
  txt.className = "reste-txt" + (est.level === "over" ? " over" : "");
  txt.textContent = est.shortLabel;

  wrap.append(track, txt);
  return wrap;
}

/** Colonnes dont le tri compare un champ simple déjà résolu par Rust (D9) — "remaining" est
 * exclu : son tri a une règle propre (tirets toujours en dernier), traitée à part dans
 * `compareNodes`. */
type SimpleSortCol = Exclude<SortCol, "remaining">;

function sortValue(col: SimpleSortCol, node: AgentNode): string | number {
  switch (col) {
    case "agent":
      return node.persona.toLowerCase();
    case "task":
      return node.task.toLowerCase();
    case "model":
      return node.model.toLowerCase();
    case "active":
      return node.activeSecs;
    case "started":
      // Non disponible -> trié en dernier (Infinity), jamais avant une valeur connue.
      return node.startedSecs ?? Number.POSITIVE_INFINITY;
  }
}

function compareNodes(col: SortCol, dir: 1 | -1, a: AgentNode, b: AgentNode): number {
  if (col === "remaining") {
    // Q6-A (temps-restant.md) : dépassement = valeur négative, donc en tête en tri croissant ;
    // SANS estimation = TOUJOURS en dernier, quel que soit le sens du tri (pas seulement en
    // tri croissant) — comparateur dédié plutôt qu'une valeur sentinelle sensible à `dir`.
    const ra = a.estimate?.remainingSecs;
    const rb = b.estimate?.remainingSecs;
    const aHas = ra !== undefined;
    const bHas = rb !== undefined;
    if (aHas && !bHas) return -1;
    if (!aHas && bHas) return 1;
    if (!aHas && !bHas) return 0;
    if (ra! < rb!) return -1 * dir;
    if (ra! > rb!) return 1 * dir;
    return 0;
  }
  const va = sortValue(col, a);
  const vb = sortValue(col, b);
  if (va < vb) return -1 * dir;
  if (va > vb) return 1 * dir;
  return 0;
}

/** Trie RÉCURSIVEMENT les enfants d'un nœud par la colonne choisie, sans jamais casser
 * l'adjacence parent → enfant (l'indentation reste toujours vraie, même après un clic de tri) :
 * on ne fait que réordonner les FRÈRES à chaque niveau, jamais les niveaux entre eux. Amendement
 * A6 : le tri ne porte plus QUE sur cette fratrie (à l'intérieur d'un groupe de dépôt) — les
 * groupes eux-mêmes ne se réordonnent plus entre eux (cf. `sortedSnapshot`). */
function sortChildrenRecursively(node: AgentNode, col: SortCol, dir: 1 | -1): void {
  node.children.sort((a, b) => compareNodes(col, dir, a, b));
  for (const child of node.children) sortChildrenRecursively(child, col, dir);
}

/** Amendement A6, point 2 : l'ordre des GROUPES (un par dépôt) reste celui, déjà déterministe,
 * renvoyé par le backend (activité décroissante, `snapshot_at`) — cliquer un en-tête ne trie plus
 * que l'arbre de délégation À L'INTÉRIEUR de chaque groupe (cf. doc de
 * `sortChildrenRecursively`). */
function sortedSnapshot(snap: AgentsSnapshot): SessionNode[] {
  const sessions = snap.sessions.map((s) => ({
    project: s.project,
    coordinator: structuredClone(s.coordinator),
  }));
  if (!sort) return sessions;
  const { col, dir } = sort;
  for (const s of sessions) sortChildrenRecursively(s.coordinator, col, dir);
  return sessions;
}

interface AgentRow {
  kind: "agent";
  node: AgentNode;
  depth: number;
  /** Guide ASCII de délégation (├─ / └─ / │, amendement A3 point 4) : vide à la racine (depth
   * 0, un coordinateur par session — pas de fratrie à représenter). */
  guide: string;
}

/** Ligne de séparation pleine largeur portant le nom du dépôt (amendement A6, point 2). */
interface GroupRow {
  kind: "group";
  project: string;
}

type TableRow = AgentRow | GroupRow;

/** Aplatit l'arbre en lignes GROUPÉES PAR DÉPÔT (amendement A6, point 2) : une `GroupRow` par
 * session, suivie de l'arbre de délégation de cette session, tout en calculant, pour chaque
 * nœud, son guide de délégation — même algorithme que la maquette V4 de Loki (`buildTreeRows` /
 * `buildGroupedLiveRows`) : un parcours en profondeur où chaque fratrie connaît sa propre
 * longueur (pour distinguer le dernier enfant, `└─`, des autres, `├─`) et transmet un préfixe
 * cumulé (`│  ` sous un frère non-dernier, `   ` sous le dernier) à ses propres enfants. */
function flattenGrouped(sessions: SessionNode[]): TableRow[] {
  const rows: TableRow[] = [];
  function walk(nodes: AgentNode[], depth: number, prefix: string): void {
    nodes.forEach((node, i) => {
      const isLast = i === nodes.length - 1;
      const guide = depth === 0 ? "" : prefix + (isLast ? "└─ " : "├─ ");
      rows.push({ kind: "agent", node, depth, guide });
      if (node.children.length > 0) {
        const childPrefix = depth === 0 ? "" : prefix + (isLast ? "   " : "│  ");
        walk(node.children, depth + 1, childPrefix);
      }
    });
  }
  for (const s of sessions) {
    rows.push({ kind: "group", project: s.project });
    walk([s.coordinator], 0, "");
  }
  return rows;
}

/** Ordre des colonnes : Agent · Runner · Modèle · Tâche · Démarré il y a · Actif il y a · État ·
 * Reste (specs/instructions/temps-restant.md, Q3-A : « Reste » remplace « Statut » ; amendement
 * A6, point 2 : « Projet » disparaît, portée par la ligne de groupe pleine largeur ; point 3 :
 * « État » s'intercale avant « Reste », comme les deux fenêtres produit de la maquette V4).
 * `compact: true` = colonne conservée dans la fenêtre compacte (amendement A4, point 1) : leur
 * ordre relatif dans ce tableau, une fois filtré, donne Agent · Runner · Actif · État · Reste. */
type ColumnDef = {
  id: SortCol | "runner" | "status";
  label: string;
  /** Intitulé plus court réservé au mode compact (amendement A4, point 1), quand l'intitulé
   * complet ne laisserait plus de place au contenu de la colonne — ex. « Actif » au lieu de
   * « Actif il y a », comme l'aperçu produit de la maquette V4. Absent -> `label` sert dans les
   * deux modes. */
  compactLabel?: string;
  sortable: boolean;
  compact: boolean;
};
const COLUMNS: ColumnDef[] = [
  { id: "agent", label: "Agent", sortable: true, compact: true },
  { id: "runner", label: "Runner", sortable: false, compact: true },
  { id: "model", label: "Modèle", sortable: true, compact: false },
  { id: "task", label: "Tâche", sortable: true, compact: false },
  { id: "started", label: "Démarré il y a", sortable: true, compact: false },
  { id: "active", label: "Actif il y a", compactLabel: "Actif", sortable: true, compact: true },
  { id: "status", label: "État", sortable: false, compact: true },
  { id: "remaining", label: "Reste", sortable: true, compact: true },
];

/** Colonnes effectivement affichées pour le mode courant. */
function visibleColumns(): ColumnDef[] {
  return compact ? COLUMNS.filter((c) => c.compact) : COLUMNS;
}

function headerRow(): HTMLElement {
  const tr = document.createElement("tr");
  for (const { id, label, compactLabel, sortable } of visibleColumns()) {
    const th = document.createElement("th");
    th.textContent = compact && compactLabel ? compactLabel : label;
    if (sortable) {
      th.dataset.col = id;
      // Indicateur ▲/▼ sur la colonne active (amendement A3, point 5).
      if (sort && sort.col === id) {
        th.textContent += sort.dir === 1 ? " ▲" : " ▼";
      }
    } else {
      th.className = "not-sortable";
    }
    tr.appendChild(th);
  }
  return tr;
}

/** Ligne de séparation pleine largeur portant le nom du dépôt (amendement A6, point 2), même
 * rôle que `.pv-group-row` de la maquette V4 : `colspan` couvre toutes les colonnes visibles du
 * mode courant. */
function groupRowElement(project: string, columnCount: number): HTMLElement {
  const tr = document.createElement("tr");
  tr.className = "agents-group-row";
  const td = document.createElement("td");
  td.colSpan = columnCount;
  td.textContent = project;
  tr.appendChild(td);
  return tr;
}

function agentRow(row: AgentRow): HTMLElement {
  const tr = document.createElement("tr");
  tr.className = row.node.overflow !== null ? "agents-overflow-row" : "";
  tr.title = row.node.tooltip;
  const isOverflow = row.node.overflow !== null;

  for (const { id } of visibleColumns()) {
    const td = document.createElement("td");
    switch (id) {
      case "agent": {
        td.className = "agents-cell-agent";
        if (row.guide) {
          const guideEl = document.createElement("span");
          guideEl.className = "tree-guide";
          guideEl.textContent = row.guide;
          td.appendChild(guideEl);
        }
        td.appendChild(renderSpriteIcon(row.node));
        const name = document.createElement("span");
        name.className = "agents-persona";
        name.textContent = row.node.persona;
        td.appendChild(name);
        break;
      }
      case "runner":
        if (!isOverflow) td.appendChild(renderChip("Claude Code"));
        break;
      case "model":
        if (!isOverflow) td.appendChild(renderChip(row.node.model || "—"));
        break;
      case "task":
        td.className = "agents-cell-task";
        td.textContent = isOverflow ? "" : row.node.task || "—";
        break;
      case "started":
        td.textContent = isOverflow ? "" : row.node.startedAgo || "—";
        break;
      case "active":
        td.textContent = isOverflow ? "" : row.node.activeAgo;
        break;
      case "status":
        // Amendement A6, point 3 : rien pour la ligne de débordement (comme les autres colonnes
        // d'info) ; sinon pastille « running » + boutons désactivés (bus MQTT absent).
        if (!isOverflow) td.appendChild(renderStatusCell());
        break;
      case "remaining":
        // DA-T6 : la ligne de débordement affiche aussi un tiret (pas de barre), contrairement
        // aux autres colonnes qui restent simplement vides pour cette ligne.
        td.appendChild(renderRemainingCell(row.node));
        break;
    }
    tr.appendChild(td);
  }

  return tr;
}

function renderTable(sessions: SessionNode[]): HTMLElement {
  const table = document.createElement("table");
  // Amendement A4, point 1 : classe "compact" pour le mode 5 colonnes ; amendement A6, point 1 :
  // plus de `table-layout: fixed` ni de largeurs en pourcentage — la largeur de la FENÊTRE
  // s'ajuste au contenu (cf. `measureContentWidth` + `main.ts`), pas l'inverse.
  table.className = compact ? "agents-table compact" : "agents-table";
  const thead = document.createElement("thead");
  thead.appendChild(headerRow());
  table.appendChild(thead);
  const tbody = document.createElement("tbody");
  const columnCount = visibleColumns().length;
  for (const row of flattenGrouped(sessions)) {
    tbody.appendChild(row.kind === "group" ? groupRowElement(row.project, columnCount) : agentRow(row));
  }
  table.appendChild(tbody);
  return table;
}

function bindHeaderSort(container: HTMLElement, onResort: () => void): void {
  container.querySelectorAll("th[data-col]").forEach((th) => {
    th.addEventListener("click", () => {
      const col = (th as HTMLElement).dataset.col as SortCol;
      sort = sort && sort.col === col ? { col, dir: sort.dir === 1 ? -1 : 1 } : { col, dir: 1 };
      onResort();
    });
  });
}

/** Rend le tableau des agents en cours dans `#agents`, et le compteur dans `#count`. État vide
 * (DA-4) : « Aucun agent en cours » — la liste EST l'app, une fenêtre vide serait ambiguë. */
export function renderAgents(snap: AgentsSnapshot): void {
  lastSnapshot = snap;
  const section = document.getElementById("agents");
  const count = document.getElementById("count");
  if (count) {
    count.textContent = `${snap.count} agent${snap.count === 1 ? "" : "s"}`;
  }
  if (!section) return;
  section.replaceChildren();

  if (snap.sessions.length === 0) {
    const p = document.createElement("p");
    p.className = "empty";
    p.textContent = "Aucun agent en cours";
    section.appendChild(p);
    return;
  }

  const sessions = sortedSnapshot(snap);
  const table = renderTable(sessions);
  section.appendChild(table);
  bindHeaderSort(table, () => {
    if (lastSnapshot) renderAgents(lastSnapshot);
  });

  if (snap.overflowSessions !== null) {
    const more = document.createElement("div");
    more.className = "agents-more";
    more.textContent = `+${snap.overflowSessions} session(s)`;
    section.appendChild(more);
  }
}

/** Mesure la largeur NATURELLE (« max-content ») du tableau actuellement peint (amendement A6,
 * point 1) : seule la webview connaît le rendu réel des polices, donc seule elle peut mesurer.
 * Passe temporairement le tableau en largeur libre, mesure son rendu, puis restaure — même
 * technique que `fitMockupWindow` de la maquette V4 de Loki. La colonne « Tâche » (seule à
 * pouvoir ellipser, cf. `.agents-cell-task` dans styles.css) garde son `max-width` pendant la
 * mesure : elle contribue à la largeur naturelle par sa borne, pas par le texte entier. Retourne
 * `null` si aucun tableau n'est peint (état vide). Le résultat est envoyé tel quel à Rust
 * (`fit_window_width`), qui borne (480–1200 px) et applique l'hystérésis — cette fonction ne
 * décide de rien, elle mesure. */
export function measureContentWidth(): number | null {
  const table = document.querySelector<HTMLTableElement>("#agents table");
  if (!table) return null;
  const previous = table.style.width;
  table.style.width = "max-content";
  const width = Math.ceil(table.getBoundingClientRect().width);
  table.style.width = previous;
  return width;
}
