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

import type {
  AgentNode,
  AgentsSnapshot,
  FleetAgent,
  FleetMachine,
  FleetSession,
  RemoteView,
  SessionNode,
} from "./types";

/** Colonnes triables du tableau. "runner" et "status" n'en font PAS partie : "runner" est
 * désormais un libellé résolu côté Rust (agnostique-mqtt.md, étape 8), mais un seul lot MQTT de
 * base ne fait cohabiter qu'un runner à la fois en pratique — pas de tri jugé utile ici pour
 * l'instant ; "status" (colonne « État ») n'a aujourd'hui qu'un seul état réel possible (running,
 * amendement A6 point 3, boutons désactivés tant que l'amendement A1 de controle n'est pas livré)
 * — rien à trier.
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
 * A6 : le tri ne porte plus QUE sur la fratrie À L'INTÉRIEUR d'un groupe de dépôt — les groupes
 * eux-mêmes ne se réordonnent plus entre eux (cf. `sortedGroups`). */
function sortChildrenRecursively(node: AgentNode, col: SortCol, dir: 1 | -1): void {
  node.children.sort((a, b) => compareNodes(col, dir, a, b));
  for (const child of node.children) sortChildrenRecursively(child, col, dir);
}

/** Un groupe = un dépôt (amendement A6, point 2). PLUSIEURS sessions peuvent partager le même
 * projet (ex. deux terminaux ouverts sur le même dépôt) : elles rejoignent alors le MÊME groupe
 * — un seul en-tête par nom de projet, jamais deux lignes de séparation consécutives pour le
 * même dépôt — et leurs coordinateurs deviennent des racines FRÈRES de ce groupe (`roots`),
 * triables entre elles comme n'importe quelle fratrie. */
interface Group {
  project: string;
  roots: AgentNode[];
  /** Index de couleur stable du dépôt (amendement A7, point 3), déjà résolu côté Rust — même
   * valeur pour toutes les sessions d'un même projet (fonction pure du nom, cf. `agents.rs`). */
  color: number;
}

/** Regroupe les sessions par nom de projet, en conservant l'ordre de PREMIÈRE apparition (déjà
 * déterministe côté backend, activité décroissante) — c'est cet ordre de GROUPES qui reste
 * stable au clic sur un en-tête (amendement A6, point 2 : le tri ne porte que sur l'intérieur
 * d'un groupe, jamais sur l'ordre des groupes entre eux). */
function groupByProject(sessions: SessionNode[]): Group[] {
  const order: string[] = [];
  const roots = new Map<string, AgentNode[]>();
  const colors = new Map<string, number>();
  for (const s of sessions) {
    if (!roots.has(s.project)) {
      roots.set(s.project, []);
      colors.set(s.project, s.groupColor);
      order.push(s.project);
    }
    roots.get(s.project)!.push(structuredClone(s.coordinator));
  }
  return order.map((project) => ({ project, roots: roots.get(project)!, color: colors.get(project)! }));
}

/** Trie chaque groupe indépendamment : ses racines (coordinateurs) entre elles, puis
 * récursivement leurs descendants — jamais les groupes entre eux (amendement A6, point 2). */
function sortedGroups(snap: AgentsSnapshot): Group[] {
  const groups = groupByProject(snap.sessions);
  if (!sort) return groups;
  const { col, dir } = sort;
  for (const g of groups) {
    g.roots.sort((a, b) => compareNodes(col, dir, a, b));
    for (const root of g.roots) sortChildrenRecursively(root, col, dir);
  }
  return groups;
}

interface AgentRow {
  kind: "agent";
  node: AgentNode;
  depth: number;
  /** Guide ASCII de délégation (├─ / └─ / │, amendement A3 point 4) : vide à la racine (depth
   * 0 — les racines d'un même groupe ne portent pas de guide entre elles, seuls leurs
   * descendants en affichent). */
  guide: string;
}

/** Ligne de séparation pleine largeur portant le nom du dépôt (amendement A6, point 2 ; couleur
 * stable par dépôt, amendement A7 point 3). */
interface GroupRow {
  kind: "group";
  project: string;
  color: number;
}

type TableRow = AgentRow | GroupRow;

/** Aplatit les groupes en lignes (amendement A6, point 2) : une `GroupRow` par dépôt, suivie de
 * TOUTES les racines (coordinateurs) de ce dépôt et de leurs arbres de délégation — même
 * algorithme que la maquette V4 de Loki (`buildTreeRows` / `buildGroupedLiveRows`) : un parcours
 * en profondeur où chaque fratrie connaît sa propre longueur (pour distinguer le dernier enfant,
 * `└─`, des autres, `├─`) et transmet un préfixe cumulé (`│  ` sous un frère non-dernier, `   `
 * sous le dernier) à ses propres enfants. */
function flattenGrouped(groups: Group[]): TableRow[] {
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
  for (const g of groups) {
    rows.push({ kind: "group", project: g.project, color: g.color });
    walk(g.roots, 0, "");
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
 * mode courant. Couleur stable par dépôt (amendement A7, point 3) : la classe
 * `agents-group-row--c<index>` porte le fond teinté, le liseré gauche et le texte en couleur —
 * toutes les valeurs de couleur vivent dans `styles.css`, l'index seul vient de Rust. */
function groupRowElement(project: string, color: number, columnCount: number): HTMLElement {
  const tr = document.createElement("tr");
  tr.className = `agents-group-row agents-group-row--c${color}`;
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
        // Libellé déjà résolu côté Rust (agnostique-mqtt.md, étape 8) : plus de constante figée.
        if (!isOverflow) td.appendChild(renderChip(row.node.runner || "—"));
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

function renderTable(groups: Group[]): HTMLElement {
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
  for (const row of flattenGrouped(groups)) {
    tbody.appendChild(
      row.kind === "group" ? groupRowElement(row.project, row.color, columnCount) : agentRow(row),
    );
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

  const groups = sortedGroups(snap);
  const table = renderTable(groups);
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

// ---- « Autres machines » (vue distante, instruction de fusion D4) --------------------------------
// Tableau V4 existant (mêmes jetons, aucun nouveau composant), GROUPÉ PAR MACHINE : ligne de groupe
// (nom, OS, état), puis sessions, puis agents indentés sous leur session quand leur <sid> s'y
// rattache, sinon « agents non rattachés ». Tout est déjà résolu côté Rust (libellés, états).
// L'absence ne se déguise jamais : central injoignable ⇒ « broker injoignable : <motif> », jamais
// une section vide ; texte de tâche non publié ⇒ « tâche masquée », jamais une cellule vide.

const FLEET_COLUMNS = ["Session / agent", "Projet", "État", "Tâche", "Actif"];

function fleetCell(text: string, className = ""): HTMLElement {
  const td = document.createElement("td");
  if (className) td.className = className;
  td.textContent = text;
  return td;
}

function fleetNote(text: string, className: string): HTMLElement {
  const p = document.createElement("p");
  p.className = className;
  p.textContent = text;
  return p;
}

function fleetSpanRow(text: string, className: string): HTMLElement {
  const tr = document.createElement("tr");
  tr.className = className;
  const td = document.createElement("td");
  td.colSpan = FLEET_COLUMNS.length;
  td.textContent = text;
  tr.appendChild(td);
  return tr;
}

function fleetAgentRow(a: FleetAgent, guide: string): HTMLElement {
  const tr = document.createElement("tr");
  tr.className = a.live ? "" : "fleet-row--stale";
  const first = document.createElement("td");
  first.className = "agents-cell-agent";
  const g = document.createElement("span");
  g.className = "tree-guide";
  g.textContent = guide;
  first.appendChild(g);
  const who = document.createElement("span");
  who.className = "agents-persona";
  who.textContent = a.persona ?? a.agentId;
  first.appendChild(who);
  tr.appendChild(first);
  tr.appendChild(fleetCell(a.project ?? "—"));
  tr.appendChild(fleetCell(a.stateLabel));
  tr.appendChild(
    a.taskMasked
      ? fleetCell("tâche masquée", "agents-cell-task fleet-task--masked")
      : fleetCell(a.task ?? "—", "agents-cell-task"),
  );
  tr.appendChild(fleetCell(a.activeAgo));
  return tr;
}

function fleetSessionRow(s: FleetSession): HTMLElement {
  const tr = document.createElement("tr");
  tr.className = s.live ? "" : "fleet-row--stale";
  const where = s.repo ? `${s.repo}@${s.branch ?? "—"}` : (s.cwd ?? "—");
  tr.appendChild(fleetCell(s.name ? `${s.pid} · ${s.name}` : String(s.pid), "agents-cell-agent"));
  tr.appendChild(fleetCell(where));
  tr.appendChild(fleetCell(s.statusLabel));
  tr.appendChild(fleetCell(""));
  tr.appendChild(fleetCell(""));
  return tr;
}

function fleetMachineRows(m: FleetMachine, index: number): HTMLElement[] {
  const rows: HTMLElement[] = [];
  rows.push(
    fleetSpanRow(
      `${m.node} · ${m.os ?? "?"} · ${m.stateLabel}`,
      `agents-group-row agents-group-row--c${index % 6}`,
    ),
  );
  for (const alert of m.alerts) rows.push(fleetSpanRow(`! ${alert}`, "fleet-alert"));
  if (m.sessions.length === 0 && m.unattachedAgents.length === 0) {
    rows.push(fleetSpanRow("(aucune session publiée)", "fleet-none"));
  }
  for (const s of m.sessions) {
    rows.push(fleetSessionRow(s));
    s.agents.forEach((a, i) => rows.push(fleetAgentRow(a, i === s.agents.length - 1 ? "└─" : "├─")));
  }
  if (m.unattachedAgents.length > 0) {
    rows.push(fleetSpanRow("agents non rattachés", "fleet-none"));
    m.unattachedAgents.forEach((a) => rows.push(fleetAgentRow(a, "·")));
  }
  return rows;
}

/** Rend la section « Autres machines ». Cachée tant qu'aucun central n'est configuré (le Monitor
 * marche seul : rien ne change sans configuration). */
export function renderFleet(view: RemoteView): void {
  const section = document.getElementById("fleet");
  if (!section) return;
  section.replaceChildren();
  if (view.status === "not_configured") {
    section.hidden = true;
    return;
  }
  section.hidden = false;
  const title = document.createElement("div");
  title.className = "fleet-title";
  title.textContent = "Autres machines";
  section.appendChild(title);

  if (view.status === "unreachable") {
    section.appendChild(fleetNote(`broker injoignable : ${view.reason ?? "motif inconnu"}`, "fleet-notice fleet-notice--down"));
  } else if (view.status === "connecting") {
    section.appendChild(fleetNote("connexion au broker…", "fleet-notice"));
  }

  if (view.machines.length === 0) {
    if (view.status === "connected") {
      section.appendChild(fleetNote("Aucune autre machine n'a publié", "empty"));
    }
    return;
  }
  if (view.status === "unreachable") {
    section.appendChild(fleetNote("dernier état connu :", "fleet-notice"));
  }
  const table = document.createElement("table");
  table.className = "agents-table compact fleet-table";
  const thead = document.createElement("thead");
  const hr = document.createElement("tr");
  for (const label of FLEET_COLUMNS) {
    const th = document.createElement("th");
    th.className = "not-sortable";
    th.textContent = label;
    hr.appendChild(th);
  }
  thead.appendChild(hr);
  table.appendChild(thead);
  const tbody = document.createElement("tbody");
  view.machines.forEach((m, i) => fleetMachineRows(m, i).forEach((r) => tbody.appendChild(r)));
  table.appendChild(tbody);
  section.appendChild(table);
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
  const tables = document.querySelectorAll<HTMLTableElement>("#agents table, #fleet table");
  if (tables.length === 0) return null;
  let width = 0;
  tables.forEach((table) => {
    const previous = table.style.width;
    table.style.width = "max-content";
    width = Math.max(width, Math.ceil(table.getBoundingClientRect().width));
    table.style.width = previous;
  });
  return width;
}

/** Mesure la hauteur NATURELLE du contenu peint (amendement A7, point 1) : même principe que
 * `measureContentWidth`, appliqué à la hauteur — la section `#agents` a un `max-height` CSS
 * (`.agents`, défilement au-delà) qui empêcherait de mesurer le contenu réel ; on la lève
 * temporairement, on mesure, on restaure. Additionne la hauteur de la barre du haut et du bas
 * (topbar/footbar, hors de `#agents`) : c'est la hauteur TOTALE de fenêtre qui doit s'ajuster, pas
 * seulement celle du tableau. Toujours mesuré, même à l'état vide (DA-4, message « Aucun agent en
 * cours ») — contrairement à `measureContentWidth`, qui renvoie `null` faute de tableau : une
 * fenêtre vide doit aussi se réduire à sa hauteur minimale, pas garder la dernière hauteur pleine.
 * `null` uniquement si la structure de page attendue est absente (défensif). */
export function measureContentHeight(): number | null {
  const section = document.getElementById("agents");
  if (!section) return null;
  const topbar = document.querySelector<HTMLElement>(".topbar");
  const footbar = document.querySelector<HTMLElement>(".footbar");
  const previous = section.style.maxHeight;
  section.style.maxHeight = "none";
  const contentHeight = section.scrollHeight;
  section.style.maxHeight = previous;
  // « Autres machines » : sa hauteur s'ajoute à celle du tableau local (section absente = 0).
  const fleet = document.getElementById("fleet");
  const fleetHeight = fleet && !fleet.hidden ? fleet.getBoundingClientRect().height : 0;
  const chromeHeight =
    (topbar?.getBoundingClientRect().height ?? 0) + (footbar?.getBoundingClientRect().height ?? 0);
  return Math.ceil(contentHeight + fleetHeight + chromeHeight);
}
