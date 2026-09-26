// Rendu du tableau des agents en cours (amendement A1 du 26/09/2026, remplace la popover V1 de
// la maquette de référence ; amendement A3 du 26/09/2026, alignement visuel sur l'onglet V4 de
// Loki) : une ligne par agent, colonnes triables, INDENTÉE par profondeur de délégation (chaque
// enfant sous son parent, décalé d'un niveau) avec des guides ASCII (├─ └─ │). Toute la logique
// (roster, liveness, arbre, formatage des durées, capitalisation des personas, contraste) est
// déjà résolue côté Rust (D9) : ce module ne fait que peindre et trier — le tri ne réinterprète
// aucune donnée, il compare des champs déjà résolus.
//
// Référence de maquette : docs/maquettes/visu-agents-5-versions.html, onglet V4 (tableau dense +
// aperçu produit 520×360), adaptée ici avec indentation hiérarchique au lieu d'une liste plate.

import type { AgentNode, AgentsSnapshot, SessionNode } from "./types";

/** Colonnes triables du tableau. "runner" et "statut" n'en font PAS partie : "runner" est une
 * constante pour ce lot (Q6-A : macOS + Claude Code seul), "statut" aussi de fait — le backend
 * n'inclut JAMAIS un agent mort dans l'instantané (D2 : "session sans agent vivant -> absente"),
 * donc toute ligne affichée est "vivant" ; trier cette colonne serait un no-op. */
type SortCol = "agent" | "project" | "task" | "active" | "started" | "model";

interface SortState {
  col: SortCol;
  dir: 1 | -1;
}

// Tri par défaut : activité la plus récente (déjà l'ordre transmis par Rust côté sessions), pas
// de ré-échange nécessaire tant que l'utilisateur n'a pas cliqué un en-tête.
let sort: SortState | null = null;
let lastSnapshot: AgentsSnapshot | null = null;

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

/** Badge de statut (amendement A3, point 3) : pastille + « vivant » — la seule valeur possible
 * dans ce tableau (cf. note sur `SortCol` ci-dessus). */
function renderStatusBadge(): HTMLElement {
  const el = document.createElement("span");
  el.className = "badge b-live";
  const dot = document.createElement("span");
  dot.className = "badge-dot";
  el.append(dot, document.createTextNode("vivant"));
  return el;
}

function sortValue(col: SortCol, project: string, node: AgentNode): string | number {
  switch (col) {
    case "agent":
      return node.persona.toLowerCase();
    case "project":
      return project.toLowerCase();
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

function compareNodes(col: SortCol, dir: 1 | -1, project: string, a: AgentNode, b: AgentNode): number {
  const va = sortValue(col, project, a);
  const vb = sortValue(col, project, b);
  if (va < vb) return -1 * dir;
  if (va > vb) return 1 * dir;
  return 0;
}

/** Trie RÉCURSIVEMENT les enfants d'un nœud par la colonne choisie, sans jamais casser
 * l'adjacence parent → enfant (l'indentation reste toujours vraie, même après un clic de tri) :
 * on ne fait que réordonner les FRÈRES à chaque niveau, jamais les niveaux entre eux. */
function sortChildrenRecursively(node: AgentNode, project: string, col: SortCol, dir: 1 | -1): void {
  node.children.sort((a, b) => compareNodes(col, dir, project, a, b));
  for (const child of node.children) sortChildrenRecursively(child, project, col, dir);
}

function sortedSnapshot(snap: AgentsSnapshot): SessionNode[] {
  const sessions = snap.sessions.map((s) => ({
    project: s.project,
    coordinator: structuredClone(s.coordinator),
  }));
  if (!sort) return sessions;
  const { col, dir } = sort;
  sessions.sort((a, b) => compareNodes(col, dir, a.project, a.coordinator, b.coordinator));
  for (const s of sessions) sortChildrenRecursively(s.coordinator, s.project, col, dir);
  return sessions;
}

interface Row {
  project: string;
  node: AgentNode;
  depth: number;
  /** Guide ASCII de délégation (├─ / └─ / │, amendement A3 point 4) : vide à la racine (depth
   * 0, un coordinateur par session — pas de fratrie à représenter). */
  guide: string;
}

/** Aplatit l'arbre en lignes tout en calculant, pour chaque nœud, son guide de délégation :
 * même algorithme que la maquette V4 de Loki (`buildTreeRows`) — un parcours en profondeur où
 * chaque fratrie connaît sa propre longueur (pour distinguer le dernier enfant, `└─`, des
 * autres, `├─`) et transmet un préfixe cumulé (`│  ` sous un frère non-dernier, `   ` sous le
 * dernier) à ses propres enfants. */
function flatten(sessions: SessionNode[]): Row[] {
  const rows: Row[] = [];
  function walk(project: string, nodes: AgentNode[], depth: number, prefix: string): void {
    nodes.forEach((node, i) => {
      const isLast = i === nodes.length - 1;
      const guide = depth === 0 ? "" : prefix + (isLast ? "└─ " : "├─ ");
      rows.push({ project, node, depth, guide });
      if (node.children.length > 0) {
        const childPrefix = depth === 0 ? "" : prefix + (isLast ? "   " : "│  ");
        walk(project, node.children, depth + 1, childPrefix);
      }
    });
  }
  for (const s of sessions) walk(s.project, [s.coordinator], 0, "");
  return rows;
}

/** Ordre des colonnes = celui de l'amendement A3, point 6 : Agent · Projet · Runner · Modèle ·
 * Tâche · Démarré il y a · Actif il y a · Statut. */
type ColumnDef = { id: SortCol | "runner" | "statut"; label: string; sortable: boolean };
const COLUMNS: ColumnDef[] = [
  { id: "agent", label: "Agent", sortable: true },
  { id: "project", label: "Projet", sortable: true },
  { id: "runner", label: "Runner", sortable: false },
  { id: "model", label: "Modèle", sortable: true },
  { id: "task", label: "Tâche", sortable: true },
  { id: "started", label: "Démarré il y a", sortable: true },
  { id: "active", label: "Actif il y a", sortable: true },
  { id: "statut", label: "Statut", sortable: false },
];

function headerRow(): HTMLElement {
  const tr = document.createElement("tr");
  for (const { id, label, sortable } of COLUMNS) {
    const th = document.createElement("th");
    th.textContent = label;
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

function agentRow(row: Row): HTMLElement {
  const tr = document.createElement("tr");
  tr.className = row.node.overflow !== null ? "agents-overflow-row" : "";
  tr.title = row.node.tooltip;
  const isOverflow = row.node.overflow !== null;

  for (const { id } of COLUMNS) {
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
      case "project":
        td.textContent = isOverflow ? "" : row.project;
        break;
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
      case "statut":
        if (!isOverflow) td.appendChild(renderStatusBadge());
        break;
    }
    tr.appendChild(td);
  }

  return tr;
}

function renderTable(sessions: SessionNode[]): HTMLElement {
  const table = document.createElement("table");
  table.className = "agents-table";
  const thead = document.createElement("thead");
  thead.appendChild(headerRow());
  table.appendChild(thead);
  const tbody = document.createElement("tbody");
  for (const row of flatten(sessions)) tbody.appendChild(agentRow(row));
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
