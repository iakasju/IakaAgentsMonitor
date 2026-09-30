// Demon de machine (D6, contrat v2) : lit, en lecture seule, les fichiers de session ET les fiches
// d'agents ecrites par le publieur de hooks, puis publie sur MQTT sous iaka/<node>/out/.
// Il ne lance aucun processus, n'execute aucun ordre (jamais de shell), ne s'abonne qu'a son propre
// sous-arbre out/ (et seulement le temps de la purge de demarrage) : aucun ordre `in/` n'est lu.
import {
  presenceTopic, sessionsWildcard, agentsWildcard, agentTopic, parseTopic, sidOf,
} from '../shared/contract.js';
import { readCards as defaultReadCards, toCentral } from './cards.js';
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
    // Jamais le sessionId brut : <sid> = sha256 tronque, cle de rattachement des fiches d'agents.
    sid: s.sessionId ? sidOf(s.sessionId) : null,
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
  const readCards = deps.readCards ?? defaultReadCards;
  const findGit = deps.findGit ?? defaultFindGit;
  const findVersion = deps.findVersion ?? defaultFindVersion;
  const alive = deps.alive ?? isPidAlive;
  const timers = deps.timers ?? { setInterval, clearInterval, setTimeout, clearTimeout };
  const settleMs = deps.settleMs ?? 1000;

  const client = deps.connect(
    cfg.url,
    buildConnectOptions({ node: cfg.node, username: cfg.agentUser, password: cfg.agentPass, nowS: nowS() }),
  );
  const publisher = new Publisher({ client, node: cfg.node, nowS });

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
    const { dirOk: cardsOk, cards } = liveCards();
    publisher.sync({
      presence: {
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
        cards_source_ok: cardsOk, // false ECRIT, jamais sous-entendu (R2)
      },
      sessions: sessions.map((s) => ({ pid: s.pid, codes: sessionCodes(s, findGit(s.cwd), cfg) })),
      cards,
    });
  }

  /** Fiches du disque, filtrees pour le central (liste blanche, sid, cwd, task masquee). */
  function liveCards() {
    const { dirOk, cards } = readCards(cfg.cardsDir, { log });
    return { dirOk, cards: cards.map((c) => toCentral(c, cfg)).filter(Boolean) };
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
    client.subscribe([sessionsWildcard(cfg.node), agentsWildcard(cfg.node)], { qos: 1 }, (err) => {
      if (err) log(`abonnement de purge refuse : ${err.message}`);
      settleTimer = timers.setTimeout(() => {
        purging = false;
        const live = new Set(
          liveCards().cards.map((c) => agentTopic(cfg.node, c.topicParts.runner, c.topicParts.sid, c.topicParts.agentId)),
        );
        publisher.purgeRetained(retained, alive, live);
        client.unsubscribe?.([sessionsWildcard(cfg.node), agentsWildcard(cfg.node)]);
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
      // Ne jamais rester pendu a la sortie : si le broker ne repond pas, on force apres 2 s.
      await Promise.race([
        new Promise((r) => client.end(false, r)),
        new Promise((r) => timers.setTimeout(r, 2000)),
      ]);
    },
    onlineTopic: presenceTopic(cfg.node, 'online'),
  };
}
