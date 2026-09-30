// G-EXEC-2, couche 2 (instruction fusion § 13.1, CA-G3) : le demon, lance par les ARGUMENTS EXACTS du gabarit
// de service (deploy/launchd), tourne sous `node --permission` : il demarre, lit ses sources et publie contre
// un FAUX broker local (jamais mqtt.naonedge.com), et un child_process / Worker injecte dans SON processus est
// REFUSE (ERR_ACCESS_DENIED). Non lance sous Windows (gabarit .ps1 non execute ici).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import net from 'node:net';
import { spawn } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, cpSync, realpathSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const [major, minor] = process.versions.node.split('.').map(Number);
const permissionOk = major > 22 || (major === 22 && minor >= 14);
const skip = !permissionOk
  ? `Node ${process.versions.node} < 22.14 : modele de permissions non stable`
  : process.platform === 'win32'
    ? 'gabarit launchd : non execute sous Windows (NON MESURE)'
    : false;

const FORBIDDEN_FLAGS = ['--allow-child-process', '--allow-worker', '--allow-fs-write', '--allow-addons', '--allow-wasi'];
const templates = {
  launchd: readFileSync(join(root, 'deploy/launchd/com.naonedge.iakafleet-agent.plist'), 'utf8'),
  systemd: readFileSync(join(root, 'deploy/systemd/iakafleet-agent.service'), 'utf8'),
  windows: readFileSync(join(root, 'deploy/windows/Install-IakaFleetAgent.ps1'), 'utf8'),
};

// Contrat statique des trois gabarits (l'execution reelle ne porte que sur launchd, sur ce poste).
const withoutComments = (text) =>
  text.replace(/<!--[\s\S]*?-->/g, '').replace(/<#[\s\S]*?#>/g, '').split('\n').filter((l) => !/^\s*#/.test(l)).join('\n');

test('gabarits de service : node --permission, lectures seules, aucune permission de lancement ni d\'ecriture', () => {
  for (const [name, text] of Object.entries(templates)) {
    const code = withoutComments(text);
    assert.match(code, /--permission/, `${name} : --permission absent`);
    assert.match(code, /--allow-fs-read/, `${name} : aucune lecture accordee`);
    for (const f of FORBIDDEN_FLAGS) assert.ok(!code.includes(f), `${name} : ${f} accorde`);
  }
  assert.match(templates.windows, /22\.14/, 'windows : verification de la version de Node');
});

function fakeBroker() {
  const ev = { connects: 0, subscribes: 0, publishes: [] };
  const rl = (buf, o) => {
    let mul = 1, val = 0, i = o, b;
    do {
      if (i >= buf.length) return null;
      b = buf[i++];
      val += (b & 0x7f) * mul;
      mul *= 128;
    } while (b & 0x80);
    return { val, next: i };
  };
  const server = net.createServer((sock) => {
    ev.connects++;
    let buf = Buffer.alloc(0);
    sock.on('error', () => {});
    sock.on('data', (c) => {
      buf = Buffer.concat([buf, c]);
      for (;;) {
        if (buf.length < 2) break;
        const r = rl(buf, 1);
        if (!r || buf.length < r.next + r.val) break;
        const pkt = buf.subarray(0, r.next + r.val);
        buf = buf.subarray(r.next + r.val);
        const type = pkt[0] & 0xf0;
        if (type === 0x10) sock.write(Buffer.from([0x20, 0x02, 0x00, 0x00]));
        else if (type === 0x80) {
          ev.subscribes++;
          const id = pkt.subarray(r.next, r.next + 2);
          // SUBACK : un QoS accorde (1) par filtre demande ; on compte les filtres.
          let o = r.next + 2, n = 0;
          while (o < pkt.length) { o += 2 + pkt.readUInt16BE(o) + 1; n++; }
          sock.write(Buffer.concat([Buffer.from([0x90, 2 + n]), id, Buffer.alloc(n, 1)]));
        } else if (type === 0xa0) sock.write(Buffer.concat([Buffer.from([0xb0, 0x02]), pkt.subarray(r.next, r.next + 2)]));
        else if (type === 0xc0) sock.write(Buffer.from([0xd0, 0x00]));
        else if (type === 0x30) {
          let o = r.next;
          const tl = pkt.readUInt16BE(o); o += 2;
          const topic = pkt.toString('utf8', o, o + tl); o += tl;
          const qos = (pkt[0] & 0x06) >> 1;
          let id = null;
          if (qos > 0) { id = pkt.subarray(o, o + 2); o += 2; }
          ev.publishes.push({ topic, payload: pkt.toString('utf8', o), retain: (pkt[0] & 1) === 1 });
          if (qos === 1) sock.write(Buffer.concat([Buffer.from([0x40, 0x02]), id]));
        }
      }
    });
  });
  return new Promise((res) => server.listen(0, '127.0.0.1', () => res({ ev, port: server.address().port, close: () => new Promise((r) => { server.closeAllConnections?.(); server.close(r); }) })));
}

/** Arguments du demon = ProgramArguments du plist, placeholders substitues (le gabarit est execute tel quel). */
function plistArgs(map) {
  const block = /<key>ProgramArguments<\/key>\s*<array>([\s\S]*?)<\/array>/.exec(templates.launchd)[1];
  return [...block.matchAll(/<string>([^<]*)<\/string>/g)].map((m) =>
    Object.entries(map).reduce((s, [k, v]) => s.replaceAll(k, v), m[1]),
  );
}

async function scenario() {
  const home = realpathSync(mkdtempSync(join(tmpdir(), 'iakafleet-perm-')));
  const projects = join(home, 'work');
  const repo = join(projects, 'alpha');
  mkdirSync(join(repo, '.git'), { recursive: true });
  writeFileSync(join(repo, '.git', 'HEAD'), 'ref: refs/heads/feat/perm\n');
  mkdirSync(join(home, '.claude', 'sessions'), { recursive: true });
  mkdirSync(join(home, '.iakafleet'), { recursive: true });
  const sid = 'perm-session-1';
  writeFileSync(
    join(home, '.claude', 'sessions', `${process.pid}.json`),
    JSON.stringify({ pid: process.pid, sessionId: sid, cwd: repo, startedAt: 1780000000000, version: '2.1.285', kind: 'interactive', entrypoint: 'cli', name: 'alpha', status: 'busy', statusUpdatedAt: 1780000100000, updatedAt: 1780000100000 }),
  );
  cpSync(join(root, 'test/fixtures/cards/sess-brut-B'), join(home, '.cache', 'iaka-agents', 'sess-brut-B'), { recursive: true });
  const pkg = join(home, 'iakaframe-pkg', 'package.json');
  mkdirSync(dirname(pkg), { recursive: true });
  writeFileSync(pkg, JSON.stringify({ name: '@naonedge/iakaframe', version: '9.9.9' }));
  const probe = join(home, 'probe-spawn.mjs');
  writeFileSync(
    probe,
    `import { createRequire } from 'node:module';
import { spawn, spawnSync, execSync } from 'node:child_process';
import { Worker } from 'node:worker_threads';
const attempts = {
  spawn: () => spawn('/bin/echo', ['x']),
  spawnSync: () => spawnSync('/bin/echo', ['x']),
  execSync: () => execSync('echo x'),
  worker: () => new Worker('1', { eval: true }),
  requireExec: () => createRequire(import.meta.url)('node:child_process').execFileSync('/bin/echo', ['x']),
};
const out = {};
for (const [k, fn] of Object.entries(attempts)) {
  try { const r = fn(); out[k] = r && r.error ? r.error.code : 'AUTORISE'; } catch (e) { out[k] = e.code ?? 'ERR:' + e.message; }
}
process.stderr.write('INJECT ' + JSON.stringify(out) + '\\n');
`,
  );
  return { home, projects, repo, pkg, probe, sid };
}

async function runDaemon({ inject, drop = null }) {
  const s = await scenario();
  const broker = await fakeBroker();
  const args = plistArgs({
    __NODE__: process.execPath, __REPO__: root, __HOME__: s.home,
    __PROJECTS_ROOT__: s.projects, __IAKAFRAME_PKG__: s.pkg, __NODE_DIR__: dirname(process.execPath),
  });
  const [node, ...rest0] = args;
  const rest = rest0.filter((a) => !(drop && a.endsWith(drop))); // mutation : retire UNE lecture accordee
  // Precharge d'injection : lecture accordee a ce SEUL fichier de test, en plus du permis du gabarit.
  const flags = inject ? ['--allow-fs-read=' + s.probe, '--import', s.probe] : [];
  const child = spawn(node, [...rest.slice(0, -1), ...flags, rest.at(-1)], {
    env: {
      PATH: process.env.PATH, HOME: s.home, USERPROFILE: s.home,
      IAKA_NODE: 'permtest', IAKAFLEET_MQTT_URL: `mqtt://127.0.0.1:${broker.port}`,
      IAKAFLEET_AGENT_USER: 'fleet-permtest', IAKAFLEET_AGENT_PASS: 'not-a-secret-test-only',
      IAKAFLEET_SCAN_S: '1', IAKAFLEET_IAKAFRAME_PKG: s.pkg,
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let stderr = '';
  child.stderr.on('data', (d) => (stderr += d));
  const exited = new Promise((r) => child.on('exit', (code, sig) => r({ code, sig })));
  const deadline = Date.now() + 8000;
  const published = (re) => broker.ev.publishes.some((p) => re.test(p.topic));
  // Un balayage est termine quand la presence (dont cards_source_ok) est publiee ; on laisse 300 ms de vidange.
  while (Date.now() < deadline && child.exitCode === null && !published(/presence\/cards_source_ok\/current$/)) {
    await new Promise((r) => setTimeout(r, 100));
  }
  await new Promise((r) => setTimeout(r, 300));
  const alive = child.exitCode === null;
  child.kill('SIGTERM');
  const exit = await Promise.race([exited, new Promise((r) => setTimeout(() => r({ code: 'timeout' }), 5000))]);
  await broker.close();
  return { s, ev: broker.ev, stderr, alive, exit };
}

test('CA-G3 : le demon demarre et publie sous --permission (gabarit launchd), lit sessions, fiches, .git, package.json iakaframe', { skip }, async () => {
  const r = await runDaemon({ inject: false });
  assert.ok(r.alive, `le demon s'est arrete sous --permission :\n${r.stderr}`);
  assert.doesNotMatch(r.stderr, /ERR_ACCESS_DENIED/, r.stderr);
  const val = (topic) => r.ev.publishes.find((p) => p.topic === topic)?.payload;
  assert.match(val('iaka/permtest/out/presence/online/current') ?? '', /true/);
  assert.match(val('iaka/permtest/out/presence/iakaframe_version/current') ?? '', /9\.9\.9/);
  assert.match(val('iaka/permtest/out/presence/sessions_dir_ok/current') ?? '', /true/);
  assert.match(val('iaka/permtest/out/presence/cards_source_ok/current') ?? '', /true/);
  const branch = r.ev.publishes.find((p) => /\/out\/sessions\/\d+\/branch\/current$/.test(p.topic));
  assert.match(branch?.payload ?? '', /feat\/perm/, 'branche lue depuis .git/HEAD');
  assert.ok(r.ev.publishes.some((p) => /\/out\/agents\/claude-code\//.test(p.topic)), 'fiche d\'agent relayee');
  assert.equal(r.exit.code, 0, `arret propre attendu (SIGTERM) : ${JSON.stringify(r.exit)}\n${r.stderr}`);
});

test('CA-G3 : un child_process / Worker injecte dans le demon est REFUSE (ERR_ACCESS_DENIED), le demon publie quand meme', { skip }, async () => {
  const r = await runDaemon({ inject: true });
  const m = /INJECT (\{.*\})/.exec(r.stderr);
  assert.ok(m, `sonde d'injection non executee :\n${r.stderr}`);
  const res = JSON.parse(m[1]);
  assert.deepEqual(Object.keys(res).sort(), ['execSync', 'requireExec', 'spawn', 'spawnSync', 'worker']);
  for (const [k, code] of Object.entries(res)) assert.equal(code, 'ERR_ACCESS_DENIED', `${k} : ${code}`);
  assert.ok(r.ev.publishes.some((p) => p.topic === 'iaka/permtest/out/presence/online/current'), 'le demon publie malgre la sonde');
});

test('contre-essai : sans --permission, la meme sonde REUSSIT a lancer un processus (le refus vient bien du permis)', { skip }, async () => {
  const s = await scenario();
  const child = spawn(process.execPath, [s.probe], { stdio: ['ignore', 'ignore', 'pipe'] });
  let stderr = '';
  child.stderr.on('data', (d) => (stderr += d));
  await new Promise((r) => child.on('exit', r));
  const res = JSON.parse(/INJECT (\{.*\})/.exec(stderr)[1]);
  assert.deepEqual(Object.values(res).filter((v) => v !== 'AUTORISE'), []);
});

// Chaque lecture accordee est NECESSAIRE : la retirer degrade visiblement (jamais une liste de permissions morte),
// et le demon reste en vie (degradation ecrite, jamais un plantage silencieux).
const val = (r, re) => r.ev.publishes.find((p) => re.test(p.topic))?.payload ?? '';
const MUTATIONS = [
  ['/.claude/sessions', (r) => assert.match(val(r, /presence\/sessions_dir_ok\/current$/), /false/)],
  ['/.cache/iaka-agents', (r) => assert.match(val(r, /presence\/cards_source_ok\/current$/), /false/)],
  ['/work', (r) => assert.match(val(r, /sessions\/\d+\/branch\/current$/), /"v":null/)],
  ['/iakaframe-pkg/package.json', (r) => assert.match(val(r, /presence\/iakaframe_version\/current$/), /"v":null/)],
];
for (const [drop, check] of MUTATIONS) {
  test(`mutation : sans --allow-fs-read=...${drop}, la lecture correspondante est refusee et visible`, { skip }, async () => {
    const r = await runDaemon({ inject: false, drop });
    assert.ok(r.alive, `le demon doit rester en vie :\n${r.stderr}`);
    check(r);
  });
}
