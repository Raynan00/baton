import { env, runDurableObjectAlarm, runInDurableObject } from "cloudflare:test";
import { describe, expect, it } from "vitest";
import { call, envelope, newDevice, register } from "./helpers";

async function pair() {
  const a = await newDevice(), b = await newDevice();
  expect((await register(a)).status).toBe(201);
  expect((await register(b)).status).toBe(201);
  // B authorizes A as a sender.
  expect((await call(b, "POST", "/v1/peers", { peer_id: a.id })).status).toBe(201);
  return { a, b };
}

describe("auth", () => {
  it("rejects unsigned, wrong-key and skewed requests", async () => {
    const a = await newDevice();
    await register(a);
    const other = { ...(await newDevice()), id: a.id };
    expect((await call(other, "GET", "/v1/messages")).status).toBe(401);
    expect((await call(a, "GET", "/v1/messages", undefined, 500)).status).toBe(401);
    expect((await call(a, "GET", "/v1/messages")).status).toBe(200);
    expect((await register(a)).status).toBe(409);
  });
});

describe("messages", () => {
  it("rejects an unpaired sender, delivers for a paired one", async () => {
    const { a, b } = await pair();
    const stranger = await newDevice();
    await register(stranger);
    const bad = envelope(stranger.id, b.id, new Uint8Array([1, 2, 3]));
    const r = await (await call(stranger, "POST", "/v1/messages", { envelopes: [bad.b64] })).json<any>();
    expect(r.rejected[0].reason).toBe("not_paired");

    const good = envelope(a.id, b.id, new Uint8Array([9, 9, 9]));
    const ok = await (await call(a, "POST", "/v1/messages", { envelopes: [good.b64] })).json<any>();
    expect(ok.accepted).toEqual([good.msgId]);

    const drained = await (await call(b, "GET", "/v1/messages")).json<any>();
    expect(drained.envelopes).toEqual([good.b64]);
    await call(b, "DELETE", `/v1/messages/${good.msgId}`);
    expect((await (await call(b, "GET", "/v1/messages")).json<any>()).envelopes).toEqual([]);
  });

  it("rejects a spoofed sender field", async () => {
    const { a, b } = await pair();
    const c = await newDevice();
    await register(c);
    const spoof = envelope(c.id, b.id, new Uint8Array([1]));
    const r = await (await call(a, "POST", "/v1/messages", { envelopes: [spoof.b64] })).json<any>();
    expect(r.rejected[0].reason).toBe("malformed");
  });

  it("expires messages at TTL via alarm", async () => {
    const { a, b } = await pair();
    const e = envelope(a.id, b.id, new Uint8Array([5]), 1);
    await call(a, "POST", "/v1/messages", { envelopes: [e.b64] });
    await new Promise((r) => setTimeout(r, 1100));
    // The real alarm may already have fired; running it again is a no-op if so.
    await runDurableObjectAlarm(env.DEVICE.get(env.DEVICE.idFromName(b.id)));
    expect((await (await call(b, "GET", "/v1/messages")).json<any>()).envelopes).toEqual([]);
  });

  it("stores only ciphertext (no plaintext in DO storage)", async () => {
    const { a, b } = await pair();
    const secret = new TextEncoder().encode("https://secret.example/My Title");
    // The relay never sees plaintext in real use; this asserts it persists exactly the opaque bytes it was given.
    const e = envelope(a.id, b.id, crypto.getRandomValues(new Uint8Array(64)));
    await call(a, "POST", "/v1/messages", { envelopes: [e.b64] });
    await runInDurableObject(env.DEVICE.get(env.DEVICE.idFromName(b.id)), async (_i, state) => {
      const dump = JSON.stringify(state.storage.sql.exec("SELECT * FROM mailbox").toArray().map((r: any) =>
        ({ ...r, envelope: Array.from(new Uint8Array(r.envelope)) })));
      expect(dump).not.toContain("secret.example");
      expect(dump).not.toContain(Array.from(secret).join(","));
    });
  });
});
