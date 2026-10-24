import { SELF } from "cloudflare:test";
import { canonical, deviceIdFor } from "../src/auth";
import { b64u, hex } from "../src/encoding";

export interface TestDevice { id: string; priv: CryptoKey; pub: Uint8Array }

export async function newDevice(): Promise<TestDevice> {
  const kp = (await crypto.subtle.generateKey({ name: "Ed25519" }, true, ["sign", "verify"])) as CryptoKeyPair;
  const pub = new Uint8Array((await crypto.subtle.exportKey("raw", kp.publicKey)) as ArrayBuffer);
  return { id: await deviceIdFor(pub), priv: kp.privateKey, pub };
}

export async function call(d: TestDevice, method: string, path: string, body?: unknown, tsOffset = 0) {
  const raw = body === undefined ? new Uint8Array() : new TextEncoder().encode(JSON.stringify(body));
  const ts = Math.floor(Date.now() / 1000) + tsOffset;
  const sig = new Uint8Array(await crypto.subtle.sign("Ed25519", d.priv, await canonical(method, path, ts, raw)));
  return SELF.fetch(`https://relay.test${path}`, {
    method,
    body: method === "GET" || method === "DELETE" ? undefined : raw,
    headers: { Authorization: `Baton-Sig id=${d.id},ts=${ts},sig=${b64u.encode(sig)}` },
  });
}

export async function register(d: TestDevice) {
  return call(d, "POST", "/v1/devices", {
    ed25519_pub: b64u.encode(d.pub), x25519_pub: b64u.encode(new Uint8Array(32).fill(1)), platform: "test",
  });
}

function varint(n: number): number[] {
  const o: number[] = [];
  while (n >= 128) { o.push((n % 128) | 128); n = Math.floor(n / 128); }
  o.push(n);
  return o;
}
const bytesField = (f: number, b: Uint8Array) => [(f << 3) | 2, ...varint(b.length), ...b];
const varField = (f: number, n: number) => [(f << 3) | 0, ...varint(n)];
const fromHex = (h: string) => Uint8Array.from(h.match(/../g)!.map((x) => parseInt(x, 16)));

/** Hand-built Envelope; `ciphertext` is opaque (field 10). */
export function envelope(from: string, to: string, ciphertext: Uint8Array, ttlS = 60) {
  const msgId = crypto.getRandomValues(new Uint8Array(16));
  const bytes = new Uint8Array([
    ...varField(1, 1), ...bytesField(2, msgId), ...bytesField(3, fromHex(from)), ...bytesField(4, fromHex(to)),
    ...varField(5, Date.now()), ...varField(6, ttlS), ...bytesField(10, ciphertext),
  ]);
  return { b64: b64u.encode(bytes), msgId: hex(msgId), bytes };
}

/** Same as `envelope` but with a push hint (field 7) and collapse key (field 8). */
export function envelopeWithPush(from: string, to: string, push: number, ct = new Uint8Array([1, 2, 3])) {
  const base = envelope(from, to, ct);
  const extra = [...varField(7, push), ...bytesField(8, new Uint8Array(16).fill(7))];
  const bytes = new Uint8Array([...base.bytes, ...extra]);
  return { b64: b64u.encode(bytes), msgId: base.msgId, bytes };
}
