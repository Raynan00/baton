import { DurableObject } from "cloudflare:workers";
import { canonical, deviceIdFor, parseAuth, skewOk, verify } from "./auth";
import { b64u, hex } from "./encoding";
import { PUSH, apnsSenderFor, buildApnsRequest, type PushTokens } from "./apns";
import { parseEnvelope } from "./envelope";

export interface Env {
  DEVICE: DurableObjectNamespace<DeviceDO>;
  RENDEZVOUS: DurableObjectNamespace;
  BLOBS: R2Bucket;
  ENV: string;
  MODE: string;
  APNS_KEY_P8?: string;
  APNS_KEY_ID?: string;
  APNS_TEAM_ID?: string;
  APNS_TOPIC?: string;
}

const MAX_TTL_S = 86_400;
const MAX_ALERTS_PER_HOUR = 12;
// Single-request PUTs are buffered for signing, so stay well under the 128 MB isolate limit; chunked uploads come later.
const MAX_BLOB_BYTES = 32 * 1024 * 1024;
const DAILY_BLOB_BYTES = 1024 * 1024 * 1024;
const json = (o: unknown, status = 200) => Response.json(o, { status });

export class DeviceDO extends DurableObject<Env> {
  private sql = this.ctx.storage.sql;

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    this.sql.exec(`
      CREATE TABLE IF NOT EXISTS device (id TEXT PRIMARY KEY, ed_pub BLOB, x_pub BLOB, platform TEXT);
      CREATE TABLE IF NOT EXISTS push (id INTEGER PRIMARY KEY CHECK (id = 1), tokens TEXT);
      CREATE TABLE IF NOT EXISTS blob_usage (at_ms INTEGER, bytes INTEGER);
      CREATE TABLE IF NOT EXISTS alerts (at_ms INTEGER);
      CREATE TABLE IF NOT EXISTS peers (peer_id TEXT PRIMARY KEY);
      CREATE TABLE IF NOT EXISTS mailbox (seq INTEGER PRIMARY KEY AUTOINCREMENT, msg_id TEXT UNIQUE,
        envelope BLOB, expires_ms INTEGER);`);
  }

  private device() {
    return this.sql.exec("SELECT id, ed_pub, x_pub FROM device LIMIT 1").toArray()[0] as
      | { id: string; ed_pub: ArrayBuffer; x_pub: ArrayBuffer } | undefined;
  }

  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const body = new Uint8Array(await req.arrayBuffer());
    const auth = parseAuth(req.headers.get("Authorization"));
    if (!auth) return json({ error: "unauthorized" }, 401);
    if (!skewOk(auth.ts)) return json({ error: "skew" }, 401);
    const msg = await canonical(req.method, url.pathname + url.search, auth.ts, body);

    if (req.method === "POST" && url.pathname === "/v1/devices") return this.register(auth, msg, body);

    const dev = this.device();
    if (!dev || dev.id !== auth.id || !(await verify(new Uint8Array(dev.ed_pub), msg, auth.sig))) {
      return json({ error: "unauthorized" }, 401);
    }
    const route = `${req.method} ${url.pathname}`;
    if (route === "PUT /v1/devices/me/push") return this.setPush(body);
    if (route === "POST /v1/peers") return this.addPeer(body);
    if (req.method === "DELETE" && url.pathname.startsWith("/v1/peers/")) {
      this.sql.exec("DELETE FROM peers WHERE peer_id = ?", url.pathname.slice(10));
      return json({});
    }
    if (route === "POST /v1/blobs") return this.newBlob(dev.id, body);
    if (url.pathname.startsWith("/v1/blobs/")) return this.blob(dev.id, req, url.pathname.slice(10), body);
    if (route === "POST /v1/messages") return this.send(dev.id, body);
    if (route === "GET /v1/messages") return this.drain(url);
    if (req.method === "DELETE" && url.pathname.startsWith("/v1/messages/")) {
      this.sql.exec("DELETE FROM mailbox WHERE msg_id = ?", url.pathname.slice(13));
      return json({});
    }
    if (route === "DELETE /v1/devices/me") {
      await this.ctx.storage.deleteAll();
      return json({});
    }
    if (route === "GET /v1/stream" && req.headers.get("Upgrade") === "websocket") return this.stream();
    return json({ error: "not found" }, 404);
  }

  private async register(auth: { id: string; sig: Uint8Array }, msg: Uint8Array, body: Uint8Array) {
    if (this.device()) return json({ error: "exists" }, 409);
    let b: { ed25519_pub: string; x25519_pub: string; platform: string };
    try { b = JSON.parse(new TextDecoder().decode(body)); } catch { return json({ error: "bad body" }, 400); }
    const ed = b64u.decode(b.ed25519_pub ?? "");
    const x = b64u.decode(b.x25519_pub ?? "");
    if (ed.length !== 32 || x.length !== 32) return json({ error: "bad key" }, 400);
    const id = await deviceIdFor(ed);
    if (id !== auth.id || !(await verify(ed, msg, auth.sig))) return json({ error: "unauthorized" }, 401);
    this.sql.exec("INSERT INTO device VALUES (?, ?, ?, ?)", id, ed, x, String(b.platform ?? ""));
    return json({ device_id: id }, 201);
  }

  /** The caller (B) authorizes `peer_id` (A) to send to B. */
  private addPeer(body: Uint8Array) {
    let b: { peer_id?: string };
    try { b = JSON.parse(new TextDecoder().decode(body)); } catch { return json({ error: "bad body" }, 400); }
    if (!b.peer_id || !/^[0-9a-f]{32}$/.test(b.peer_id)) return json({ error: "bad peer" }, 400);
    this.sql.exec("INSERT OR IGNORE INTO peers VALUES (?)", b.peer_id);
    return json({}, 201);
  }

  private async send(senderId: string, body: Uint8Array) {
    let b: { envelopes?: string[] };
    try { b = JSON.parse(new TextDecoder().decode(body)); } catch { return json({ error: "bad body" }, 400); }
    const accepted: string[] = [];
    const rejected: { msg_id: string; reason: string }[] = [];
    for (const e of b.envelopes ?? []) {
      let bytes: Uint8Array, h;
      try { bytes = b64u.decode(e); h = parseEnvelope(bytes); } catch { rejected.push({ msg_id: "", reason: "malformed" }); continue; }
      const msgId = hex(h.msgId);
      if (h.v !== 1 || h.msgId.length !== 16 || hex(h.sender) !== senderId || h.recipient.length !== 16) {
        rejected.push({ msg_id: msgId, reason: "malformed" });
        continue;
      }
      const res = await this.env.DEVICE.get(this.env.DEVICE.idFromName(hex(h.recipient)))
        .deliver(senderId, msgId, bytes, Math.min(h.ttlS || MAX_TTL_S, MAX_TTL_S));
      if (res === "ok") accepted.push(msgId);
      else rejected.push({ msg_id: msgId, reason: res });
    }
    return json({ accepted, rejected });
  }

  /** RPC: may `senderId` send to this device? */
  async allows(senderId: string): Promise<boolean> {
    return this.sql.exec("SELECT 1 FROM peers WHERE peer_id = ?", senderId).toArray().length > 0;
  }

  /** `POST /v1/blobs {size, recipient}` -> `{blob_id, put_url, expires_at}`; quota-checked. */
  private async newBlob(owner: string, body: Uint8Array) {
    let b: { size?: number; recipient?: string };
    try { b = JSON.parse(new TextDecoder().decode(body)); } catch { return json({ error: "bad body" }, 400); }
    if (!b.size || b.size < 1 || b.size > MAX_BLOB_BYTES || !b.recipient || !/^[0-9a-f]{32}$/.test(b.recipient)) {
      return json({ error: "bad request" }, 400);
    }
    if (!(await this.env.DEVICE.get(this.env.DEVICE.idFromName(b.recipient)).allows(owner))) return json({ error: "not_paired" }, 403);
    this.sql.exec("DELETE FROM blob_usage WHERE at_ms < ?", Date.now() - 86_400_000);
    const used = (this.sql.exec("SELECT COALESCE(SUM(bytes), 0) AS n FROM blob_usage").toArray()[0] as { n: number }).n;
    if (used + b.size > DAILY_BLOB_BYTES) return json({ error: "quota" }, 429);
    this.sql.exec("INSERT INTO blob_usage VALUES (?, ?)", Date.now(), b.size);
    const id = crypto.randomUUID().replace(/-/g, "");
    await this.env.BLOBS.put(`${owner}/${id}`, new Uint8Array(0), { customMetadata: { recipient: b.recipient, size: String(b.size), state: "pending" } });
    return json({ blob_id: `${owner}.${id}`, put_url: `/v1/blobs/${owner}.${id}`, expires_at: Date.now() + 86_400_000 }, 201);
  }

  /** PUT (owner uploads), GET (owner or recipient, Range ok), DELETE (owner or recipient = ack). */
  private async blob(caller: string, req: Request, blobId: string, body: Uint8Array) {
    const [owner, id] = blobId.split(".");
    if (!owner || !id || !/^[0-9a-f]+$/.test(id)) return json({ error: "not found" }, 404);
    const key = `${owner}/${id}`;
    const head = await this.env.BLOBS.head(key);
    if (!head) return json({ error: "not found" }, 404);
    const recipient = head.customMetadata?.recipient;
    const isOwner = caller === owner;
    if (!isOwner && caller !== recipient) return json({ error: "forbidden" }, 403);
    if (req.method === "PUT") {
      if (!isOwner || body.length !== Number(head.customMetadata?.size)) return json({ error: "bad upload" }, 400);
      await this.env.BLOBS.put(key, body, { customMetadata: { ...head.customMetadata, state: "ready" } });
      return json({});
    }
    if (req.method === "DELETE") {
      await this.env.BLOBS.delete(key);
      return json({});
    }
    if (req.method === "GET") {
      if (head.customMetadata?.state !== "ready") return json({ error: "not ready" }, 409);
      const obj = await this.env.BLOBS.get(key, { range: req.headers });
      if (!obj || !("body" in obj)) return json({ error: "not found" }, 404);
      return new Response(obj.body, { status: req.headers.has("Range") ? 206 : 200, headers: { "content-type": "application/octet-stream" } });
    }
    return json({ error: "not found" }, 404);
  }

  /** RPC from a sender's DO. Only paired senders are accepted. */
  async deliver(senderId: string, msgId: string, envelope: Uint8Array, ttlS: number): Promise<string> {
    if (!this.device()) return "unknown_recipient";
    if (!this.sql.exec("SELECT 1 FROM peers WHERE peer_id = ?", senderId).toArray().length) return "not_paired";
    const h = parseEnvelope(envelope);
    if (h.push === PUSH.ALERT && this.alertsLastHour() >= MAX_ALERTS_PER_HOUR) return "rate_limited";
    const expires = Date.now() + ttlS * 1000;
    this.sql.exec("INSERT OR IGNORE INTO mailbox (msg_id, envelope, expires_ms) VALUES (?, ?, ?)", msgId, envelope, expires);
    for (const ws of this.ctx.getWebSockets()) ws.send(b64u.encode(envelope));
    await this.scheduleAlarm();
    await this.push(h, msgId, envelope);
    return "ok";
  }

  private alertsLastHour(): number {
    const cutoff = Date.now() - 3_600_000;
    this.sql.exec("DELETE FROM alerts WHERE at_ms < ?", cutoff);
    return (this.sql.exec("SELECT COUNT(*) AS n FROM alerts").toArray()[0] as { n: number }).n;
  }

  private tokens(): PushTokens {
    const r = this.sql.exec("SELECT tokens FROM push WHERE id = 1").toArray()[0] as { tokens: string } | undefined;
    return r ? JSON.parse(r.tokens) : {};
  }

  private setPush(body: Uint8Array) {
    let t: PushTokens;
    try { t = JSON.parse(new TextDecoder().decode(body)); } catch { return json({ error: "bad body" }, 400); }
    const merged = { ...this.tokens(), ...t };
    this.sql.exec("INSERT OR REPLACE INTO push VALUES (1, ?)", JSON.stringify(merged));
    return json({});
  }

  /** Best effort: a push failure never fails delivery (the mailbox still holds the message). */
  private async push(h: ReturnType<typeof parseEnvelope>, msgId: string, envelope: Uint8Array) {
    if (h.push === PUSH.NONE) return;
    const sender = apnsSenderFor(this.env);
    if (!sender) return;
    const req = buildApnsRequest({
      hint: h.push, tokens: this.tokens(), bundle: this.env.APNS_TOPIC ?? "app.batonsync",
      collapseKey: h.collapseKey, msgIdHex: msgId, envelope, nowS: Math.floor(Date.now() / 1000),
    });
    if (!req) return;
    if (h.push === PUSH.ALERT) this.sql.exec("INSERT INTO alerts VALUES (?)", Date.now());
    try {
      const res = await sender.send(req);
      if (res.status === 410) {
        // Unregistered: mark the token dead.
        const t = this.tokens();
        if (req.token === t.apns_token) t.apns_token = null;
        if (req.token === t.la_push_to_start_token) t.la_push_to_start_token = null;
        if (req.token === t.la_activity_token) t.la_activity_token = null;
        this.sql.exec("INSERT OR REPLACE INTO push VALUES (1, ?)", JSON.stringify(t));
      }
    } catch { /* ignore */ }
  }

  private drain(url: URL) {
    const since = Number(url.searchParams.get("since") ?? 0);
    const rows = this.sql.exec("SELECT seq, envelope FROM mailbox WHERE seq > ? ORDER BY seq", since).toArray() as
      { seq: number; envelope: ArrayBuffer }[];
    return json({
      envelopes: rows.map((r) => b64u.encode(new Uint8Array(r.envelope))),
      cursor: rows.length ? rows[rows.length - 1].seq : since,
    });
  }

  private stream() {
    const pair = new WebSocketPair();
    this.ctx.acceptWebSocket(pair[1]);
    const rows = this.sql.exec("SELECT envelope FROM mailbox ORDER BY seq").toArray() as { envelope: ArrayBuffer }[];
    for (const r of rows) pair[1].send(b64u.encode(new Uint8Array(r.envelope)));
    return new Response(null, { status: 101, webSocket: pair[0] });
  }

  /** Client → server: `{"ack":"<msg_id hex>"}` deletes the message. */
  async webSocketMessage(_ws: WebSocket, data: string | ArrayBuffer) {
    try {
      const m = JSON.parse(typeof data === "string" ? data : new TextDecoder().decode(data));
      if (typeof m.ack === "string") this.sql.exec("DELETE FROM mailbox WHERE msg_id = ?", m.ack);
    } catch { /* ignore */ }
  }

  private async scheduleAlarm() {
    const r = this.sql.exec("SELECT MIN(expires_ms) AS m FROM mailbox").toArray()[0] as { m: number | null };
    if (r.m) await this.ctx.storage.setAlarm(r.m);
  }

  async alarm() {
    this.sql.exec("DELETE FROM mailbox WHERE expires_ms <= ?", Date.now());
    await this.scheduleAlarm();
  }
}
