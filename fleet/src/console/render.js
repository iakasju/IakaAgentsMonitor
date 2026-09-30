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
  if (m.state === 'offline' && m.since != null) {
    const d = formatDuration(nowS - m.since);
    // Le « depuis » d'une machine hors ligne n'est jamais affirme : dernier signe de vie, ou heure d'arret inconnue.
    return m.basis === 'last_seen'
      ? `${base} (dernier signe de vie il y a ${d})`
      : `${base} (heure d'arrêt inconnue ; connecté il y a ${d})`;
  }
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

export const TASK_MASKED = 'tache masquee';

/** Ligne d'un agent (D5) : persona, tache (masquee cote machine => dit, jamais une cellule vide), duree. */
export function agentLine(a, nowS, indent) {
  const who = `${a.persona ?? a.agentId ?? '?'}`;
  const task = a.task === null ? TASK_MASKED : a.task;
  const since = a.startedAt != null ? formatDuration(nowS - a.startedAt) : '-';
  const st = a.live ? dash(a.state) : `? (dernier connu : ${dash(a.state)})`;
  return `${indent}- ${who.padEnd(10)} ${st.padEnd(8)} ${task}  (${since}${a.model ? `, ${a.model}` : ''})`;
}

export function renderFleet(fleet, nowS) {
  if (fleet.machines.length === 0) return "aucune machine n'a publie sur iaka/.\n";
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
      for (const a of s.agents ?? []) lines.push(agentLine(a, nowS, '           '));
    }
    if ((m.unattachedAgents ?? []).length > 0) {
      lines.push('  agents non rattaches :');
      for (const a of m.unattachedAgents) lines.push(agentLine(a, nowS, '    '));
    }
    lines.push('');
  }
  return `${lines.join('\n')}`;
}
