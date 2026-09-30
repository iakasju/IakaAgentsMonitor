#!/usr/bin/env node
// delegation-guard.mjs — Garde iakaframe du canal GESTES (la faille diagnostiquee).
// Cable sur PreToolUse + PostToolUse, matcher "Task" (outil de delegation a un sous-agent).
//
// Pourquoi : les gardes d'identite (identity-guard.mjs) ne lisent que le canal ADRESSE
// (blocs texte) aux frontieres de parole (Stop/SubagentStop). La delegation, elle, est un
// GESTE (tool_use) dont le payload echappe totalement a ces gardes. Ce script pose un garde
// sur le bon canal :
//   - PreToolUse  : journalise l'ALLER verbatim (agent cible + prompt envoye) et verifie que
//                   l'agent cible appartient au roster connu. Rend l'aller auditable.
//   - PostToolUse : journalise le RETOUR verbatim. Rend "restitution verbatim" verifiable
//                   au lieu de reposer sur la seule bonne foi de l'orchestrateur. EMET aussi
//                   (L5) un document MACHINE de delegation sur le canal geste vers la base de logs /
//                   le broker configures par les variables d'environnement IAKALOG_* / DOCDB_*.
//
// MVP honnete : ce garde rend les gestes AUDITABLES (il ne pretend pas policer
// semantiquement les frontieres de role, ce qui n'est pas fiable). FAIL-OPEN : ne bloque
// jamais un travail reel pour un bug interne. Journal : ~/.claude/iakaframe-delegations.log
//
// L5 — Tracage MACHINE des delegations (canal geste -> base de logs / broker via IAKALOG_* / DOCDB_*) :
//   A PostToolUse, si subagent_type est un agent du ROSTER iakaframe, on EMET un document
//   { role:"system", content:"Delegation X -> Y : ...", meta:{canal:"geste", event:"delegation",
//   from, to, verdict?} } vers la base de logs / le broker (IAKALOG_* / DOCDB_*). Best-effort, NON BLOQUANT, fail-open total.
//   Transport selectionnable par IAKALOG_TRANSPORT : "broker" (defaut) | "docdb"
//   (fallback recette offline, POST {DOCDB_URL}/{db} Basic auth, calque sur bridge/index.js).
//   _id deterministe (idempotence ; 409 = succes). Sous-agents natifs -> AUCUNE emission.
//   Borne iakaframe : identite de log (IAKALOG_* / DOCDB_*) absente -> aucune emission, exit 0.

import { appendFileSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import net from "node:net";
import { verdictDelegation, ROSTER, BUILTINS, AGENT_UNSET } from "./guard-core.mjs";

const LOG = join(homedir(), ".claude", "iakaframe-delegations.log");

// Plafond d'attente de l'emission. Le garde est un hook VIVANT (chaque Task, y compris
// la session en cours) : on ne PEND JAMAIS. Au-dela, on abandonne et on laisse passer.
// Declare ICI (avant `await main()`) : sinon TDZ -> ReferenceError avalee = 0 emission.
const EMIT_TIMEOUT_MS = 1500;

const allow = () => process.exit(0);

const ts = () => new Date().toISOString();
const write = (rec) => {
  try { appendFileSync(LOG, JSON.stringify(rec) + "\n", "utf8"); } catch { /* fail-open */ }
};

await main();

async function main() {
  try {
  const raw = readAll();
  if (!raw.trim()) allow();
  let p;
  try { p = JSON.parse(raw); } catch { allow(); return; }
  const event = p.hook_event_name || "";
  const session = p.session_id || null;

  if (event === "PreToolUse") {
    const ti = p.tool_input || {};
    const agent = ti.subagent_type || ti.subagentType || AGENT_UNSET;
    write({
      at: ts(), event: "ALLER", session,
      agent,
      description: ti.description || null,
      prompt: ti.prompt ?? null, // verbatim, jamais reformule
    });
    const { refused } = verdictDelegation(agent);
    if (refused) {
      // Hors roster connu -> REFUS (exit 2). L'ALLER est deja journalise ci-dessus.
      write({ at: ts(), event: "REFUS", session, agent, raison: "hors_roster" });
      // L5 : on EMET aussi la tentative refusee (signal methode), best-effort non bloquant,
      // AVANT le refus (l'emission ne doit jamais empecher le refus de partir).
      // Emission AWAITED + bornee + fail-open AVANT le refus (sinon process.exit la tue).
      await emitDelegation({
        session,
        agent,
        description: ti.description || null,
        response: null,
        atAller: ts(),
        refused: true,
      });
      process.stderr.write(
        "[delegation-guard] Delegation REFUSEE : agent cible hors roster iakaframe : '" + agent +
        "'. Roster autorise : " + ROSTER.join(", ") + " (+ sous-agents natifs : " +
        BUILTINS.join(", ") + "). Corrige subagent_type, ou ajoute l'agent au roster du garde.\n"
      );
      process.exit(2);
    }
    allow();
  }

  if (event === "PostToolUse") {
    const ti = p.tool_input || {};
    const agent = (ti.subagent_type || ti.subagentType) || null;
    const response = extractText(p.tool_response); // verbatim
    write({
      at: ts(), event: "RETOUR", session,
      agent,
      response,
    });
    // L5 : RETOUR = moment d'emission de reference (delegation complete : cible + resultat).
    // Best-effort non bloquant ; n'emet QUE pour le roster iakaframe (anti-bruit D5).
    // AWAITED + bornee : on flushe l'emission AVANT process.exit, sinon 0 doc en base.
    await emitDelegation({
      session,
      agent,
      description: ti.description || null,
      response,
      atAller: ts(),
      refused: false,
    });
    allow();
  }

  allow();
  } catch {
    allow(); // fail-open
  }
}

function readAll() {
  try { return readFileSync(0, "utf8"); }
  catch { return ""; }
}

function extractText(resp) {
  if (resp == null) return null;
  if (typeof resp === "string") return resp;
  // tool_response peut etre un tableau de blocs {type:"text", text:"..."} ou un objet.
  try {
    if (Array.isArray(resp)) {
      const t = resp.filter((c) => c && c.type === "text" && c.text).map((c) => c.text);
      return t.length ? t.join("\n") : JSON.stringify(resp);
    }
    return typeof resp === "object" ? JSON.stringify(resp) : String(resp);
  } catch { return String(resp); }
}

// ---------------------------------------------------------------------------
// L5 — Emission MACHINE de la delegation (canal geste). Tout est fail-open : la
// moindre anomalie est avalee, on ne casse JAMAIS la session ni le garde.
// ---------------------------------------------------------------------------

// Extraction best-effort du verdict (D4 / A4). Renseigne UNIQUEMENT si un marqueur
// explicite PASS/FAIL existe dans la reponse ; absent sinon (pas de parsing intelligent).
function deriveVerdict(response) {
  if (!response || typeof response !== "string") return undefined;
  // Marqueurs explicites : "gate ... PASS", "PASS", "FAIL", "verdict: pass", "KO/OK gate".
  if (/\bgate[^\n]{0,40}\bFAIL\b/i.test(response) || /\bverdict\s*[:=]\s*fail\b/i.test(response)) return "FAIL";
  if (/\bgate[^\n]{0,40}\bPASS\b/i.test(response) || /\bverdict\s*[:=]\s*pass\b/i.test(response)) return "PASS";
  if (/\bFAIL\b/.test(response)) return "FAIL";
  if (/\bPASS\b/.test(response)) return "PASS";
  return undefined;
}

// _id deterministe (idempotence). 409 lors d'un POST = succes silencieux.
function makeDocId(session, to, atAller, refused) {
  const safe = (s) => String(s == null ? "x" : s).replace(/[^a-zA-Z0-9_-]/g, "-");
  const kind = refused ? "refused" : "deleg";
  return `${kind}-${safe(session)}-${safe(atAller)}-${safe(to)}`;
}

// Course fail-open : resout (timeout) au bout de ms quoi qu'il arrive ; jamais de reject.
function withTimeout(promise, ms) {
  return Promise.race([
    Promise.resolve(promise).catch(() => "error"),
    new Promise((resolve) => setTimeout(() => resolve("timeout"), ms)),
  ]).catch(() => "error");
}

// AWAITED + bornee + fail-open. main() doit AWAIT cet appel AVANT process.exit, sinon
// l'emission ne part jamais (le defaut bloquant corrige en L5 : 0 doc en base).
async function emitDelegation({ session, agent, description, response, atAller, refused }) {
  try {
    const to = agent == null ? null : String(agent);
    // D5 anti-bruit : on n'emet QUE pour le roster iakaframe. Les sous-agents natifs
    // (Explore/Plan/general-purpose...) et un agent absent -> AUCUNE emission.
    if (!to || !ROSTER.includes(to.toLowerCase())) {
      // Exception : une tentative refusee (hors roster) PEUT etre emise si demande.
      // refused=true vient d'un agent hors roster ; on l'emet quand meme (signal methode).
      if (!refused) return;
    }

    const from = process.env.IAKALOG_AGENT || "unknown";
    const royaume = process.env.IAKALOG_ROYAUME || "unknown";

    // D6 — Borne iakaframe : sans identite de log (IAKALOG_* / DOCDB_*) configuree, AUCUNE emission.
    const transport = (process.env.IAKALOG_TRANSPORT || "broker").toLowerCase();
    const hasBrokerId = !!(process.env.IAKALOG_USER && process.env.IAKALOG_PASS);
    const hasDocId = !!(process.env.DOCDB_URL &&
      (process.env.DOCDB_USER || process.env.DOCDB_PASSWORD || process.env.DOCDB_AUTH));
    if (transport === "docdb" ? !hasDocId : !hasBrokerId) return;

    const verdict = deriveVerdict(response);
    const conv = session || process.env.IAKALOG_CONV || "default";
    const eventName = refused ? "delegation_refused" : "delegation";
    const arrow = refused ? "(REFUSE) " : "";
    const desc = description ? " : " + description : "";
    const content = `Delegation ${arrow}${from} -> ${to || "(inconnu)"}${desc}`;
    const meta = { canal: "geste", event: eventName, from, to: to || null };
    if (verdict) meta.verdict = verdict;
    if (refused) meta.refused = true;
    const _id = makeDocId(session, to, atAller, refused);

    const doc = { _id, role: "system", content, ts: atAller, tokens: 0, meta, royaume, agent: from, conv_id: conv };

    // AWAITED mais BORNEE : on flushe l'emission, sans jamais depasser EMIT_TIMEOUT_MS.
    if (transport === "docdb") {
      await withTimeout(emitDocDB(doc), EMIT_TIMEOUT_MS);
    } else {
      await withTimeout(emitBroker({ royaume, agent: from, conv, doc }), EMIT_TIMEOUT_MS);
    }
  } catch { /* fail-open : une emission ne doit jamais casser le garde */ }
}

// Transport base de documents : POST {DOCDB_URL}/{db} Basic auth (calque sur bridge/index.js).
// AWAITED : on attend la reponse (flush) ; AbortController borne la requete ; 409 = succes
// (idempotence). Toute erreur est avalee -> fail-open total, jamais de throw propage.
async function emitDocDB(doc) {
  const ctrl = new AbortController();
  const t = setTimeout(() => { try { ctrl.abort(); } catch { /* noop */ } }, EMIT_TIMEOUT_MS);
  try {
    const base = String(process.env.DOCDB_URL).replace(/\/$/, "");
    const db = process.env.DOCDB_DB || "conversations";
    const user = process.env.DOCDB_USER || "";
    const pass = process.env.DOCDB_PASSWORD || "";
    const auth = process.env.DOCDB_AUTH ||
      ("Basic " + Buffer.from(`${user}:${pass}`).toString("base64"));
    // fetch dispo en Node 18+. On AWAIT pour garantir le flush avant process.exit.
    const res = await fetch(`${base}/${db}`, {
      method: "POST",
      headers: { "Content-Type": "application/json", Authorization: auth },
      body: JSON.stringify(doc),
      signal: ctrl.signal,
    });
    // 409 Conflict = le doc (meme _id) existe deja -> succes silencieux (idempotence).
    if (res && res.status === 409) return "exists";
    if (res && !res.ok) return "http-" + res.status;
    return "ok";
  } catch {
    return "error"; // fail-open : timeout/abort/reseau -> on abandonne sans bloquer
  } finally {
    clearTimeout(t);
  }
}

// Transport broker : publie sur le topic IAKALOG_PREFIX/<royaume>/<agent>/<conv> (calque sur iakalog.mjs).
// _id voyage dans le payload ; le bridge l'honore s'il est present. AWAITED + timeout court :
// resout quand le PUBLISH est ecrit OU au timeout, jamais de reject (fail-open).
function emitBroker({ royaume, agent, conv, doc }) {
  return new Promise((resolve) => {
    let done = false;
    const finish = (r) => { if (!done) { done = true; resolve(r); } };
    try {
      const url = process.env.IAKALOG_BROKER_URL || "";
      const u = url.match(/^\w+:\/\/([^:/]+)(?::(\d+))?/);
      if (!u) return finish("bad-url");
      const host = u[1], port = parseInt(u[2] || "1883", 10);
      const USER = process.env.IAKALOG_USER, PASS = process.env.IAKALOG_PASS;
      const prefix = process.env.IAKALOG_PREFIX || "logs";
      const topic = `${prefix}/${royaume}/${agent}/${conv}`;
      const payload = JSON.stringify(doc);

      const remLen = (n) => { const o = []; do { let d = n % 128; n = Math.floor(n / 128); if (n > 0) d |= 0x80; o.push(d); } while (n > 0); return Buffer.from(o); };
      const mstr = (s) => { const b = Buffer.from(s, "utf8"); return Buffer.concat([Buffer.from([b.length >> 8, b.length & 0xff]), b]); };
      const connectPkt = () => {
        const rest = Buffer.concat([mstr(String.fromCharCode(0x4d,0x51,0x54,0x54)), Buffer.from([4, 0xC2, 0, 30]), mstr("delegguard-" + process.pid), mstr(USER), mstr(PASS)]);
        return Buffer.concat([Buffer.from([0x10]), remLen(rest.length), rest]);
      };
      const publishPkt = () => {
        const body = Buffer.concat([mstr(topic), Buffer.from(payload, "utf8")]);
        return Buffer.concat([Buffer.from([0x30]), remLen(body.length), body]);
      };

      const sock = net.connect({ host, port });
      const timer = setTimeout(() => { try { sock.destroy(); } catch { /* noop */ } finish("timeout"); }, EMIT_TIMEOUT_MS);
      sock.on("error", () => { clearTimeout(timer); try { sock.destroy(); } catch { /* noop */ } finish("error"); });
      let connacked = false;
      sock.on("connect", () => { try { sock.write(connectPkt()); } catch { /* noop */ } });
      sock.on("data", (d) => {
        if (connacked || d[0] !== 0x20) return;
        connacked = true;
        if (d[3] !== 0) { clearTimeout(timer); try { sock.destroy(); } catch { /* noop */ } return finish("refused"); }
        try { sock.write(publishPkt(), () => { clearTimeout(timer); try { sock.end(); } catch { /* noop */ } finish("ok"); }); }
        catch { clearTimeout(timer); try { sock.destroy(); } catch { /* noop */ } finish("error"); }
      });
    } catch { finish("error"); }
  });
}
