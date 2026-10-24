import { SELF } from "cloudflare:test";
import { expect, it } from "vitest";

it("serves /healthz", async () => {
  const res = await SELF.fetch("https://relay.test/healthz");
  expect(res.status).toBe(200);
  expect(await res.text()).toBe("ok");
});
