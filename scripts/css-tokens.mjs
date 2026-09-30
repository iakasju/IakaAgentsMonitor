// Lecture des jetons de couleur de src/styles.css (partagé par test-contrast.mjs et test-fontsize.mjs).
// Script Node SANS dépendance. Trois blocs : clair (`:root`), sombre suivi de macOS
// (`@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { … } }`) et sombre forcé
// (`:root[data-theme="dark"] { … }`).
import { readFileSync } from 'node:fs';

const HEX = /^#[0-9a-fA-F]{6}$/;

/** Corps du premier bloc `{ … }` qui suit `selector` (accolades équilibrées). */
export function blockAfter(css, selector, from = 0) {
  const at = css.indexOf(selector, from);
  if (at < 0) throw new Error(`bloc introuvable : ${selector}`);
  const open = css.indexOf('{', at);
  let depth = 0;
  for (let i = open; i < css.length; i++) {
    if (css[i] === '{') depth++;
    else if (css[i] === '}' && --depth === 0) return { body: css.slice(open + 1, i), end: i };
  }
  throw new Error(`accolade non fermée : ${selector}`);
}

/** Déclarations `--jeton: #rrggbb;` d'un corps de bloc (les autres jetons — polices, tailles — sont ignorés). */
export function colorTokens(body) {
  const out = {};
  for (const m of body.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)) {
    const v = m[2].trim();
    if (HEX.test(v)) out[m[1]] = v.toLowerCase();
  }
  return out;
}

/** { light, darkMedia, darkForced } : jetons de couleur de chaque thème (le sombre hérite du clair). */
export function readThemes(path) {
  const css = readFileSync(path, 'utf8');
  const light = colorTokens(blockAfter(css, ':root {').body);
  const media = blockAfter(css, '@media (prefers-color-scheme: dark)');
  const darkMedia = { ...light, ...colorTokens(blockAfter(media.body, ':root:not([data-theme="light"])').body) };
  const darkForced = { ...light, ...colorTokens(blockAfter(css, ':root[data-theme="dark"]').body) };
  return { css, light, darkMedia, darkForced };
}

// ---- WCAG 2.x ----
export const toRgb = (hex) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
const lin = (c) => {
  c /= 255;
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
};
export const luminance = (hex) => {
  const [r, g, b] = toRgb(hex);
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
};
export function ratio(a, b) {
  const x = luminance(a);
  const y = luminance(b);
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}
/** Teinte HSL (degrés, 0-360). */
export function hue(hex) {
  const [r, g, b] = toRgb(hex).map((v) => v / 255);
  const mx = Math.max(r, g, b);
  const mn = Math.min(r, g, b);
  const d = mx - mn;
  if (d === 0) return 0;
  let h = mx === r ? ((g - b) / d) % 6 : mx === g ? (b - r) / d + 2 : (r - g) / d + 4;
  h *= 60;
  return (h + 360) % 360;
}
export const hueDistance = (a, b) => {
  const d = Math.abs(hue(a) - hue(b));
  return Math.min(d, 360 - d);
};
