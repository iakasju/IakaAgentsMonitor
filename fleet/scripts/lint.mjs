// Lint sans dependance : syntaxe (node --check) + regles du projet. Hors src/ : child_process autorise ici.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const walk = (d) =>
  readdirSync(d).flatMap((n) => {
    if (n === 'node_modules' || n === 'fixtures') return [];
    const p = join(d, n);
    return statSync(p).isDirectory() ? walk(p) : /\.m?js$/.test(n) ? [p] : [];
  });

const files = ['src', 'bin', 'test', 'scripts'].flatMap((d) => walk(join(root, d)));
const problems = [];
for (const f of files) {
  const r = spawnSync(process.execPath, ['--check', f], { encoding: 'utf8' });
  if (r.status !== 0) problems.push(`${f} : erreur de syntaxe\n${r.stderr}`);
  const text = readFileSync(f, 'utf8');
  if (/\t/.test(text)) problems.push(`${f} : tabulation`);
  if (/\r\n/.test(text)) problems.push(`${f} : fins de ligne CRLF`);
  if (f.includes(`${join(root, 'src')}`) && /\bconsole\.(log|error)\b/.test(text)) {
    problems.push(`${f} : console.* interdit dans src/ (utiliser les flux injectes)`);
  }
  if (/(?:PASS|TOKEN|SECRET)\w*\s*[:=]\s*['"][^'"\s]{6,}['"]/.test(text) && !f.includes('test')) {
    problems.push(`${f} : secret en dur ?`);
  }
}
if (problems.length) {
  process.stderr.write(`${problems.join('\n')}\n`);
  process.stderr.write(`lint : ${problems.length} probleme(s) sur ${files.length} fichiers\n`);
  process.exit(1);
}
process.stdout.write(`lint : ${files.length} fichiers, 0 probleme\n`);
