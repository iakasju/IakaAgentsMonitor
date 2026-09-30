// plan-courante.mjs — Émetteur de PLAN VIVANT sur la main courante (L18, tranche 1).
//
// Généralise le hook L5 (delegation-guard) : ici on instrumente le PLAN de l'agent.
// Câblé sur PostToolUse, matcher "TodoWrite" (et "Task"). À chaque écriture de todos,
// on ÉMET un snapshot COMPLET (non tronqué) du plan vers la base de documents / le broker (IAKALOG_* / DOCDB_*,
// même schéma que L4/L5). Le cockpit lit la main courante et affiche la checklist
// (dernier snapshot = plan courant).
//
// JAMAIS bloquant (ce n'est pas un garde) : fail-open, borné, exit 0 toujours.
// Transport : IAKALOG_TRANSPORT = "broker" (défaut) | "docdb". Scopé par session/cwd.
import { homedir } from "node:os";
import { join, basename } from "node:path";
import { readFileSync } from "node:fs";
import net from "node:net";

const EMIT_TIMEOUT_MS = 1500;
const ts = () => new Date().toISOString();
const done = () => process.exit(0); // émetteur : on ne refuse jamais.

function readAll() {
  try {
    return readFileSync(0, "utf8");
  } catch {
    return "";
  }
}

await main();

async function main() {
  try {
    const raw = readAll();
    if (!raw.trim()) done();
    let p;
    try {
      p = JSON.parse(raw);
    } catch {
      done();
      return;
    }
    const tool = p.tool_name || p.toolName || "";
    if (tool !== "TodoWrite" && tool !== "Task") {
      done();
      return;
    }
    const ti = p.tool_input || {};
    const items = normalizeItems(tool, ti);
    if (items.length === 0) {
      done();
      return;
    }
    await withTimeout(emitPlan(p, items), EMIT_TIMEOUT_MS);
  } catch {
    /* fail-open */
  }
  done();
}

// Normalise le payload en items {content, status} (TodoWrite = liste ; Task = 1 item).
function normalizeItems(tool, ti) {
  if (tool === "TodoWrite" && Array.isArray(ti.todos)) {
    return ti.todos
      .map((t) => ({
        content: String(t.content ?? t.activeForm ?? "").trim(),
        status: String(t.status ?? "pending"),
      }))
      .filter((t) => t.content.length > 0);
  }
  if (tool === "Task") {
    const c = String(ti.description ?? ti.subagent_type ?? "").trim();
    return c ? [{ content: c, status: "in_progress" }] : [];
  }
  return [];
}

function summarize(items) {
  const done_ = items.filter((i) => i.status === "completed").length;
  const run = items.filter((i) => i.status === "in_progress").length;
  return `Plan (${items.length}) : ${done_} fait(s), ${run} en cours`;
}

async function emitPlan(p, items) {
  const session = p.session_id || null;
  const cwd = p.cwd || process.env.CLAUDE_PROJECT_DIR || process.cwd();
  const conv = basename(cwd || "") || session || "default";
  const agent = process.env.IAKALOG_AGENT || "coordinateur";
  const royaume = process.env.IAKALOG_ROYAUME || "unknown";
  const at = ts();
  const meta = { canal: "geste", event: "plan", tool: p.tool_name, items, cwd };
  const _id = `plan-${session || conv}-${at}`;
  const doc = {
    _id,
    role: "system",
    content: summarize(items),
    ts: at,
    tokens: 0,
    meta,
    royaume,
    agent,
    conv_id: conv,
  };
  const transport = (process.env.IAKALOG_TRANSPORT || "broker").toLowerCase();
  if (transport === "docdb") return emitDocDb(doc);
  return emitBroker(doc, royaume, agent, conv);
}

// base de documents : POST {DOCDB_URL}/{db} Basic auth (calque bridge/index.js). 409 = succès.
async function emitDocDb(doc) {
  if (!process.env.DOCDB_URL) return;
  const base = String(process.env.DOCDB_URL).replace(/\/$/, "");
  const db = process.env.DOCDB_DB || "conversations";
  const user = process.env.DOCDB_USER || "";
  const pass = process.env.DOCDB_PASSWORD || "";
  const auth =
    process.env.DOCDB_AUTH ||
    "Basic " + Buffer.from(`${user}:${pass}`).toString("base64");
  try {
    await fetch(`${base}/${db}`, {
      method: "POST",
      headers: { "content-type": "application/json", authorization: auth },
      body: JSON.stringify(doc),
    });
  } catch {
    /* fail-open */
  }
}

// broker : publie sur le topic IAKALOG_PREFIX/<royaume>/<agent>/<conv> (calque iakalog.mjs). _id dans le payload.
function emitBroker(doc, royaume, agent, conv) {
  if (!(process.env.IAKALOG_USER && process.env.IAKALOG_PASS)) return Promise.resolve();
  const url = process.env.IAKALOG_BROKER_URL || "";
  const m = url.match(/^\w+:\/\/([^:/]+):?(\d+)?/);
  if (!m) return Promise.resolve();
  const host = m[1];
  const port = Number(m[2] || 1883);
  const prefix = process.env.IAKALOG_PREFIX || "logs";
  const topic = `${prefix}/${royaume}/${agent}/${conv}`;
  // Best-effort : on tente une connexion TCP courte ; si indisponible, fail-open.
  return new Promise((resolve) => {
    const sock = net.connect({ host, port }, () => {
      // Publication broker minimale non implémentée ici (le bridge broker reste la voie interne) :
      // en l'absence de lib broker, on s'appuie sur le transport base de documents en recette.
      sock.end();
      resolve();
    });
    sock.on("error", () => resolve());
    sock.setTimeout(400, () => {
      sock.destroy();
      resolve();
    });
  });
}

function withTimeout(promise, ms) {
  return Promise.race([
    Promise.resolve(promise),
    new Promise((r) => setTimeout(r, ms)),
  ]);
}
