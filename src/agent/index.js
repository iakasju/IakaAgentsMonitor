// Agent de flotte (D6) : lit, en lecture seule, les fichiers de session et publie sur MQTT.
// Il ne lance aucun processus, n'execute aucun ordre, ne s'abonne qu'a son propre sous-arbre
// de sessions (et seulement le temps de la purge de demarrage).
import { metaTopic, sessionsWildcard, parseTopic } from '../shared/contract.js';
import { Publisher, buildConnectOptions } from './publisher.js';
import { readSessions as defaultReadSessions, isPidAlive, relativeToHome } from './sessions.js';
import { findGit as defaultFindGit } from './gitHead.js';
import { findIakaframeVersion as defaultFindVersion } from './iakaframeVersion.js';

export function sessionCodes(s, git, cfg) {
  return {
    status: s.status,
    status_since: s.statusSince,
    stuck_suspect: s.stuckSuspect,
    name: cfg.maskNames ? null : s.name,
    kind: s.kind,
    entrypoint: s.entrypoint,
    claude_version: s.claudeVersion,
    cwd: relativeToHome(s.cwd, cfg.home),
    repo: git.repo,
    branch: git.branch,
    rc: s.rc,
    started_at: s.startedAt,
  };
}

/**
 * deps : { connect, nowS, readSessions, findGit, findVersion, alive, info, timers, log, settleMs }
 *   info = { platform, release, nodeVersion, agentVersion }
 */
export function createAgent(cfg, deps) {
  const nowS = deps.nowS ?? (() => Math.floor(Date.now() / 1000));
  const log = deps.log ?? (() => {});
  const readSessions = deps.readSessions ?? defaultReadSessions;
  const findGit = deps.findGit ?? defaultFindGit;
  const findVersion = deps.findVersion ?? defaultFindVersion;
  const alive = deps.alive ?? isPidAlive;
  const timers = deps.timers ?? { setInterval, clearInterval, setTimeout, clearTimeout };
  const settleMs = deps.settleMs ?? 1000;

  const client = deps.connect(
    cfg.url,
    buildConnectOptions({ host: cfg.host, username: cfg.agentUser, password: cfg.agentPass, nowS: nowS() }),
  );
  const publisher = new Publisher({ client, host: cfg.host, nowS });

  let scanTimer = null;
  let settleTimer = null;
  let purging = false;
  let retained = [];
  let lastTick = 0;
  let iakaframeVersion = null;
  let running = false;

  function scan() {
    const now = nowS();
    const heartbeatDue = lastTick === 0 || now - lastTick >= cfg.heartbeatS;
    if (heartbeatDue) {
      lastTick = now;
      iakaframeVersion = findVersion(); // relu a chaque battement (D8)
    }
    const { dirOk, sessions } = readSessions(cfg.sessionsDir, { log, alive });
    publisher.sync({
      meta: {
        online: true,
        os: deps.info.platform,
        os_release: deps.info.release,
        hostname: cfg.hostnameRaw,
        agent_version: deps.info.agentVersion,
        node_version: deps.info.nodeVersion,
        iakaframe_version: iakaframeVersion,
        sessions_dir_ok: dirOk,
        sessions_count: sessions.length,
        heartbeat_s: cfg.heartbeatS,
        last_tick_at: lastTick,
      },
      sessions: sessions.map((s) => ({ pid: s.pid, codes: sessionCodes(s, findGit(s.cwd), cfg) })),
    });
  }

  function stopTimers() {
    if (scanTimer) timers.clearInterval(scanTimer);
    if (settleTimer) timers.clearTimeout(settleTimer);
    scanTimer = null;
    settleTimer = null;
  }

  function onConnect() {
    stopTimers();
    publisher.resetMemory(); // republication integrale apres (re)connexion
    purging = true;
    retained = [];
    client.subscribe(sessionsWildcard(cfg.host), { qos: 1 }, (err) => {
      if (err) log(`abonnement de purge refuse : ${err.message}`);
      settleTimer = timers.setTimeout(() => {
        purging = false;
        publisher.purgeRetained(retained, alive);
        client.unsubscribe?.(sessionsWildcard(cfg.host));
        scan();
        scanTimer = timers.setInterval(scan, cfg.scanS * 1000);
      }, settleMs);
    });
  }

  client.on('connect', onConnect);
  client.on('close', stopTimers);
  client.on('error', (e) => log(`erreur MQTT : ${e.message}`));
  client.on('message', (topic) => {
    if (purging && parseTopic(topic)) retained.push(topic);
  });

  return {
    client,
    publisher,
    scan,
    /** Arret propre : online=false puis deconnexion. */
    async stop() {
      if (running === 'stopping') return;
      running = 'stopping';
      stopTimers();
      await Promise.race([
        publisher.goOffline(),
        new Promise((r) => timers.setTimeout(r, 2000)),
      ]);
      await new Promise((r) => client.end(false, r));
    },
    onlineTopic: metaTopic(cfg.host, 'online'),
  };
}
