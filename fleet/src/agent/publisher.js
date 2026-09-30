// Publication (contrat v2) : retained QoS 1, differentielle, testament, purge.
// Le client MQTT est INJECTE (MQTT.js en production, faux client en test).
import {
  encode, presenceTopic, sessionTopic, agentTopic, parseTopic, ContractError,
} from '../shared/contract.js';

export const KEEPALIVE_S = 30;
const PUB = { qos: 1, retain: true };

/** Options de connexion : le testament dit "hors ligne" quand personne ne l'annonce (F20, R3). */
export function buildConnectOptions({ node, username, password, nowS }) {
  return {
    clientId: `iakafleet-agent-${node}`,
    username,
    password,
    protocolVersion: 4,
    clean: true,
    keepalive: KEEPALIVE_S,
    reconnectPeriod: 5000,
    connectTimeout: 15000,
    will: {
      topic: presenceTopic(node, 'online'),
      payload: encode(false, nowS),
      qos: 1,
      retain: true,
    },
  };
}

export class Publisher {
  constructor({ client, node, nowS }) {
    this.client = client;
    this.node = node;
    this.nowS = nowS;
    this.memory = new Map(); // topic -> valeur (JSON) ou charge utile de fiche, derniere publiee
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

  #putRaw(topic, payload) {
    if (this.memory.get(topic) === payload) return false;
    this.client.publish(topic, payload, PUB);
    this.memory.set(topic, payload);
    return true;
  }

  #clear(topic) {
    this.client.publish(topic, '', PUB); // payload vide 0 octet, retained : efface le code / la fiche
    this.memory.delete(topic);
  }

  /**
   * presence : { code: valeur } ; sessions : [{ pid, codes: { code: valeur } }] ;
   * cards : [{ topicParts: {runner, sid, agentId}, payload }] (fiches deja filtrees).
   * Ne republie que ce qui change ; efface tous les codes des sessions disparues et les fiches
   * disparues du disque (G-PURGE).
   */
  sync({ presence, sessions, cards = [] }) {
    for (const [code, v] of Object.entries(presence)) this.#put(presenceTopic(this.node, code), v);
    const livePids = new Set();
    for (const s of sessions) {
      livePids.add(String(s.pid));
      for (const [code, v] of Object.entries(s.codes)) this.#put(sessionTopic(this.node, s.pid, code), v);
    }
    const liveCards = new Set();
    for (const c of cards) {
      const { runner, sid, agentId } = c.topicParts;
      const topic = agentTopic(this.node, runner, sid, agentId);
      liveCards.add(topic);
      this.#putRaw(topic, c.payload);
    }
    for (const topic of [...this.memory.keys()]) {
      const p = parseTopic(topic);
      if (!p) continue;
      if (p.kind === 'session' && !livePids.has(String(p.pid))) this.#clear(topic);
      else if (p.kind === 'agent' && !liveCards.has(topic)) this.#clear(topic);
    }
  }

  /**
   * Purge au demarrage : efface les retained d'une execution precedente — sessions dont le pid
   * n'est pas vivant, fiches qui ne sont plus sur le disque (liveCardTopics).
   */
  purgeRetained(topics, isAlive, liveCardTopics = new Set()) {
    for (const topic of topics) {
      const p = parseTopic(topic);
      if (!p || p.node !== this.node) continue;
      if (p.kind === 'session' && !isAlive(p.pid)) this.#clear(topic);
      else if (p.kind === 'agent' && !liveCardTopics.has(topic)) this.#clear(topic);
    }
  }

  /** Arret propre : online=false PUIS deconnexion (une deconnexion propre n'emet pas le testament). */
  goOffline() {
    return new Promise((resolve) => {
      this.client.publish(presenceTopic(this.node, 'online'), encode(false, this.nowS()), PUB, () => resolve());
    });
  }
}

export { ContractError };
