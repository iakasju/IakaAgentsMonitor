#!/usr/bin/env node
// Garde de lisibilité des tailles (ce-poste-vue-locale-lisibilite.md D6.1-D6.3, CA-8) : AUCUNE valeur
// `font-size` en px hors du bloc de jetons `--sc-fs-*` ; l'échelle respecte le plancher (corps >= 14 px,
// secondaire >= 13 px, rien sous 12 px) et les trois crans 14 / 16 / 18 px, 16 px par défaut.
//   npm run test:fontsize [-- --css autre.css]
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const cssPath = args.includes('--css') ? args[args.indexOf('--css') + 1] : join(here, '..', 'src', 'styles.css');
const css = readFileSync(cssPath, 'utf8');
const failures = [];

// (1) font-size en dur : toute déclaration doit passer par l'échelle de jetons.
css.split('\n').forEach((line, i) => {
  const m = /(?<![-\w])font-size\s*:\s*([^;}]+)/.exec(line);
  if (!m) return;
  const v = m[1].trim();
  if (/\d(\.\d+)?\s*(px|pt|rem|em)\b/.test(v.replace(/var\([^)]*\)/g, ''))) {
    failures.push(`styles.css:${i + 1} font-size en dur « ${v} » : utiliser var(--sc-fs-…) (hors bloc de jetons)`);
  } else if (!/var\(--sc-fs-[a-z]+\)/.test(v)) {
    failures.push(`styles.css:${i + 1} font-size « ${v} » ne passe pas par l'échelle --sc-fs-*`);
  }
});

// (2) Échelle : valeurs par cran.
function tokensIn(selector) {
  const at = css.indexOf(selector);
  if (at < 0) return null;
  const open = css.indexOf('{', at);
  const close = css.indexOf('}', open);
  const out = {};
  for (const m of css.slice(open + 1, close).matchAll(/--sc-fs-([a-z]+)\s*:\s*([0-9.]+)px/g)) out[m[1]] = Number(m[2]);
  return out;
}
const scale = {
  s: tokensIn(':root[data-size="s"]'),
  m: tokensIn(':root[data-size="m"]'),
  l: tokensIn(':root[data-size="l"]'),
};
const expectedBody = { s: 14, m: 16, l: 18 };
for (const k of ['s', 'm', 'l']) {
  const t = scale[k];
  if (!t) {
    failures.push(`cran ${k} absent : :root[data-size="${k}"] avec --sc-fs-* attendu`);
    continue;
  }
  if (t.body !== expectedBody[k]) failures.push(`cran ${k} : --sc-fs-body = ${t.body} px, attendu ${expectedBody[k]} px`);
  for (const [name, px] of Object.entries(t)) {
    if (px < 12) failures.push(`cran ${k} : --sc-fs-${name} = ${px} px < 12 px (rien sous 12 px)`);
    if (['body'].includes(name) && px < 14) failures.push(`cran ${k} : texte courant ${px} px < 14 px`);
    if (['sec', 'chip'].includes(name) && px < 13) failures.push(`cran ${k} : texte secondaire --sc-fs-${name} = ${px} px < 13 px`);
  }
}
// (3) Le cran par défaut (sans attribut data-size) est le 16 px.
const rootAt = css.indexOf(':root {');
const rootBlockEnd = css.indexOf('\n}', rootAt);
const defaultBody = /--sc-fs-body\s*:\s*([0-9.]+)px/.exec(css.slice(rootAt, rootBlockEnd));
if (!defaultBody || Number(defaultBody[1]) !== 16) failures.push(`cran par défaut : --sc-fs-body dans :root = ${defaultBody ? defaultBody[1] : 'absent'} px, attendu 16 px`);

// (4) Tout var(--sc-fs-x) employé est défini.
const defined = new Set(Object.keys(scale.m ?? {}));
for (const m of css.matchAll(/var\(--sc-fs-([a-z]+)\)/g)) if (!defined.has(m[1])) failures.push(`--sc-fs-${m[1]} employé mais non défini dans le cran m`);

const n = css.split('\n').filter((l) => /(?<![-\w])font-size\s*:/.test(l)).length;
console.log(`${n} déclaration(s) font-size ; crans : ${['s', 'm', 'l'].map((k) => `${k}=${scale[k]?.body ?? '?'}px`).join(' ')} ; ${failures.length} échec(s)`);
if (failures.length) {
  console.error(failures.map((f) => `  ✖ ${f}`).join('\n'));
  process.exit(1);
}
