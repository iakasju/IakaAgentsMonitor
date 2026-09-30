import { EventEmitter } from 'node:events';

/** Faux client MQTT (interface minimale de MQTT.js), enregistre tout ce qui est envoye. */
export class FakeClient extends EventEmitter {
  constructor() {
    super();
    this.published = []; // {topic, payload, opts}
    this.subscribed = [];
    this.calls = []; // ordre global : 'publish' | 'end'
    this.ended = false;
  }

  publish(topic, payload, opts, cb) {
    this.published.push({ topic, payload: String(payload), opts });
    this.calls.push({ kind: 'publish', topic, payload: String(payload) });
    if (typeof cb === 'function') queueMicrotask(() => cb(null));
    return this;
  }

  subscribe(topic, opts, cb) {
    this.subscribed.push({ topic, opts });
    const done = typeof opts === 'function' ? opts : cb;
    if (typeof done === 'function') queueMicrotask(() => done(null, [{ topic, qos: 1 }]));
    return this;
  }

  end(force, cb) {
    this.ended = true;
    this.calls.push({ kind: 'end' });
    const done = typeof force === 'function' ? force : cb;
    if (typeof done === 'function') queueMicrotask(() => done());
    return this;
  }

  /** Simule un message retained recu du broker. */
  deliver(topic, payload) {
    this.emit('message', topic, Buffer.from(payload), { retain: true });
  }
}
