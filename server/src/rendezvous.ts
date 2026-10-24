import { DurableObject } from "cloudflare:workers";

const TTL_MS = 10 * 60 * 1000;
const MAX_FRAMES = 8;
const MAX_FRAME_BYTES = 8 * 1024;

/**
 * Relays opaque Noise frames between exactly two parties (BATON.md §10.1).
 * Frames sent before the peer joins are held in storage and flushed on join.
 */
export class RendezvousDO extends DurableObject {
  async fetch(req: Request): Promise<Response> {
    if (req.headers.get("Upgrade") !== "websocket") return new Response("expected websocket", { status: 426 });
    const live = this.ctx.getWebSockets();
    if (live.length >= 2) return new Response("full", { status: 409 });
    const tag = live.some((w) => this.ctx.getTags(w).includes("a")) ? "b" : "a";
    const pair = new WebSocketPair();
    this.ctx.acceptWebSocket(pair[1], [tag]);
    if (!(await this.ctx.storage.getAlarm())) await this.ctx.storage.setAlarm(Date.now() + TTL_MS);
    const pending = (await this.ctx.storage.get<string[]>(`to:${tag}`)) ?? [];
    for (const f of pending) pair[1].send(f);
    await this.ctx.storage.delete(`to:${tag}`);
    return new Response(null, { status: 101, webSocket: pair[0] });
  }

  async webSocketMessage(ws: WebSocket, data: string | ArrayBuffer) {
    const frame = typeof data === "string" ? data : btoa(String.fromCharCode(...new Uint8Array(data)));
    const n = ((await this.ctx.storage.get<number>("frames")) ?? 0) + 1;
    if (frame.length > MAX_FRAME_BYTES || n > MAX_FRAMES) return ws.close(1008, "limit");
    await this.ctx.storage.put("frames", n);
    const me = this.ctx.getTags(ws)[0];
    const other = this.ctx.getWebSockets().find((w) => this.ctx.getTags(w)[0] !== me);
    if (other) return other.send(frame);
    const key = `to:${me === "a" ? "b" : "a"}`;
    await this.ctx.storage.put(key, [...((await this.ctx.storage.get<string[]>(key)) ?? []), frame]);
  }

  async alarm() {
    for (const w of this.ctx.getWebSockets()) w.close(1000, "expired");
    await this.ctx.storage.deleteAll();
  }
}
