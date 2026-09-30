#!/usr/bin/env node
// Test de contraste WCAG 2.x de src/styles.css (ce-poste-vue-locale-lisibilite.md D6.4, D8, CA-9, CA-9b).
// Sans dépendance. Lit les jetons de couleur (clair, sombre suivi de macOS, sombre forcé) et un
// REGISTRE versionné des couples encre/fond réellement employés (scripts/contrast-registry.json).
//   npm run test:contrast                  # échoue (code 1) au moindre couple sous le seuil
//   node scripts/test-contrast.mjs --css f # contre-essai sur une autre feuille de style
// Vérifie : (1) chaque couple du registre >= 4,5:1 en clair ET en sombre ; (2) bandeaux de dépôt >= 5:1 ;
// (3) coloris conservés : seuls les jetons d8Tokens diffèrent de la base, teinte à ± 5° ; aucun nouveau
// jeton de couleur ; (4) registre non muet : tout jeton employé en `color:` figure au registre.
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { hueDistance, ratio, readThemes } from './css-tokens.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const cssPath = args.includes('--css') ? args[args.indexOf('--css') + 1] : join(here, '..', 'src', 'styles.css');
const registry = JSON.parse(readFileSync(join(here, 'contrast-registry.json'), 'utf8'));
const { css, light, darkMedia, darkForced } = readThemes(cssPath);
const themes = [
  ['clair', light, 'light'],
  ['sombre (suit macOS)', darkMedia, 'dark'],
  ['sombre (forcé)', darkForced, 'dark'],
];

const failures = [];
const rows = [];
const fail = (msg) => failures.push(msg);

for (const [label, tokens] of themes) {
  const check = (p, min) => {
    const fg = tokens[p.fg];
    const bg = tokens[p.bg];
    if (!fg || !bg) return fail(`[${label}] jeton manquant : ${!fg ? p.fg : p.bg}`);
    const r = ratio(fg, bg);
    rows.push({ label, fg: p.fg, bg: p.bg, fgHex: fg, bgHex: bg, r, min });
    if (r < min) fail(`[${label}] ${p.fg} ${fg} sur ${p.bg} ${bg} = ${r.toFixed(2)}:1 < ${min}:1 (${p.use})`);
  };
  for (const p of registry.pairs) check(p, registry.textMin);
  for (const p of registry.bands) check(p, registry.bandMin);
}

// (3) Coloris conservés (D8, CA-9b).
for (const mode of ['light', 'dark']) {
  const current = mode === 'light' ? light : darkMedia;
  const forced = mode === 'light' ? light : darkForced;
  const base = registry.baseline[mode];
  const allowed = new Set(registry.d8Tokens[mode]);
  for (const [name, v] of Object.entries(current)) {
    if (!(name in base)) {
      fail(`[${mode}] nouveau jeton de couleur ${name} ${v} : aucune nouvelle teinte n'est autorisée (D8)`);
      continue;
    }
    if (v !== base[name] && !allowed.has(name)) fail(`[${mode}] ${name} : ${base[name]} -> ${v}, jeton hors liste D8`);
    if (allowed.has(name) && v !== base[name]) {
      const d = hueDistance(base[name], v);
      if (d > registry.hueTolerance) fail(`[${mode}] ${name} : ${base[name]} -> ${v}, teinte déplacée de ${d.toFixed(1)}° (> ${registry.hueTolerance}°)`);
    }
    if (mode === 'dark' && forced[name] !== v) fail(`[sombre] ${name} diffère entre le bloc macOS (${v}) et le bloc forcé (${forced[name]})`);
  }
}

// (4) Registre non muet : tout jeton lu par `color:` doit figurer comme premier plan du registre.
const known = new Set([...registry.pairs, ...registry.bands].map((p) => p.fg));
const used = new Set();
for (const m of css.matchAll(/(?<![-\w])color\s*:\s*var\((--[a-z0-9-]+)\)/g)) used.add(m[1]);
for (const t of used) if (!known.has(t)) fail(`registre muet : ${t} est employé en color: mais absent du registre (scripts/contrast-registry.json)`);

// Sortie chiffrée (M3).
if (args.includes('--table') || failures.length > 0) {
  for (const x of rows.filter((r) => !args.includes('--only-fail') || r.r < r.min)) {
    console.log(`${x.r >= x.min ? 'ok  ' : 'FAIL'} ${x.r.toFixed(2).padStart(5)}:1  ${x.fg} ${x.fgHex} / ${x.bg} ${x.bgHex}  [${x.label}]`);
  }
}
const worst = rows.reduce((a, b) => (b.r < a.r ? b : a));
console.log(`${rows.length} couples x thèmes ; plus faible : ${worst.r.toFixed(2)}:1 (${worst.fg} sur ${worst.bg}, ${worst.label}) ; ${failures.length} échec(s)`);
if (failures.length > 0) {
  console.error(failures.map((f) => `  ✖ ${f}`).join('\n'));
  process.exit(1);
}
