import { b64u, hex, sha256 } from "./encoding";

export const MAX_SKEW_S = 120;

export interface SigHeader { id: string; ts: number; sig: Uint8Array }

/** `Authorization: Baton-Sig id=<hex device_id>,ts=<unix>,sig=<b64url>` */
export function parseAuth(h: string | null): SigHeader | null {
  if (!h?.startsWith("Baton-Sig ")) return null;
  const m: Record<string, string> = {};
  for (const kv of h.slice(10).split(",")) {
    const [k, ...v] = kv.split("=");
    m[k.trim()] = v.join("=");
  }
  const ts = Number(m.ts);
  if (!m.id || !m.sig || !Number.isInteger(ts)) return null;
  try { return { id: m.id, ts, sig: b64u.decode(m.sig) }; } catch { return null; }
}

/** Canonical signed string: METHOD \n path \n ts \n hex(sha256(body)). */
export async function canonical(method: string, path: string, ts: number, body: Uint8Array): Promise<Uint8Array> {
  return new TextEncoder().encode(`${method}\n${path}\n${ts}\n${hex(await sha256(body))}`);
}

export async function verify(edPub: Uint8Array, msg: Uint8Array, sig: Uint8Array): Promise<boolean> {
  const key = await crypto.subtle.importKey("raw", edPub, { name: "Ed25519" }, false, ["verify"]);
  return crypto.subtle.verify("Ed25519", key, sig, msg);
}

/** device_id = first 16 bytes of sha256(ed25519 public key), hex. */
export async function deviceIdFor(edPub: Uint8Array): Promise<string> {
  return hex((await sha256(edPub)).subarray(0, 16));
}

export function skewOk(ts: number, nowMs = Date.now()): boolean {
  return Math.abs(nowMs / 1000 - ts) <= MAX_SKEW_S;
}
