#!/usr/bin/env node
// iaka-agents-publish.mjs — publieur Claude Code du contrat de bus iaka v2
// (specs/contrat-bus-iaka.md ; fiche : specs/instructions/agnostique-mqtt.md, étape 9). Node >= 18, AUCUNE dépendance (client MQTT
// 3.1.1 minimal écrit à la main, patron repris de ~/.claude/delegation-guard.mjs).
//
// Appelé par des hooks Claude Code (hook_event_name lu sur stdin) : SessionStart,
// UserPromptSubmit, Stop, SessionEnd, SubagentStart, SubagentStop, PostToolUse. Publie/efface des
// fiches sur `iaka/<node>/out/agents/claude-code/<sessionId>/<agentId>` (retenues, QoS 1) — jamais
// ailleurs. Toujours code de sortie 0, jamais de sortie sur stdout (DA-M3) : ce script ne doit
// JAMAIS ralentir ni bloquer une session Claude Code, même si le broker est absent ou lent.
//
// Contrat consommé : specs/instructions/agnostique-mqtt.md § « Contrat sur le fil ».
// Ce fichier est le SEUL, avec sources/claude_code_files.rs, où le nom "claude-code" a le droit
// d'être écrit en dur (grille de neutralité).

import net from "node:net";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

// ============================== Constantes (DA-M*) ==============================

const RUNNER_ID = "claude-code";
const MQTT_HOST = "127.0.0.1"; // DA-M2 : jamais configurable côté publieur non plus.

/** Port MQTT, lu à CHAQUE appel (jamais figé à l'import) — pour rester testable contre un faux
 * serveur écoutant sur un port choisi dynamiquement (cf. test/publish.test.mjs). */
export function mqttPort() {
  return parseInt(process.env.IAKAAM_MQTT_PORT || "1883", 10);
}
const TOTAL_BUDGET_MS = 1500; // DA-M3 : budget total du script, connexion + publication comprise.
const HEARTBEAT_MIN_INTERVAL_MS = 20_000; // Q2-A : au plus une publication toutes les 20 s/agent.
const MAX_ENRICH_ATTEMPTS = 5; // DA-M5 : au plus 5 relectures du compagnon tant que task est vide.
const TASK_MAX_CHARS = 200; // contrat § Fiche, champ task.
const COORDINATOR_SNIPPET_MAX_CHARS = 60; // DA-M6, même borne que A3 point 11 côté app.
const CACHE_ROOT = path.join(os.homedir(), ".cache", "iaka-agents"); // DA-M4.
const SEGMENT_RE = /^[A-Za-z0-9._-]{1,128}$/; // contrat § Topics.

// ============================== Utilitaires purs (testables) ==============================

/** Un seul point d'écriture pour "maintenant", en secondes epoch (contrat : toutes les dates du
 * fil sont en secondes UTC, cohérent avec le contrat conso). */
export function nowSecs() {
  return Math.floor(Date.now() / 1000);
}

/** Un segment de topic valide (contrat § Topics : `[A-Za-z0-9._-]{1,128}`, jamais `+`/`#`/`/`). */
export function isValidSegment(s) {
  return typeof s === "string" && SEGMENT_RE.test(s);
}

/** Réduit un texte à un libellé d'une ligne, tronqué à `maxChars` (miroir de `truncate_task` côté
 * Rust) : retours à la ligne aplatis, "…" si tronqué. */
export function truncateOneLine(text, maxChars) {
  if (typeof text !== "string") return "";
  const oneLine = text.split(/\s+/).filter(Boolean).join(" ");
  const chars = Array.from(oneLine);
  if (chars.length <= maxChars) return oneLine;
  return chars.slice(0, maxChars).join("") + "…";
}

/** Étiquette de ligne de consigne, casse et accent indifférents (miroir minimal de la grammaire
 * de temps-restant.md, Q1). Le publieur n'a PAS à valider la valeur (Q5-A) : il extrait le texte
 * brut qui suit l'étiquette, l'app seule valide avec sa grammaire (une seule grammaire). */
const CONSIGNE_LABEL_RE = /^(?:dur[ée]e)\s+estim[ée]e\s*:\s*(.+?)\s*$/i;

/** Cherche la ligne d'estimation parmi les 5 premières lignes NON VIDES d'une consigne, renvoie
 * le texte BRUT de la valeur (`"~10 min"`), sans validation (Q5-A). `null` si absente. */
export function findConsigneEstimateRaw(consigne) {
  if (typeof consigne !== "string" || consigne.length === 0) return null;
  const lines = consigne
    .split(/\r?\n/)
    .filter((l) => l.trim().length > 0)
    .slice(0, 5);
  for (const line of lines) {
    const m = line.trim().match(CONSIGNE_LABEL_RE);
    if (m) return m[1].trim();
  }
  return null;
}

/** Extrait le texte BRUT du suffixe `"(~10 min)"` d'un libellé court, SANS le retirer (le libellé
 * affiché reste celui donné par le runner, contrat § Fiche champ `task`). `null` si absent. */
export function suffixEstimateRaw(label) {
  if (typeof label !== "string") return null;
  const trimmed = label.trimEnd();
  const window = trimmed.slice(-40);
  const m = window.match(/\(\s*~\s*([^()]*)\)\s*$/);
  if (!m) return null;
  const value = m[1].trim();
  return value.length > 0 ? `~${value}` : null;
}

/** Estimation annoncée (priorité à la ligne de consigne, repli sur le suffixe du libellé court,
 * Q1-A/Q5-A) — même priorité que `estimate::resolve` côté Rust, en amont de sa validation. */
export function resolveEstimateRaw(consigne, shortLabel) {
  return findConsigneEstimateRaw(consigne) ?? suffixEstimateRaw(shortLabel) ?? null;
}

// ============================== État du publieur (DA-M4) ==============================

function sessionDir(sessionId) {
  return path.join(CACHE_ROOT, sessionId);
}

function stateFilePath(sessionId, agentId) {
  return path.join(sessionDir(sessionId), `${agentId}.json`);
}

/** Lit l'état d'un agent (fiche courante + compteurs). `null` si absent/illisible/invalide —
 * jamais de panique : un état perdu ne fait que réinitialiser les compteurs. */
export function readState(sessionId, agentId) {
  try {
    const raw = fs.readFileSync(stateFilePath(sessionId, agentId), "utf8");
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

/** Écrit l'état d'un agent (best effort : une erreur d'écriture ne doit jamais faire échouer le
 * hook — DA-M3, jamais bloquant). */
export function writeState(sessionId, agentId, state) {
  try {
    fs.mkdirSync(sessionDir(sessionId), { recursive: true, mode: 0o700 });
    fs.writeFileSync(stateFilePath(sessionId, agentId), JSON.stringify(state));
  } catch {
    // best effort — DA-M3.
  }
}

/** Supprime le dossier d'ÉTAT complet d'une session (DA-M4 : "dossier de session supprimé au
 * SessionEnd"). Best effort. */
export function removeSessionState(sessionId) {
  try {
    fs.rmSync(sessionDir(sessionId), { recursive: true, force: true });
  } catch {
    // best effort.
  }
}

// ============================== Lecture du compagnon (DA-M5) ==============================

/** Chemin du compagnon `agent-<id>.meta.json`, à côté du transcript du sous-agent (même
 * convention que `sources/claude_code_files.rs`, `path.with_extension("meta.json")`). */
function sidecarPathFor(transcriptPath) {
  if (typeof transcriptPath !== "string") return null;
  return transcriptPath.replace(/\.[^/.]+$/, ".meta.json");
}

/** Lit le compagnon d'un sous-agent. `{}` si absent/illisible/invalide (R3 : pas encore écrit au
 * SubagentStart — DA-M5 relit aux battements suivants). */
export function readCompanion(transcriptPath) {
  const sidecar = sidecarPathFor(transcriptPath);
  if (!sidecar) return {};
  try {
    const raw = fs.readFileSync(sidecar, "utf8");
    const v = JSON.parse(raw);
    return {
      description: typeof v.description === "string" ? v.description : null,
      parentAgentId: typeof v.parentAgentId === "string" ? v.parentAgentId : null,
      model: typeof v.model === "string" ? v.model : null,
    };
  } catch {
    return {};
  }
}

/** Première ligne utilisateur réelle (hors balise de service) du transcript d'UN sous-agent,
 * lue bornée (1 MiB, même plafond que côté app) — sert à extraire l'estimation par la ligne de
 * consigne (Q1-A) quand le compagnon seul n'en donne pas le moyen. Best effort. */
export function firstUserLineOfTranscript(transcriptPath) {
  if (typeof transcriptPath !== "string") return null;
  try {
    const fd = fs.openSync(transcriptPath, "r");
    const cap = 1024 * 1024;
    const buf = Buffer.alloc(cap);
    const n = fs.readSync(fd, buf, 0, cap, 0);
    fs.closeSync(fd);
    const head = buf.toString("utf8", 0, n);
    for (const line of head.split("\n")) {
      if (!line.trim()) continue;
      let v;
      try {
        v = JSON.parse(line);
      } catch {
        continue;
      }
      if (v.type !== "user") continue;
      const content = v.message && v.message.content;
      let text = null;
      if (typeof content === "string") text = content;
      else if (Array.isArray(content)) {
        const block = content.find((b) => b && b.type === "text" && typeof b.text === "string");
        if (block) text = block.text;
      }
      if (!text || !text.trim() || text.trim().startsWith("<")) continue;
      return text;
    }
  } catch {
    // best effort.
  }
  return null;
}

// ============================== Client MQTT minimal (patron delegation-guard.mjs) ==============

function remainingLength(n) {
  const out = [];
  do {
    let d = n % 128;
    n = Math.floor(n / 128);
    if (n > 0) d |= 0x80;
    out.push(d);
  } while (n > 0);
  return Buffer.from(out);
}

function mstr(s) {
  const b = Buffer.from(s, "utf8");
  return Buffer.concat([Buffer.from([b.length >> 8, b.length & 0xff]), b]);
}

function connectPacket(clientId) {
  // Protocole MQTT 3.1.1, connexion ANONYME (broker iakahub, DA-M2), session propre.
  const variableHeader = Buffer.concat([mstr("MQTT"), Buffer.from([4, 0x02, 0, 30])]);
  const payload = mstr(clientId);
  const body = Buffer.concat([variableHeader, payload]);
  return Buffer.concat([Buffer.from([0x10]), remainingLength(body.length), body]);
}

/** PUBLISH QoS 1, retenu. `packetId` : identifiant de paquet (1-65535). */
function publishPacket(topic, payloadBuf, packetId) {
  const header = Buffer.concat([mstr(topic), Buffer.from([packetId >> 8, packetId & 0xff])]);
  const body = Buffer.concat([header, payloadBuf]);
  // 0x30 | retain(0x01) | QoS1(0x02) = 0x33.
  return Buffer.concat([Buffer.from([0x33]), remainingLength(body.length), body]);
}

/** Publie une fiche (ou une suppression, `payloadBuf` vide) sur `topic`, budget `budgetMs`
 * restant. Résout TOUJOURS (jamais de rejet) : `"ok"`, `"refused"`, `"timeout"` ou `"error"` —
 * fail-open (DA-M3), l'appelant ignore la valeur de résolution sauf pour les tests.
 */
export function publishOnce({ host, port, topic, payloadBuf, budgetMs }) {
  return new Promise((resolve) => {
    let done = false;
    const finish = (r) => {
      if (done) return;
      done = true;
      resolve(r);
    };
    let sock;
    const timer = setTimeout(() => {
      try {
        sock && sock.destroy();
      } catch {
        /* noop */
      }
      finish("timeout");
    }, Math.max(1, budgetMs));

    try {
      const clientId = `${RUNNER_ID}-pub-${process.pid}-${Math.floor(Math.random() * 1e6)}`;
      const packetId = 1 + Math.floor(Math.random() * 65534);
      sock = net.connect({ host, port });
      let connacked = false;
      sock.on("error", () => {
        clearTimeout(timer);
        try {
          sock.destroy();
        } catch {
          /* noop */
        }
        finish("error");
      });
      sock.on("connect", () => {
        try {
          sock.write(connectPacket(clientId));
        } catch {
          /* noop */
        }
      });
      sock.on("data", (d) => {
        if (!connacked) {
          if (d[0] !== 0x20) return;
          connacked = true;
          if (d.length >= 4 && d[3] !== 0) {
            clearTimeout(timer);
            try {
              sock.destroy();
            } catch {
              /* noop */
            }
            return finish("refused");
          }
          try {
            sock.write(publishPacket(topic, payloadBuf, packetId));
          } catch {
            clearTimeout(timer);
            try {
              sock.destroy();
            } catch {
              /* noop */
            }
            finish("error");
          }
          return;
        }
        // Attente du PUBACK (0x40) portant le même identifiant — sinon on considère la
        // publication faite dès l'écriture (le broker a reçu les octets) : fail-open, ne jamais
        // bloquer plus que le budget.
        if (d[0] === 0x40) {
          clearTimeout(timer);
          try {
            sock.end();
          } catch {
            /* noop */
          }
          finish("ok");
        }
      });
    } catch {
      clearTimeout(timer);
      finish("error");
    }
  });
}

// ============================== Construction d'une fiche ==============================

/** Construit la fiche JSON (contrat § Fiche) — jamais de champ manquant sur les obligatoires.
 * `extra` porte les champs optionnels déjà résolus par l'appelant (persona, model, task, cwd,
 * estimate, project). */
export function buildCard({ agentId, parentId, sessionId, startedAt, lastSeenAt, state, extra = {} }) {
  return {
    v: 1,
    runner: RUNNER_ID,
    runnerVersion: extra.runnerVersion ?? null,
    model: extra.model ?? null,
    persona: extra.persona ?? null,
    agentId,
    parentId,
    sessionId,
    project: extra.project ?? null,
    cwd: extra.cwd ?? null,
    task: extra.task != null ? truncateOneLine(extra.task, TASK_MAX_CHARS) : null,
    estimate: extra.estimate ?? null,
    startedAt,
    lastSeenAt,
    state,
  };
}

/** Contrat v2 par. 1 : forme normalisée du `<node>` (même règle que fleet/src/shared/contract.js et
 * src-tauri/src/contract.rs, prouvée par contract-vectors/node-normalization.json). `null` si refusé
 * (vide, ou contenant `+ # /`) : jamais de publication sur un topic mal formé. */
export function normalizeNode(raw) {
  if (typeof raw !== "string" || /[+#/]/.test(raw)) return null;
  const node = raw
    .trim()
    .toLowerCase()
    .replace(/\.local$/, "")
    .replace(/[^a-z0-9-]/gu, "-");
  return node === "" || /^-+$/.test(node) ? null : node;
}

/** Valeur de `key` dans `~/.iakafleet/.env` (lecture seule, tolérante) — même fichier que le démon,
 * pour que le `<node>` du bus local et celui du central restent identiques. */
function envFileValue(key) {
  try {
    const text = fs.readFileSync(path.join(os.homedir(), ".iakafleet", ".env"), "utf8");
    for (const raw of text.split(/\r?\n/)) {
      const line = raw.trim();
      const eq = line.indexOf("=");
      if (line.startsWith("#") || eq < 1 || line.slice(0, eq).trim() !== key) continue;
      const v = line.slice(eq + 1).trim().replace(/^(["'])(.*)\1$/, "$2");
      if (v !== "") return v;
    }
  } catch {
    /* fichier absent ou illisible : repli sur le nom d'hôte */
  }
  return undefined;
}

/** `<node>` de cette machine : IAKA_NODE (env puis ~/.iakafleet/.env), alias IAKAFLEET_HOST, sinon
 * le nom d'hôte. Lu à CHAQUE appel. `null` si la valeur est refusée. */
export function nodeId() {
  const raw =
    process.env.IAKA_NODE || envFileValue("IAKA_NODE") ||
    process.env.IAKAFLEET_HOST || envFileValue("IAKAFLEET_HOST") || os.hostname();
  return normalizeNode(raw);
}

/** Topic d'une fiche : `iaka/<node>/out/agents/<runner>/<sid>/<agentId>` (bus local : `<sid>` = sessionId brut). */
export function agentTopic(node, runner, sid, agentId) {
  return `iaka/${node}/out/agents/${runner}/${sid}/${agentId}`;
}

/** Publie une fiche (ou une suppression) si et seulement si les segments du topic sont valides
 * (contrat § Topics — jamais de publication sur un topic mal formé). `deadline` = horodatage
 * epoch ms au-delà duquel on n'essaie même plus (budget global du script, DA-M3). */
async function publishCardOrDelete(sessionId, agentId, cardOrNull, deadline) {
  if (!isValidSegment(sessionId) || !isValidSegment(agentId)) return;
  const node = nodeId();
  if (node === null) return; // <node> refusé : jamais de publication sur un topic mal formé
  const remaining = deadline - Date.now();
  if (remaining <= 0) return;
  const payloadBuf = cardOrNull ? Buffer.from(JSON.stringify(cardOrNull), "utf8") : Buffer.alloc(0);
  await publishOnce({
    host: MQTT_HOST,
    port: mqttPort(),
    topic: agentTopic(node, RUNNER_ID, sessionId, agentId),
    payloadBuf,
    budgetMs: remaining,
  });
}

// ============================== Routage des événements de hook ==============================

/** DA-M7 : `SubagentStart`/`SubagentStop` à `agentType` vide sont des fantômes (#91910) — ignorés
 * AVANT toute connexion. */
function isGhostSubagentEvent(payload) {
  return typeof payload.agent_type !== "string" || payload.agent_type.trim() === "";
}

/** Traite un événement de hook déjà parsé. `deadline` = horodatage epoch ms limite (DA-M3).
 * `clockNow` (secondes epoch) est un point d'injection d'horloge RÉSERVÉ AUX TESTS (limitation
 * de débit Q2-A, enrichissement DA-M5) — en production, toujours `nowSecs()`. Ne lève jamais :
 * toute erreur est absorbée, le script sort en 0 dans tous les cas (`main`). */
export async function handleEvent(payload, deadline, clockNow) {
  const hookEvent = payload.hook_event_name;
  const sessionId = payload.session_id;
  if (typeof sessionId !== "string" || sessionId.length === 0) return;
  const now = typeof clockNow === "number" ? clockNow : nowSecs();

  switch (hookEvent) {
    case "SessionStart": {
      const card = buildCard({
        agentId: "main",
        parentId: null,
        sessionId,
        startedAt: now,
        lastSeenAt: now,
        state: "running",
        extra: { cwd: payload.cwd, task: "coordinateur" },
      });
      writeState(sessionId, "main", { card, lastPublishedAt: now });
      await publishCardOrDelete(sessionId, "main", card, deadline);
      return;
    }
    case "UserPromptSubmit": {
      const prior = readState(sessionId, "main");
      const startedAt = prior?.card?.startedAt ?? now;
      const task = truncateOneLine(payload.prompt || "", COORDINATOR_SNIPPET_MAX_CHARS);
      const card = buildCard({
        agentId: "main",
        parentId: null,
        sessionId,
        startedAt,
        lastSeenAt: now,
        state: "running",
        extra: { cwd: payload.cwd, task: task || "coordinateur" },
      });
      writeState(sessionId, "main", { card, lastPublishedAt: now });
      await publishCardOrDelete(sessionId, "main", card, deadline);
      return;
    }
    case "Stop": {
      const prior = readState(sessionId, "main");
      const startedAt = prior?.card?.startedAt ?? now;
      const task = prior?.card?.task ?? "coordinateur";
      const card = buildCard({
        agentId: "main",
        parentId: null,
        sessionId,
        startedAt,
        lastSeenAt: now,
        state: "idle",
        extra: { cwd: payload.cwd, task },
      });
      writeState(sessionId, "main", { card, lastPublishedAt: now });
      await publishCardOrDelete(sessionId, "main", card, deadline);
      return;
    }
    case "SessionEnd": {
      await publishCardOrDelete(sessionId, "main", null, deadline);
      removeSessionState(sessionId);
      return;
    }
    case "SubagentStart": {
      if (isGhostSubagentEvent(payload)) return; // DA-M7 — AUCUNE connexion.
      const agentId = payload.agent_id;
      if (typeof agentId !== "string" || agentId.length === 0) return;
      const companion = readCompanion(payload.transcript_path);
      const consigne = firstUserLineOfTranscript(payload.transcript_path);
      const estimate = resolveEstimateRaw(consigne, companion.description);
      const card = buildCard({
        agentId,
        parentId: companion.parentAgentId || "main",
        sessionId,
        startedAt: now,
        lastSeenAt: now,
        state: "running",
        extra: {
          persona: payload.agent_type,
          model: companion.model,
          task: companion.description,
          cwd: payload.cwd,
          estimate,
        },
      });
      writeState(sessionId, agentId, { card, lastPublishedAt: now, enrichAttempts: 1 });
      await publishCardOrDelete(sessionId, agentId, card, deadline);
      return;
    }
    case "SubagentStop": {
      if (isGhostSubagentEvent(payload)) return; // DA-M7 — AUCUNE connexion.
      const agentId = payload.agent_id;
      if (typeof agentId !== "string" || agentId.length === 0) return;
      await publishCardOrDelete(sessionId, agentId, null, deadline);
      return;
    }
    case "PostToolUse": {
      const agentId = typeof payload.agent_id === "string" && payload.agent_id ? payload.agent_id : "main";
      const prior = readState(sessionId, agentId);
      // Q2-A : au plus une publication toutes les 20 s par agent — sortie immédiate (lecture
      // fichier seule, AUCUN réseau) si la dernière publication est trop récente (R5).
      if (prior && now - Math.floor((prior.lastPublishedAt ?? 0)) < HEARTBEAT_MIN_INTERVAL_MS / 1000) {
        return;
      }
      const startedAt = prior?.card?.startedAt ?? now;
      let task = prior?.card?.task ?? null;
      let estimate = prior?.card?.estimate ?? null;
      let model = prior?.card?.model ?? null;
      let parentId = prior?.card?.parentId ?? (agentId === "main" ? null : "main");
      let persona = prior?.card?.persona ?? (agentId === "main" ? null : payload.agent_type ?? null);
      const attempts = prior?.enrichAttempts ?? 0;

      // DA-M5 : relecture du compagnon tant que la tâche est vide, au plus 5 fois (R3 : pas
      // encore écrit au SubagentStart).
      let nextAttempts = attempts;
      if (agentId !== "main" && !task && attempts < MAX_ENRICH_ATTEMPTS) {
        const companion = readCompanion(payload.transcript_path);
        if (companion.description) task = companion.description;
        if (companion.model) model = companion.model;
        if (companion.parentAgentId) parentId = companion.parentAgentId;
        if (!estimate) {
          const consigne = firstUserLineOfTranscript(payload.transcript_path);
          estimate = resolveEstimateRaw(consigne, task);
        }
        nextAttempts = attempts + 1;
      }

      const card = buildCard({
        agentId,
        parentId,
        sessionId,
        startedAt,
        lastSeenAt: now,
        state: "running",
        extra: { persona, model, task: task ?? (agentId === "main" ? "coordinateur" : null), cwd: payload.cwd, estimate },
      });
      writeState(sessionId, agentId, { card, lastPublishedAt: now, enrichAttempts: nextAttempts });
      await publishCardOrDelete(sessionId, agentId, card, deadline);
      return;
    }
    default:
      return; // Événement non reconnu -> rien à publier, jamais d'erreur.
  }
}

// ============================== Lecture de stdin + main ==============================

function readStdin() {
  return new Promise((resolve) => {
    let data = "";
    let settled = false;
    const finish = (v) => {
      if (settled) return;
      settled = true;
      resolve(v);
    };
    try {
      process.stdin.setEncoding("utf8");
      process.stdin.on("data", (chunk) => {
        data += chunk;
      });
      process.stdin.on("end", () => finish(data));
      process.stdin.on("error", () => finish(data));
      // Si stdin n'est pas un flux (pas de TTY, pas de pipe) : ne jamais rester bloqué.
      setTimeout(() => finish(data), 500);
    } catch {
      finish("");
    }
  });
}

async function main() {
  const deadline = Date.now() + TOTAL_BUDGET_MS;
  try {
    const raw = await readStdin();
    if (!raw) return;
    let payload;
    try {
      payload = JSON.parse(raw);
    } catch {
      return;
    }
    await handleEvent(payload, deadline);
  } catch {
    // DA-M3 : jamais d'exception qui remonte, jamais de sortie non nulle.
  }
}

// N'exécute `main` que si ce fichier est le point d'entrée (permet l'import des fonctions pures
// depuis les tests sans déclencher de connexion réseau ni de lecture de stdin).
const isMainModule = process.argv[1] && import.meta.url === `file://${process.argv[1]}`;
if (isMainModule) {
  main().finally(() => {
    process.exitCode = 0; // DA-M3 : toujours 0, quoi qu'il arrive.
  });
}
