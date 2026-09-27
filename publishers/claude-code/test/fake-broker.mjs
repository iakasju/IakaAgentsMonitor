// fake-broker.mjs — faux serveur MQTT 3.1.1 minimal en node:net, pour les tests du publieur
// (specs/instructions/agnostique-mqtt.md, étape 10) : accepte un CONNECT (répond CONNACK
// succès), enregistre chaque PUBLISH (topic, payload, retain, QoS) et répond PUBACK pour un
// QoS 1. Aucune dépendance, comme le publieur lui-même.

import net from "node:net";

function decodeRemainingLength(buf, offset) {
  let multiplier = 1;
  let value = 0;
  let i = offset;
  let byte;
  do {
    if (i >= buf.length) return null;
    byte = buf[i];
    value += (byte & 0x7f) * multiplier;
    multiplier *= 128;
    i++;
  } while ((byte & 0x80) !== 0);
  return { value, nextOffset: i };
}

/** Démarre un faux broker sur un port libre. Renvoie `{ server, port, events, close }`.
 * `events.connects` = nombre de connexions TCP acceptées ; `events.publishes` = liste
 * `{ topic, payload, retain, qos }` dans l'ordre de réception. */
export function startFakeBroker() {
  const events = { connects: 0, publishes: [] };
  const server = net.createServer((sock) => {
    events.connects++;
    let buf = Buffer.alloc(0);
    sock.on("data", (chunk) => {
      buf = Buffer.concat([buf, chunk]);
      for (;;) {
        if (buf.length < 2) break;
        const type = buf[0] & 0xf0;
        const rl = decodeRemainingLength(buf, 1);
        if (!rl) break;
        const totalLen = rl.nextOffset + rl.value;
        if (buf.length < totalLen) break;
        const packet = buf.subarray(0, totalLen);
        buf = buf.subarray(totalLen);

        if (type === 0x10) {
          sock.write(Buffer.from([0x20, 0x02, 0x00, 0x00])); // CONNACK, succès.
        } else if (type === 0x30) {
          let o = rl.nextOffset;
          const topicLen = packet.readUInt16BE(o);
          o += 2;
          const topic = packet.toString("utf8", o, o + topicLen);
          o += topicLen;
          const qos = (packet[0] & 0x06) >> 1;
          let packetId = null;
          if (qos > 0) {
            packetId = packet.readUInt16BE(o);
            o += 2;
          }
          const payload = packet.subarray(o);
          const retain = (packet[0] & 0x01) === 1;
          events.publishes.push({ topic, payload: payload.toString("utf8"), retain, qos });
          if (qos === 1 && packetId != null) {
            sock.write(Buffer.from([0x40, 0x02, packetId >> 8, packetId & 0xff]));
          }
        }
      }
    });
  });

  return new Promise((resolve) => {
    server.listen(0, "127.0.0.1", () => {
      resolve({
        server,
        port: server.address().port,
        events,
        close: () => new Promise((r) => server.close(r)),
      });
    });
  });
}
