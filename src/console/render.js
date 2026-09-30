// Rendu texte de la flotte. Aucune logique d'etat ici : tout vient de fleetModel.

export function formatDuration(sec) {
  const s = Math.max(0, Math.round(sec));
  if (s < 60) return `${s} s`;
  if (s < 3600) return `${Math.floor(s / 60)} min`;
  if (s < 86400) return `${Math.floor(s / 3600)} h`;
  return `${Math.floor(s / 86400)} j`;
}

const STATE_LABEL = {
  online: 'en ligne',
  mute: 'muette',
  offline: 'hors ligne',
  unknown: 'inconnue',
};

export function machineStateLabel(m, nowS) {
  const base = STATE_LABEL[m.state];
  if (m.state === 'offline' && m.since != null) return `${base} depuis ${formatDuration(nowS - m.since)}`;
  if (m.state === 'mute' && m.since != null) return `${base} (dernier battement il y a ${formatDuration(nowS - m.since)})`;
  return base;
}

/** Statut d'une session : "?" si la machine n'est pas en ligne, "busy ?" si suspecte. */
export function sessionStatusLabel(s, nowS) {
  const st = s.status ?? 'inconnu';
  if (!s.live) return `? (dernier connu : ${st}, il y a ${formatDuration(nowS - s.lastKnownT)})`;
  if (s.stuckSuspect && st === 'busy') return 'busy ?';
  return st;
}

const dash = (v) => (v === null || v === undefined ? '-' : String(v));

export function renderFleet(fleet, nowS) {
  if (fleet.machines.length === 0) return "aucune machine n'a publie sur iakafleet/.\n";
  const lines = [];
  for (const m of fleet.machines) {
    lines.push(`${m.host}  [${machineStateLabel(m, nowS)}]`);
    lines.push(
      `  os ${dash(m.os)} ${dash(m.osRelease)} | agent ${dash(m.agentVersion)} | node ${dash(m.nodeVersion)} | iakaframe ${dash(m.iakaframeVersion)}`,
    );
    for (const a of m.alerts) lines.push(`  ! ${a}`);
    if (m.sessions.length === 0) lines.push('  (aucune session publiee)');
    for (const s of m.sessions) {
      const where = s.repo ? `${s.repo}@${dash(s.branch)}` : '-';
      lines.push(
        `  ${String(s.pid).padEnd(7)} ${sessionStatusLabel(s, nowS).padEnd(11)} ${where}  ${dash(s.cwd)}  ` +
          `${s.rc ? 'rc' : '--'}  ${dash(s.name)}  v${dash(s.claudeVersion)} ${dash(s.entrypoint)}`,
      );
    }
    lines.push('');
  }
  return `${lines.join('\n')}`;
}
