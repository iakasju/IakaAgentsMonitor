// D8 : version d'@naonedge/iakaframe par lecture de package.json, jamais par execution (F21).
import { existsSync, readFileSync, realpathSync } from 'node:fs';
import path from 'node:path';

const PKG_NAME = '@naonedge/iakaframe';

function versionFrom(file, readFile) {
  try {
    const j = JSON.parse(readFile(file));
    return j?.name === PKG_NAME && typeof j.version === 'string' ? j.version : null;
  } catch {
    return null;
  }
}

/**
 * deps : { env, platform, exists, realpath, readFile }
 * Retourne la version (string) ou null (introuvable = inconnu, jamais invente).
 */
export function findIakaframeVersion(deps = {}) {
  const env = deps.env ?? process.env;
  const platform = deps.platform ?? process.platform;
  const exists = deps.exists ?? existsSync;
  const realpath = deps.realpath ?? realpathSync;
  const readFile = deps.readFile ?? ((p) => readFileSync(p, 'utf8'));
  const P = platform === 'win32' ? path.win32 : path.posix;

  if (env.IAKAFLEET_IAKAFRAME_PKG) return versionFrom(env.IAKAFLEET_IAKAFRAME_PKG, readFile);

  const pathVar = env.PATH ?? env.Path ?? '';
  for (const dir of pathVar.split(P.delimiter).filter(Boolean)) {
    if (platform === 'win32') {
      if (!exists(P.join(dir, 'iakaframe.cmd'))) continue;
      const v = versionFrom(P.join(dir, 'node_modules', '@naonedge', 'iakaframe', 'package.json'), readFile);
      if (v) return v;
      continue;
    }
    const bin = P.join(dir, 'iakaframe');
    if (!exists(bin)) continue;
    let real;
    try {
      real = realpath(bin);
    } catch {
      continue;
    }
    for (let d = P.dirname(real); ; d = P.dirname(d)) {
      const v = versionFrom(P.join(d, 'package.json'), readFile);
      if (v) return v;
      if (P.dirname(d) === d) break;
    }
  }
  return null;
}
