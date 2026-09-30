// publish.test.mjs — tests du publieur Claude Code (specs/instructions/agnostique-mqtt.md,
// étape 10). `node --test`, aucune dépendance. Fait broker en `node:net` (fake-broker.mjs) pour
// vérifier topic/retenu/fiche ; un vrai sous-processus pour le cas "broker absent".

import { test, beforeEach, afterEach } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { startFakeBroker } from "./fake-broker.mjs";
import {
  handleEvent,
  readState,
  writeState,
  removeSessionState,
  truncateOneLine,
  findConsigneEstimateRaw,
  suffixEstimateRaw,
} from "../iaka-agents-publish.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SCRIPT_PATH = path.join(__dirname, "..", "iaka-agents-publish.mjs");

function uniqueSessionId(tag) {
  return `test-${tag}-${Date.now()}-${Math.floor(Math.random() * 1e6)}`;
}

let broker;
let usedSessionIds = [];

beforeEach(async () => {
  broker = await startFakeBroker();
  process.env.IAKAAM_MQTT_PORT = String(broker.port);
  process.env.IAKA_NODE = "testnode";
  usedSessionIds = [];
});

afterEach(async () => {
  await broker.close();
  for (const sid of usedSessionIds) removeSessionState(sid);
});

function deadline(ms = 1500) {
  return Date.now() + ms;
}

// ---------------------------- Utilitaires purs ----------------------------

test("truncateOneLine aplatit et tronque avec des points de suspension", () => {
  assert.equal(truncateOneLine("a\nb   c", 100), "a b c");
  assert.equal(truncateOneLine("x".repeat(70), 60), "x".repeat(60) + "…");
});

test("findConsigneEstimateRaw trouve la ligne, casse et accent indifferents", () => {
  assert.equal(findConsigneEstimateRaw("Durée estimée : ~10 min\nsuite"), "~10 min");
  assert.equal(findConsigneEstimateRaw("Duree estimee: 10 min"), "10 min");
  assert.equal(findConsigneEstimateRaw("rien ici"), null);
});

test("suffixEstimateRaw extrait le suffixe sans retirer le libelle", () => {
  assert.equal(suffixEstimateRaw("Tache (~10 min)"), "~10 min");
  assert.equal(suffixEstimateRaw("Legolas passe 2 sur 3 (relance)"), null);
});

// ---------------------------- SessionStart / UserPromptSubmit / Stop ----------------------------

test("SessionStart publie une fiche main retenue sur le bon topic", async () => {
  const sessionId = uniqueSessionId("start");
  usedSessionIds.push(sessionId);
  await handleEvent({ hook_event_name: "SessionStart", session_id: sessionId, cwd: "/tmp/proj" }, deadline());

  assert.equal(broker.events.publishes.length, 1);
  const pub = broker.events.publishes[0];
  assert.equal(pub.topic, `iaka/testnode/out/agents/claude-code/${sessionId}/main`);
  assert.equal(pub.retain, true);
  assert.equal(pub.qos, 1);
  const card = JSON.parse(pub.payload);
  assert.equal(card.v, 1);
  assert.equal(card.runner, "claude-code");
  assert.equal(card.agentId, "main");
  assert.equal(card.parentId, null);
  assert.equal(card.state, "running");
});

test("UserPromptSubmit met la tache = debut du message, tronque a 60 caracteres", async () => {
  const sessionId = uniqueSessionId("prompt");
  usedSessionIds.push(sessionId);
  await handleEvent({ hook_event_name: "SessionStart", session_id: sessionId }, deadline());
  await handleEvent(
    { hook_event_name: "UserPromptSubmit", session_id: sessionId, prompt: "x".repeat(120) },
    deadline(),
  );
  const last = broker.events.publishes.at(-1);
  const card = JSON.parse(last.payload);
  assert.equal(card.task, "x".repeat(60) + "…");
  assert.equal(card.state, "running");
});

test("Stop passe l'etat du coordinateur a idle", async () => {
  const sessionId = uniqueSessionId("stop");
  usedSessionIds.push(sessionId);
  await handleEvent({ hook_event_name: "SessionStart", session_id: sessionId }, deadline());
  await handleEvent({ hook_event_name: "Stop", session_id: sessionId }, deadline());
  const last = broker.events.publishes.at(-1);
  const card = JSON.parse(last.payload);
  assert.equal(card.state, "idle");
});

test("SessionEnd publie un payload vide retenu et supprime l'etat", async () => {
  const sessionId = uniqueSessionId("end");
  usedSessionIds.push(sessionId);
  await handleEvent({ hook_event_name: "SessionStart", session_id: sessionId }, deadline());
  await handleEvent({ hook_event_name: "SessionEnd", session_id: sessionId }, deadline());

  const last = broker.events.publishes.at(-1);
  assert.equal(last.topic, `iaka/testnode/out/agents/claude-code/${sessionId}/main`);
  assert.equal(last.payload, "");
  assert.equal(last.retain, true);
  assert.equal(readState(sessionId, "main"), null);
});

// ---------------------------- SubagentStart / SubagentStop ----------------------------

test("SubagentStart publie une fiche avec parentId = main par defaut", async () => {
  const sessionId = uniqueSessionId("substart");
  usedSessionIds.push(sessionId);
  await handleEvent(
    {
      hook_event_name: "SubagentStart",
      session_id: sessionId,
      agent_id: "abc123",
      agent_type: "gimli",
      transcript_path: "/inexistant/agent-abc123.jsonl",
    },
    deadline(),
  );
  const pub = broker.events.publishes.at(-1);
  const card = JSON.parse(pub.payload);
  assert.equal(card.agentId, "abc123");
  assert.equal(card.parentId, "main");
  assert.equal(card.persona, "gimli");
});

test("SubagentStop publie un payload vide retenu pour cet agent seulement", async () => {
  const sessionId = uniqueSessionId("substop");
  usedSessionIds.push(sessionId);
  await handleEvent(
    { hook_event_name: "SubagentStart", session_id: sessionId, agent_id: "abc", agent_type: "gimli" },
    deadline(),
  );
  await handleEvent(
    { hook_event_name: "SubagentStop", session_id: sessionId, agent_id: "abc", agent_type: "gimli" },
    deadline(),
  );
  const last = broker.events.publishes.at(-1);
  assert.equal(last.topic, `iaka/testnode/out/agents/claude-code/${sessionId}/abc`);
  assert.equal(last.payload, "");
  assert.equal(last.retain, true);
});

test("agent_type vide (fantome #91910) -> AUCUNE connexion, ni SubagentStart ni SubagentStop", async () => {
  const sessionId = uniqueSessionId("fantome");
  usedSessionIds.push(sessionId);
  await handleEvent(
    { hook_event_name: "SubagentStart", session_id: sessionId, agent_id: "ghost1", agent_type: "" },
    deadline(),
  );
  await handleEvent(
    { hook_event_name: "SubagentStop", session_id: sessionId, agent_id: "ghost2", agent_type: undefined },
    deadline(),
  );
  assert.equal(broker.events.connects, 0);
  assert.equal(broker.events.publishes.length, 0);
});

// ---------------------------- PostToolUse (limitation de debit, Q2-A) ----------------------------

test("deux PostToolUse rapproches -> une seule publication (limitation 20 s)", async () => {
  const sessionId = uniqueSessionId("rate");
  usedSessionIds.push(sessionId);
  const t0 = 1_000_000;
  await handleEvent(
    { hook_event_name: "PostToolUse", session_id: sessionId, agent_id: "main" },
    deadline(),
    t0,
  );
  await handleEvent(
    { hook_event_name: "PostToolUse", session_id: sessionId, agent_id: "main" },
    deadline(),
    t0 + 5, // 5 s plus tard : bien sous les 20 s -> pas de deuxième publication.
  );
  assert.equal(broker.events.publishes.length, 1);
});

test("deux PostToolUse espaces de plus de 20 s -> deux publications", async () => {
  const sessionId = uniqueSessionId("rate-ok");
  usedSessionIds.push(sessionId);
  const t0 = 1_000_000;
  await handleEvent(
    { hook_event_name: "PostToolUse", session_id: sessionId, agent_id: "main" },
    deadline(),
    t0,
  );
  await handleEvent(
    { hook_event_name: "PostToolUse", session_id: sessionId, agent_id: "main" },
    deadline(),
    t0 + 25, // 25 s plus tard : au-delà des 20 s -> republie.
  );
  assert.equal(broker.events.publishes.length, 2);
});

test("compagnon absent puis present -> tache remplie au battement PostToolUse suivant", async () => {
  const sessionId = uniqueSessionId("enrich");
  usedSessionIds.push(sessionId);
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "iaka-am-pub-"));
  const transcriptPath = path.join(tmpDir, "agent-xyz.jsonl");
  fs.writeFileSync(transcriptPath, "");

  const t0 = 2_000_000;
  await handleEvent(
    {
      hook_event_name: "SubagentStart",
      session_id: sessionId,
      agent_id: "xyz",
      agent_type: "gimli",
      transcript_path: transcriptPath,
    },
    deadline(),
    t0,
  );
  let card = JSON.parse(broker.events.publishes.at(-1).payload);
  assert.equal(card.task, null, "compagnon absent au demarrage -> pas de tache");

  // Le compagnon apparaît (écrit par Claude Code après coup, R3) ; le battement suivant arrive
  // plus de 20 s après (au-delà de la limitation de débit, Q2-A).
  fs.writeFileSync(
    path.join(tmpDir, "agent-xyz.meta.json"),
    JSON.stringify({ description: "Fais le travail", parentAgentId: "main" }),
  );
  await handleEvent(
    {
      hook_event_name: "PostToolUse",
      session_id: sessionId,
      agent_id: "xyz",
      transcript_path: transcriptPath,
    },
    deadline(),
    t0 + 25,
  );
  card = JSON.parse(broker.events.publishes.at(-1).payload);
  assert.equal(card.task, "Fais le travail");

  fs.rmSync(tmpDir, { recursive: true, force: true });
});

// ---------------------------- Broker absent ----------------------------

test("broker absent -> le script sort en code 0 en moins de 2 s", () => {
  const sessionId = uniqueSessionId("absent");
  const start = Date.now();
  const result = spawnSync(process.execPath, [SCRIPT_PATH], {
    input: JSON.stringify({ hook_event_name: "SessionStart", session_id: sessionId }),
    env: { ...process.env, IAKAAM_MQTT_PORT: "18839" }, // port improbable, rien n'écoute.
    timeout: 5000,
  });
  const elapsedMs = Date.now() - start;
  removeSessionState(sessionId); // le sous-processus écrit l'état local malgré l'échec réseau.
  assert.equal(result.status, 0);
  assert.equal(result.stdout.toString(), "", "jamais de sortie sur stdout (DA-M3)");
  assert.ok(elapsedMs < 2000, `trop lent : ${elapsedMs} ms`);
});

// ---------------------------- État (fichier) ----------------------------

test("writeState puis readState round-trip", () => {
  const sessionId = uniqueSessionId("state");
  usedSessionIds.push(sessionId);
  writeState(sessionId, "main", { card: { agentId: "main" }, lastPublishedAt: 123 });
  const state = readState(sessionId, "main");
  assert.equal(state.card.agentId, "main");
  assert.equal(state.lastPublishedAt, 123);
});

test("readState d'une session inexistante renvoie null", () => {
  assert.equal(readState("inexistante-xyz", "main"), null);
});

// ---------------------------- <node> (contrat v2) ----------------------------

test("IAKA_NODE refusé (contient /) -> aucune publication, jamais de topic mal formé", async () => {
  process.env.IAKA_NODE = "a/b";
  const sessionId = uniqueSessionId("badnode");
  usedSessionIds.push(sessionId);
  await handleEvent({ hook_event_name: "SessionStart", session_id: sessionId, cwd: "/tmp/proj" }, deadline());
  assert.equal(broker.events.publishes.length, 0);
});

test("IAKA_NODE normalisé dans le topic (majuscules, .local)", async () => {
  process.env.IAKA_NODE = "MBP-de-sjupin.local";
  const sessionId = uniqueSessionId("nodenorm");
  usedSessionIds.push(sessionId);
  await handleEvent({ hook_event_name: "SessionStart", session_id: sessionId, cwd: "/tmp/proj" }, deadline());
  assert.equal(broker.events.publishes[0].topic, `iaka/mbp-de-sjupin/out/agents/claude-code/${sessionId}/main`);
});
