import { SELF } from "cloudflare:test";
import { describe, expect, it } from "vitest";
import { call, newDevice, register } from "./helpers";

async function ws(path: string) {
  const res = await SELF.fetch(`https://relay.test${path}`, { headers: { Upgrade: "websocket" } });
  expect(res.status).toBe(101);
  const sock = res.webSocket!;
  sock.accept();
  const inbox: string[] = [];
  sock.addEventListener("message", (e) => inbox.push(e.data as string));
  return { sock, inbox };
}
const until = async (f: () => boolean) => { for (let i = 0; i < 50 && !f(); i++) await new Promise((r) => setTimeout(r, 20)); };

describe("rendezvous", () => {
  it("relays frames both ways, buffers for a late joiner, and caps participants", async () => {
    const rid = "r".repeat(22);
    const a = await ws(`/v1/rendezvous/${rid}`);
    a.sock.send("hello-from-a"); // peer not here yet: buffered
    const b = await ws(`/v1/rendezvous/${rid}`);
    await until(() => b.inbox.length === 1);
    expect(b.inbox).toEqual(["hello-from-a"]);
    b.sock.send("reply");
    await until(() => a.inbox.length === 1);
    expect(a.inbox).toEqual(["reply"]);
    const third = await SELF.fetch(`https://relay.test/v1/rendezvous/${rid}`, { headers: { Upgrade: "websocket" } });
    expect(third.status).toBe(409);
  });

  it("closes a connection that exceeds the frame cap", async () => {
    const a = await ws(`/v1/rendezvous/${"c".repeat(22)}`);
    let closed = false;
    a.sock.addEventListener("close", () => (closed = true));
    for (let i = 0; i < 9; i++) a.sock.send("x");
    await until(() => closed);
    expect(closed).toBe(true);
  });
});

describe("blobs", () => {
  async function paired() {
    const a = await newDevice(), b = await newDevice();
    await register(a); await register(b);
    await call(b, "POST", "/v1/peers", { peer_id: a.id });
    return { a, b };
  }

  it("upload, owner/recipient read, stranger denied, delete", async () => {
    const { a, b } = await paired();
    const data = crypto.getRandomValues(new Uint8Array(1000));
    const created = await call(a, "POST", "/v1/blobs", { size: data.length, recipient: b.id });
    expect(created.status).toBe(201);
    const { blob_id, put_url } = await created.json<any>();

    // Not readable before the upload completes.
    expect((await call(b, "GET", `/v1/blobs/${blob_id}`)).status).toBe(409);

    const put = await (async () => {
      const { canonical } = await import("../src/auth");
      const { b64u } = await import("../src/encoding");
      const ts = Math.floor(Date.now() / 1000);
      const sig = new Uint8Array(await crypto.subtle.sign("Ed25519", a.priv, await canonical("PUT", put_url, ts, data)));
      return SELF.fetch(`https://relay.test${put_url}`, {
        method: "PUT", body: data, headers: { Authorization: `Baton-Sig id=${a.id},ts=${ts},sig=${b64u.encode(sig)}` },
      });
    })();
    expect(put.status).toBe(200);

    const got = new Uint8Array(await (await call(b, "GET", `/v1/blobs/${blob_id}`)).arrayBuffer());
    expect(got).toEqual(data);

    const c = await newDevice(); await register(c);
    expect((await call(c, "GET", `/v1/blobs/${blob_id}`)).status).toBe(403);

    expect((await call(b, "DELETE", `/v1/blobs/${blob_id}`)).status).toBe(200);
    expect((await call(b, "GET", `/v1/blobs/${blob_id}`)).status).toBe(404);
  });

  it("refuses blobs for unpaired recipients and oversize files", async () => {
    const { a, b } = await paired();
    const c = await newDevice(); await register(c);
    expect((await call(a, "POST", "/v1/blobs", { size: 10, recipient: c.id })).status).toBe(403);
    expect((await call(a, "POST", "/v1/blobs", { size: 10 ** 9, recipient: b.id })).status).toBe(400);
  });
});
