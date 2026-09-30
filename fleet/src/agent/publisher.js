// Publication (D4/D6) : retained QoS 1, differentielle, testament, purge.
// Le client MQTT est INJECTE (MQTT.js en production, faux client en test).
import {
  encode, metaTopic, sessionTopic, parseTopic, ContractError,
} from '../shared/contract.js';

export const KEEPALIVE_S = 30;
const PUB = { qos: 1, retain: true };

/** Options de connexion : le testament dit "hors ligne" quand personne ne l'annonce (F20, R3). */
export function buildConnectOptions({ host, username, password, nowS }) {
  return {
    clientId: `iakafleet-agent-${host}`,
    username,
    password,
    protocolVersion: 4,
    clean: true,
    keepalive: KEEPALIVE_S,
    reconnectPeriod: 5000,
    connectTimeout: 15000,
    will: {
      topic: metaTopic(host, 'online'),
      payload: encode(false, nowS),
      qos: 1,
      retain: true,
    },
  };
}

export class Publisher {
  constructor({ client, host, nowS }) {
    this.client = client;
    this.host = host;
    this.nowS = nowS;
    this.memory = new Map(); // topic -> valeur (JSON) derniere publiee
  }

  /** Reconnexion : la memoire differentielle est remise a zero => republication integrale. */
  resetMemory() {
    this.memory.clear();
  }

  #put(topic, v) {
    const key = JSON.stringify(v);
    if (this.memory.get(topic) === key) return false;
    this.client.publish(topic, encode(v, this.nowS()), PUB);
    this.memory.set(topic, key);
    return true;
  }

  #clear(topic) {
    this.client.publish(topic, '', PUB); // payload vide 0 octet, retained : efface le code
    this.memory.delete(topic);
  }

  /**
   * meta : { code: valeur } ; sessions : [{ pid, codes: { code: valeur } }].
   * Ne republie que ce qui change ; efface tous les codes des sessions disparues.
   */
  sync({ meta, sessions }) {
    for (const [code, v] of Object.entries(meta)) this.#put(metaTopic(this.host, code), v);
    const livePids = new Set();
    for (const s of sessions) {
      livePids.add(String(s.pid));
      for (const [code, v] of Object.entries(s.codes)) this.#put(sessionTopic(this.host, s.pid, code), v);
    }
    const prefix = `iakafleet/${this.host}/sessions/`;
    for (const topic of [...this.memory.keys()]) {
      if (!topic.startsWith(prefix)) continue;
      const pid = topic.slice(prefix.length).split('/')[0];
      if (!livePids.has(pid)) this.#clear(topic);
    }
  }

  /** Purge au demarrage : efface les retained d'une execution precedente dont le pid n'est pas vivant. */
  purgeRetained(topics, isAlive) {
    for (const topic of topics) {
      const p = parseTopic(topic);
      if (!p || p.kind !== 'session' || p.host !== this.host) continue;
      if (!isAlive(p.pid)) this.#clear(topic);
    }
  }

  /** Arret propre : online=false PUIS deconnexion (une deconnexion propre n'emet pas le testament). */
  goOffline() {
    return new Promise((resolve) => {
      this.client.publish(metaTopic(this.host, 'online'), encode(false, this.nowS()), PUB, () => resolve());
    });
  }
}

export { ContractError };
