// Rendu du tableau des agents en cours (amendement A1 du 26/09/2026, remplace la popover V1 de
// la maquette de référence) : une ligne par agent, colonnes triables, INDENTÉE par profondeur de
// délégation (chaque enfant sous son parent, décalé d'un niveau). Toute la logique (roster,
// liveness, arbre, formatage des durées) est déjà résolue côté Rust (D9) : ce module ne fait que
// peindre et trier — le tri ne réinterprète aucune donnée, il compare des champs déjà résolus.
//
// Référence de maquette : docs/maquettes/visu-agents-5-versions.html, version 4 (tableau dense),
// adaptée ici avec indentation hiérarchique au lieu d'une liste plate.

import type { AgentNode, AgentsSnapshot, SessionNode } from "./types";

/** Colonnes triables du tableau (amendement A1). "runner" n'en fait PAS partie : c'est une
 * constante pour ce lot (Q6-A : macOS + Claude Code seul), affichée mais non triable. */
type SortCol = "agent" | "project" | "task" | "active" | "started" | "model";

interface SortState {
  col: SortCol;
  dir: 1 | -1;
}

// Tri par défaut : activité la plus récente (déjà l'ordre transmis par Rust côté sessions), pas
// de ré-échange nécessaire tant que l'utilisateur n'a pas cliqué un en-tête.
let sort: SortState | null = null;
let lastSnapshot: AgentsSnapshot | null = null;

/** Point d'extension UNIQUE pour l'icône d'un agent (amendement A1) : aujourd'hui une lettre dans
 * un carré à la couleur de sa pastille. Loki proposera des icônes pixel art 4×4 ; ce sera alors
 * la SEULE fonction à changer pour les faire apparaître dans le tableau. */
function renderSpriteIcon(node: AgentNode): HTMLElement {
  const el = document.createElement("span");
  el.className = "sprite";
  el.style.background = node.sprite.bg;
  el.style.color = node.sprite.fg;
  el.textContent = node.sprite.letter;
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
}

function flatten(sessions: SessionNode[]): Row[] {
  const rows: Row[] = [];
  function walk(project: string, node: AgentNode, depth: number): void {
    rows.push({ project, node, depth });
    for (const child of node.children) walk(project, child, depth + 1);
  }
  for (const s of sessions) walk(s.project, s.coordinator, 0);
  return rows;
}

/** Ordre des colonnes = celui de l'amendement A1 (agent, projet, runner, modèle, tâche, actif il
 * y a, démarré il y a). "runner" est la seule colonne non triable (constante pour ce lot,
 * Q6-A : macOS + Claude Code seul). */
type ColumnDef = { id: SortCol | "runner"; label: string; sortable: boolean };
const COLUMNS: ColumnDef[] = [
  { id: "agent", label: "Agent", sortable: true },
  { id: "project", label: "Projet", sortable: true },
  { id: "runner", label: "Runner", sortable: false },
  { id: "model", label: "Modèle", sortable: true },
  { id: "task", label: "Tâche", sortable: true },
  { id: "active", label: "Actif il y a", sortable: true },
  { id: "started", label: "Démarré il y a", sortable: true },
];

function headerRow(): HTMLElement {
  const tr = document.createElement("tr");
  for (const { id, label, sortable } of COLUMNS) {
    const th = document.createElement("th");
    th.textContent = label;
    if (sortable) {
      th.dataset.col = id;
      if (sort && sort.col === id) {
        th.textContent += sort.dir === 1 ? " ▲" : " ▼";
      }
    }
    tr.appendChild(th);
  }
  return tr;
}

/** Contenu texte d'une cellule (hors colonne Agent, rendue à part pour l'indentation + sprite).
 * Une ligne de débordement (« +N agent(s) ») ne remplit que la colonne Agent : le reste est vide. */
function cellText(id: Exclude<ColumnDef["id"], "agent">, row: Row): string {
  if (row.node.overflow !== null) return "";
  switch (id) {
    case "project":
      return row.project;
    case "runner":
      return "Claude Code";
    case "model":
      return row.node.model || "—";
    case "task":
      return row.node.task || "—";
    case "active":
      return row.node.activeAgo;
    case "started":
      return row.node.startedAgo || "—";
  }
}

function agentRow(row: Row): HTMLElement {
  const tr = document.createElement("tr");
  tr.className = row.node.overflow !== null ? "agents-overflow-row" : "";
  tr.title = row.node.tooltip;

  for (const { id } of COLUMNS) {
    if (id === "agent") {
      const agentTd = document.createElement("td");
      agentTd.className = "agents-cell-agent";
      agentTd.style.paddingLeft = `${row.depth * 16}px`;
      agentTd.appendChild(renderSpriteIcon(row.node));
      const name = document.createElement("span");
      name.className = "agents-persona";
      name.textContent = row.node.persona;
      agentTd.appendChild(name);
      tr.appendChild(agentTd);
      continue;
    }
    const td = document.createElement("td");
    if (id === "task") td.className = "agents-cell-task";
    td.textContent = cellText(id, row);
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
    (th as HTMLElement).style.cursor = "pointer";
    th.addEventListener("click", () => {
      const col = (th as HTMLElement).dataset.col as SortCol;
      sort = sort && sort.col === col ? { col, dir: sort.dir === 1 ? -1 : 1 } : { col, dir: 1 };
      onResort();
    });
  });
}

/** Rend le tableau des agents en cours dans `#agents`, et le compteur dans `#count` (amendement
 * A1). État vide (DA-4) : « Aucun agent en cours » — la liste EST l'app, une fenêtre vide serait
 * ambiguë. */
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
