// Vecteurs partages (contract-vectors/) — la part du publieur : normalisation de <node> et
// construction du topic de fiche (specs/contrat-bus-iaka.md par. 6). Le hachage du <sid> n'est pas
// dans le publieur : sur le bus local, le sessionId reste brut (hors couverture declaree).
import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { normalizeNode, agentTopic } from "../iaka-agents-publish.mjs";

const dir = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "contract-vectors");
const load = (n) => JSON.parse(fs.readFileSync(path.join(dir, n), "utf8"));

test("vecteurs : normalisation de <node>", () => {
  const v = load("node-normalization.json");
  assert.ok(v.ok.length > 0 && v.refused.length > 0);
  for (const c of v.ok) assert.equal(normalizeNode(c.input), c.expected, c.input);
  for (const bad of v.refused) assert.equal(normalizeNode(bad), null, JSON.stringify(bad));
});

test("vecteurs : topic de fiche d'agent", () => {
  for (const c of load("topics.json").build.agent) {
    assert.equal(agentTopic(c.node, c.runner, c.sid, c.agentId), c.expected);
  }
});
