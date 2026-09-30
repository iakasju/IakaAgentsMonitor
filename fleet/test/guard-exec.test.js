// G-EXEC-2, couche 1 (instruction fusion § 13.1) : garde statique par LISTE BLANCHE de modules.
// Aucun child_process, worker, vm, eval, import dynamique... dans src/ et bin/, quelle que soit la forme.
// Analyse par expressions regulieres sur le texte source, sans dependance AST.
// Hors couverture, declare : le code des dependances (mqtt et son arbre) n'est pas lu ici ; il est couvert
// par la couche 2 (node --permission, test/permission.test.js) et par l'epinglage + lockfile.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readdirSync, readFileSync, statSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');

// Registre MOTIVE des modules importables (pas de liste muette). Ajouter un module = ajouter sa ligne ici.
export const WHITELIST = {
  'node:fs': 'lecture seule des sessions, fiches, .git, .env, package.json (readFileSync, readdirSync, statSync...)',
  'node:path': 'jointure et normalisation de chemins',
  'node:os': 'nom d\'hote, home, release du systeme (informations de presence)',
  'node:crypto': 'sha256 tronque de <sid> (createHash), jamais de chiffrement applicatif',
  mqtt: 'client MQTT : la seule dependance applicative, epinglee + lockfile',
};

// Membres de `process` utilises (registre motive). Tout autre acces (binding, dlopen, process[...], alias) est rouge.
export const PROCESS_MEMBERS = {
  env: 'variables d\'environnement (config)',
  platform: 'os de la presence',
  argv: 'arguments de la CLI',
  stdout: 'sortie de la CLI',
  stderr: 'journal du demon et de la CLI',
  on: 'signaux SIGINT/SIGTERM',
  exit: 'code de sortie',
  version: 'version de node publiee en presence',
  kill: 'kill(pid, 0) : test de vie d\'un pid (aucun signal envoye), pas un lancement de processus',
};

const MODULE_NAMES = ['worker_threads', 'cluster', 'vm', 'child_process', 'module', 'inspector', 'repl', 'wasi'];

// Regles rouges quelle que soit la forme (appliquees au texte brut ET au texte "replie" des litteraux adjacents).
const RULES = [
  { id: 'import-dynamique', re: /\bimport\s*\(/ },
  { id: 'require', re: /\brequire\b/ },
  { id: 'eval', re: /\beval\b/ },
  { id: 'new-function', re: /\bFunction\b/ },
  { id: 'constructor-acces', re: /\.\s*constructor\b/ },
  { id: 'process-binding', re: /\bprocess\s*\.\s*(?:binding|_linkedBinding|dlopen|mainModule|getBuiltinModule|_rawDebug)\b/ },
  { id: 'acces-calcule', re: /\b(?:process|globalThis|global)\s*\[/ },
  { id: 'process-non-liste', re: new RegExp(`\\bprocess\\b(?!\\s*\\.\\s*(?:${Object.keys(PROCESS_MEMBERS).join('|')})\\b)`) },
  { id: 'worker_threads', re: /worker_threads|\bnew\s+Worker\b/ },
  { id: 'child_process', re: /child_process/ },
  ...MODULE_NAMES.filter((m) => m !== 'child_process' && m !== 'worker_threads').map((m) => ({
    id: m,
    re: new RegExp(`(['"\`])(?:node:)?${m}\\1`),
  })),
];
export const RULE_IDS = [...RULES.map((r) => r.id), 'import-non-analysable', 'import-hors-liste-blanche', 'binaire-natif'];

const LIT = '([\'"`])([^\'"`\\\\\\n]*)\\1';

/** Texte "replie" : escapes \xHH / \uHHHH decodes, puis litteraux adjacents ('a' + 'b', 'a'.concat('b')) fusionnes. */
export function fold(text) {
  let t = text
    .replace(/\\u\{([0-9a-fA-F]+)\}/g, (_, h) => String.fromCodePoint(parseInt(h, 16)))
    .replace(/\\[xu]([0-9a-fA-F]{2}(?:[0-9a-fA-F]{2})?)/g, (_, h) => String.fromCharCode(parseInt(h, 16)));
  const plus = new RegExp(`${LIT}\\s*\\+\\s*([\\'"\`])([^\\'"\`\\\\\\n]*)\\3`, 'g');
  const concat = new RegExp(`${LIT}\\s*\\.\\s*concat\\(\\s*([\\'"\`])([^\\'"\`\\\\\\n]*)\\3\\s*\\)`, 'g');
  for (let prev = null; prev !== t; ) {
    prev = t;
    t = t.replace(plus, (_, q, a, _q2, b) => `${q}${a}${b}${q}`).replace(concat, (_, q, a, _q2, b) => `${q}${a}${b}${q}`);
  }
  return t;
}

/** Retourne { rules: Set<id>, specifiers: string[] } pour un texte source. */
export function scanSource(text) {
  const rules = new Set();
  const views = [text, fold(text)].map((t) => t.replace(/process\.kill\.bind\(process\)/g, 'process.kill'));
  for (const v of views) for (const r of RULES) if (r.re.test(v)) rules.add(r.id);

  const base = text.replace(/\bimport\.meta\.url\b/g, '');
  const specifiers = [];
  let staticImports = 0;
  const from = /\b(?:import|export)\b[^;'"`()]*?\bfrom\s*(['"])([^'"\n]+)\1/g;
  const bare = /\bimport\s*(['"])([^'"\n]+)\1/g;
  for (const m of base.matchAll(from)) {
    specifiers.push(m[2]);
    if (m[0].startsWith('import')) staticImports++;
  }
  for (const m of base.matchAll(bare)) {
    specifiers.push(m[2]);
    staticImports++;
  }
  const importTokens = (base.match(/\bimport\b(?!\s*\()/g) ?? []).length;
  if (importTokens !== staticImports) rules.add('import-non-analysable');
  return { rules, specifiers };
}

function walk(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    return statSync(p).isDirectory() ? walk(p) : [p];
  });
}

/** Scanne des dossiers ; retourne [{ file, rule, detail }] et l'ensemble des specifieurs de liste blanche utilises. */
export function scanTree(dirs, whitelist = WHITELIST) {
  const violations = [];
  const used = new Set();
  const roots = dirs.map((d) => resolve(d) + sep);
  for (const file of dirs.flatMap(walk)) {
    if (/\.(node|wasm)$/.test(file)) {
      violations.push({ file, rule: 'binaire-natif', detail: 'module natif/wasm interdit sous src/ et bin/' });
      continue;
    }
    if (!/\.(m?js|cjs)$/.test(file)) continue;
    const { rules, specifiers } = scanSource(readFileSync(file, 'utf8'));
    for (const rule of rules) violations.push({ file, rule, detail: 'forme interdite' });
    for (const spec of specifiers) {
      if (spec.startsWith('./') || spec.startsWith('../')) {
        const target = resolve(dirname(file), spec);
        if (!roots.some((r) => target.startsWith(r))) {
          violations.push({ file, rule: 'import-hors-liste-blanche', detail: `${spec} sort de src/ et bin/` });
        }
      } else if (Object.hasOwn(whitelist, spec)) {
        used.add(spec);
      } else {
        violations.push({ file, rule: 'import-hors-liste-blanche', detail: spec });
      }
    }
  }
  return { violations, used };
}

const realDirs = () => [join(root, 'src'), join(root, 'bin')];

// ---------------------------------------------------------------------------------------------
// Contre-essais (CA-G1, CA-G2) : un fichier temoin par forme interdite, pose sous un src/ temporaire.
// ---------------------------------------------------------------------------------------------
const WITNESSES = [
  ['import-dynamique', "const m = await import('node:fs');"],
  ['import-dynamique', "const m = await import ('node:child_pro' + 'cess');"],
  ['require', "const cp = require('node:child_process');"],
  ['require', 'const r = module.require;'],
  ['eval', "eval('1+1');"],
  ['new-function', "const f = new Function('return 1');"],
  ['new-function', "const f = Function('return 1');"],
  ['constructor-acces', "const f = (() => 1).constructor('return process')();"],
  ['process-binding', "const b = process.binding('spawn_sync');"],
  ['process-binding', "const b = process._linkedBinding('x');"],
  ['process-binding', "process.dlopen(module, '/tmp/x.node');"],
  ['process-binding', "const cp = process.getBuiltinModule('node:child_process');"],
  ['acces-calcule', "const b = process['bin' + 'ding']('spawn_sync');"],
  ['acces-calcule', "const p = globalThis['proc' + 'ess'];"],
  ['process-non-liste', 'const p = process; p.abort();'],
  ['process-non-liste', 'const { binding } = process;'],
  ['worker_threads', "import { Worker } from 'node:worker_threads';"],
  ['worker_threads', "const w = new Worker('./x.js');"],
  ['cluster', "import cluster from 'node:cluster';"],
  ['vm', "import vm from 'node:vm';"],
  ['module', "import { createRequire } from 'node:module';"],
  ['child_process', "import { spawn } from 'node:child_process';"],
  ['child_process', "import { spawn } from 'child_process';"],
  ['child_process', "const n = 'child_pro' + 'cess';"],
  ['child_process', 'const n = "child" + "_" + "process";'],
  ['child_process', "const n = 'child_'.concat('process');"],
  ['child_process', "const n = '\\x63hild_process';"],
  ['child_process', "const n = '\\u0063hild_process';"],
  ['import-non-analysable', "import\n// commentaire\n  x from 'node:fs'\nimport ;"],
  ['inspector', "import inspector from 'node:inspector';"],
  ['repl', "import repl from 'node:repl';"],
  ['wasi', "import { WASI } from 'node:wasi';"],
  ['binaire-natif', null],
];

test('CA-G1 : chaque forme interdite, deposee dans un fichier temoin sous src/, rend la garde rouge', () => {
  const tmp = mkdtempSync(join(tmpdir(), 'gexec2-'));
  for (const [i, [rule, code]] of WITNESSES.entries()) {
    const dir = join(tmp, `case${i}`, 'src', 'agent');
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, code === null ? 'natif.node' : 'temoin.js'), code ?? '');
    const { violations } = scanTree([join(tmp, `case${i}`, 'src')]);
    const seen = violations.map((v) => v.rule);
    assert.ok(seen.includes(rule), `temoin #${i} (${rule}) : garde restee verte (regles vues : ${seen.join(',') || 'aucune'})`);
  }
});

test('cliquet : chaque regle de la garde a au moins un temoin', () => {
  const covered = new Set(WITNESSES.map(([rule]) => rule));
  const missing = RULE_IDS.filter((id) => id !== 'import-hors-liste-blanche' && !covered.has(id));
  assert.deepEqual(missing, []);
});

test('CA-G2 : un import hors liste blanche rend la garde rouge', () => {
  const tmp = mkdtempSync(join(tmpdir(), 'gexec2-'));
  const cases = ["import net from 'node:net';", "import fs from 'fs';", "import x from 'left-pad';", "import { a } from '../../evil.js';"];
  for (const [i, code] of cases.entries()) {
    const dir = join(tmp, `wl${i}`, 'src');
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, 'temoin.js'), code);
    const { violations } = scanTree([dir]);
    assert.ok(
      violations.some((v) => v.rule === 'import-hors-liste-blanche'),
      `import hors liste blanche non detecte : ${code}`,
    );
  }
});

test('temoin propre : imports de la liste blanche, chemins relatifs internes et `import.meta.url` restent verts', () => {
  const tmp = mkdtempSync(join(tmpdir(), 'gexec2-'));
  const dir = join(tmp, 'src');
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'b.js'), 'export const b = 1;\n');
  writeFileSync(
    join(dir, 'a.js'),
    [
      "import { readFileSync } from 'node:fs';",
      "import mqtt from 'mqtt';",
      "import { b } from './b.js';",
      "export { b } from './b.js';",
      "const here = new URL('./x', import.meta.url);",
      "process.stderr.write('ok ' + 'fin');",
      'const isAlive = process.kill.bind(process);',
    ].join('\n'),
  );
  assert.deepEqual(scanTree([dir]).violations, []);
});

// ---------------------------------------------------------------------------------------------
// Le code reel
// ---------------------------------------------------------------------------------------------
test('G-EXEC : src/ et bin/ ne comportent aucune forme interdite ni import hors liste blanche', () => {
  const { violations } = scanTree(realDirs());
  assert.deepEqual(violations, []);
});

test('registre : chaque module de la liste blanche est reellement importe et motive (pas de permission morte)', () => {
  const { used } = scanTree(realDirs());
  assert.deepEqual(Object.keys(WHITELIST).filter((m) => !used.has(m)), []);
  for (const [m, motif] of [...Object.entries(WHITELIST), ...Object.entries(PROCESS_MEMBERS)]) {
    assert.ok(motif.length >= 10, `motif manquant pour ${m}`);
  }
});

test('G-EXEC : la verification de certificat TLS n\'est jamais debrayee (D10)', () => {
  const files = realDirs().flatMap(walk).filter((f) => /\.m?js$/.test(f));
  const offenders = files.filter((f) => /rejectUnauthorized/.test(readFileSync(f, 'utf8')));
  assert.deepEqual(offenders, []);
});
