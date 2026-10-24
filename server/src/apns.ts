import { b64u } from "./encoding";

export const PUSH = { NONE: 0, ALERT: 1, LA_START: 2, LA_UPDATE: 3, LA_END: 4 } as const;
export const GENERIC = { title: "Baton", body: "New item from your PC" };
const APNS_MAX_INLINE = 3 * 1024;

export interface ApnsRequest {
  host: string;
  token: string;
  headers: Record<string, string>;
  body: Record<string, unknown>;
}
export interface ApnsResult { status: number; reason?: string }
export interface ApnsSender { send(req: ApnsRequest): Promise<ApnsResult> }

export interface PushTokens {
  apns_token?: string | null;
  apns_env?: "sandbox" | "production";
  la_push_to_start_token?: string | null;
  la_activity_token?: string | null;
}

export interface BuildInput {
  hint: number;
  tokens: PushTokens;
  bundle: string;
  collapseKey: Uint8Array;
  msgIdHex: string;
  envelope: Uint8Array;
  nowS: number;
}

/** Content-free request. The envelope rides along as ciphertext (or just its id when too large). */
export function buildApnsRequest(i: BuildInput): ApnsRequest | null {
  const host = i.tokens.apns_env === "sandbox" ? "api.sandbox.push.apple.com" : "api.push.apple.com";
  const collapse = b64u.encode(i.collapseKey).slice(0, 64);
  const inline = i.envelope.length <= APNS_MAX_INLINE;
  const payload = inline ? { m: b64u.encode(i.envelope) } : { id: i.msgIdHex };
  const enc = inline ? b64u.encode(i.envelope) : "";
  switch (i.hint) {
    case PUSH.ALERT:
      if (!i.tokens.apns_token) return null;
      return {
        host, token: i.tokens.apns_token,
        headers: {
          "apns-push-type": "alert", "apns-priority": "10", "apns-topic": i.bundle,
          ...(collapse && { "apns-collapse-id": collapse }),
        },
        body: {
          aps: { alert: GENERIC, "mutable-content": 1, "thread-id": "continue", "interruption-level": "active", category: "CONTINUE" },
          ...payload,
        },
      };
    case PUSH.LA_START:
      if (!i.tokens.la_push_to_start_token) return null;
      return laRequest(host, i.tokens.la_push_to_start_token, i, {
        event: "start", "attributes-type": "ContinueAttributes", attributes: {},
        "content-state": { enc, title: "Continue from your PC", progress: 0 }, alert: GENERIC,
      });
    case PUSH.LA_UPDATE:
      if (!i.tokens.la_activity_token) return null;
      return laRequest(host, i.tokens.la_activity_token, i, {
        event: "update", "content-state": { enc, title: "Continue from your PC", progress: 0 },
      });
    case PUSH.LA_END:
      if (!i.tokens.la_activity_token) return null;
      return laRequest(host, i.tokens.la_activity_token, i, { event: "end", "dismissal-date": i.nowS, "content-state": { enc: "", title: "", progress: 0 } });
    default:
      return null;
  }
}

function laRequest(host: string, token: string, i: BuildInput, aps: Record<string, unknown>): ApnsRequest {
  return {
    host, token,
    headers: { "apns-push-type": "liveactivity", "apns-priority": "10", "apns-topic": `${i.bundle}.push-type.liveactivity` },
    body: { aps: { timestamp: i.nowS, ...aps } },
  };
}

// ---- real sender (production Workers only; local workerd lacks HTTP/2, see BATON.md §10.5) ----

export interface ApnsSecrets { APNS_KEY_P8: string; APNS_KEY_ID: string; APNS_TEAM_ID: string }

export class WorkerApnsSender implements ApnsSender {
  private jwt?: { value: string; at: number };
  constructor(private s: ApnsSecrets) {}

  async send(r: ApnsRequest): Promise<ApnsResult> {
    const res = await fetch(`https://${r.host}/3/device/${r.token}`, {
      method: "POST",
      headers: { ...r.headers, authorization: `bearer ${await this.token()}`, "content-type": "application/json" },
      body: JSON.stringify(r.body),
    });
    if (res.ok) return { status: res.status };
    const j = (await res.json().catch(() => ({}))) as { reason?: string };
    return { status: res.status, reason: j.reason };
  }

  private async token(): Promise<string> {
    const now = Math.floor(Date.now() / 1000);
    if (this.jwt && now - this.jwt.at < 50 * 60) return this.jwt.value;
    this.jwt = { value: await signApnsJwt(this.s, now), at: now };
    return this.jwt.value;
  }
}

export async function signApnsJwt(s: ApnsSecrets, nowS: number): Promise<string> {
  const pem = s.APNS_KEY_P8.replace(/-----[^-]+-----|\s/g, "");
  const key = await crypto.subtle.importKey("pkcs8", b64u.decode(pem.replace(/\+/g, "-").replace(/\//g, "_")),
    { name: "ECDSA", namedCurve: "P-256" }, false, ["sign"]);
  const enc = (o: unknown) => b64u.encode(new TextEncoder().encode(JSON.stringify(o)));
  const input = `${enc({ alg: "ES256", kid: s.APNS_KEY_ID })}.${enc({ iss: s.APNS_TEAM_ID, iat: nowS })}`;
  const sig = new Uint8Array(await crypto.subtle.sign({ name: "ECDSA", hash: "SHA-256" }, key, new TextEncoder().encode(input)));
  return `${input}.${b64u.encode(sig)}`;
}

// ---- test seam ----
let override: ApnsSender | undefined;
export function setApnsSender(s: ApnsSender | undefined) { override = s; }
export function apnsSenderFor(env: Partial<ApnsSecrets>): ApnsSender | null {
  if (override) return override;
  if (env.APNS_KEY_P8 && env.APNS_KEY_ID && env.APNS_TEAM_ID) return new WorkerApnsSender(env as ApnsSecrets);
  return null;
}

/** Records requests; used by tests and `mock-apns`. */
export class MockApns implements ApnsSender {
  requests: ApnsRequest[] = [];
  next: ApnsResult = { status: 200 };
  async send(r: ApnsRequest) { this.requests.push(r); return this.next; }
}
