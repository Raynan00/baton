import { describe, expect, it } from "vitest";
import { buildApnsRequest, MockApns, PUSH, setApnsSender, signApnsJwt } from "../src/apns";
import { b64u } from "../src/encoding";
import { call, envelopeWithPush, newDevice, register } from "./helpers";

const base = {
  tokens: { apns_token: "tok", la_push_to_start_token: "lat", la_activity_token: "act", apns_env: "sandbox" as const },
  bundle: "app.batonsync", collapseKey: new Uint8Array(16).fill(7), msgIdHex: "ab", envelope: new Uint8Array([1, 2, 3]), nowS: 1000,
};

describe("buildApnsRequest", () => {
  it("alert: mutable-content, collapse-id, generic text, sandbox host", () => {
    const r = buildApnsRequest({ ...base, hint: PUSH.ALERT })!;
    expect(r.host).toBe("api.sandbox.push.apple.com");
    expect(r.headers["apns-push-type"]).toBe("alert");
    expect(r.headers["apns-collapse-id"]).toBe(b64u.encode(base.collapseKey));
    const aps = (r.body as any).aps;
    expect(aps["mutable-content"]).toBe(1);
    expect(aps.alert.body).toBe("New item from your PC");
    expect(JSON.stringify(r.body)).not.toMatch(/https?:/);
  });

  it("live activity start / update / end", () => {
    const s = buildApnsRequest({ ...base, hint: PUSH.LA_START })!;
    expect(s.headers["apns-push-type"]).toBe("liveactivity");
    expect(s.headers["apns-topic"]).toBe("app.batonsync.push-type.liveactivity");
    expect(s.token).toBe("lat");
    expect((s.body as any).aps).toMatchObject({ event: "start", "attributes-type": "ContinueAttributes" });
    const u = buildApnsRequest({ ...base, hint: PUSH.LA_UPDATE })!;
    expect(u.token).toBe("act");
    expect((u.body as any).aps.event).toBe("update");
    const e = buildApnsRequest({ ...base, hint: PUSH.LA_END })!;
    expect((e.body as any).aps).toMatchObject({ event: "end", "dismissal-date": 1000 });
  });

  it("oversize envelope sends only the id; missing token yields no request", () => {
    const big = buildApnsRequest({ ...base, hint: PUSH.ALERT, envelope: new Uint8Array(4000) })!;
    expect(big.body).toMatchObject({ id: "ab" });
    expect((big.body as any).m).toBeUndefined();
    expect(buildApnsRequest({ ...base, tokens: {}, hint: PUSH.ALERT })).toBeNull();
  });
});

describe("ES256 JWT", () => {
  it("signs a verifiable token", async () => {
    const kp = (await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, ["sign", "verify"])) as CryptoKeyPair;
    const der = new Uint8Array((await crypto.subtle.exportKey("pkcs8", kp.privateKey)) as ArrayBuffer);
    const pem = `-----BEGIN PRIVATE KEY-----\n${btoa(String.fromCharCode(...der))}\n-----END PRIVATE KEY-----`;
    const jwt = await signApnsJwt({ APNS_KEY_P8: pem, APNS_KEY_ID: "KID", APNS_TEAM_ID: "TEAM" }, 5);
    const [h, p, sig] = jwt.split(".");
    expect(JSON.parse(new TextDecoder().decode(b64u.decode(h)))).toEqual({ alg: "ES256", kid: "KID" });
    expect(JSON.parse(new TextDecoder().decode(b64u.decode(p)))).toEqual({ iss: "TEAM", iat: 5 });
    const ok = await crypto.subtle.verify({ name: "ECDSA", hash: "SHA-256" }, kp.publicKey, b64u.decode(sig), new TextEncoder().encode(`${h}.${p}`));
    expect(ok).toBe(true);
  });
});

describe("push through the relay", () => {
  it("sends an alert to the recipient's token, honours rate limit and 410", async () => {
    const a = await newDevice(), b = await newDevice();
    await register(a); await register(b);
    await call(b, "POST", "/v1/peers", { peer_id: a.id });
    await call(b, "PUT", "/v1/devices/me/push", { apns_token: "btok", apns_env: "sandbox" });
    const mock = new MockApns();
    setApnsSender(mock);
    try {
      const e = envelopeWithPush(a.id, b.id, PUSH.ALERT);
      const r = await (await call(a, "POST", "/v1/messages", { envelopes: [e.b64] })).json<any>();
      expect(r.accepted).toEqual([e.msgId]);
      expect(mock.requests).toHaveLength(1);
      expect(mock.requests[0].token).toBe("btok");

      for (let i = 0; i < 11; i++) await call(a, "POST", "/v1/messages", { envelopes: [envelopeWithPush(a.id, b.id, PUSH.ALERT).b64] });
      const over = await (await call(a, "POST", "/v1/messages", { envelopes: [envelopeWithPush(a.id, b.id, PUSH.ALERT).b64] })).json<any>();
      expect(over.rejected[0].reason).toBe("rate_limited");

      // 410 marks the token dead: no further requests are built for it.
      mock.next = { status: 410, reason: "Unregistered" };
      const c = await newDevice(); await register(c);
      await call(c, "POST", "/v1/peers", { peer_id: a.id });
      await call(c, "PUT", "/v1/devices/me/push", { apns_token: "ctok" });
      await call(a, "POST", "/v1/messages", { envelopes: [envelopeWithPush(a.id, c.id, PUSH.ALERT).b64] });
      const n = mock.requests.length;
      await call(a, "POST", "/v1/messages", { envelopes: [envelopeWithPush(a.id, c.id, PUSH.ALERT).b64] });
      expect(mock.requests.length).toBe(n);
    } finally {
      setApnsSender(undefined);
    }
  });
});
