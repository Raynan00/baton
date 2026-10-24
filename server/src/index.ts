import { parseAuth } from "./auth";
import { DeviceDO, type Env } from "./device";
import { RendezvousDO } from "./rendezvous";

export { DeviceDO, RendezvousDO };

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const { pathname } = new URL(request.url);
    if (pathname === "/healthz") return new Response("ok");
    if (!pathname.startsWith("/v1/")) return new Response("not found", { status: 404 });
    // The rendezvous id is a one-time secret from the QR code (a capability); frames are opaque Noise messages.
    const rv = pathname.match(/^\/v1\/rendezvous\/([A-Za-z0-9_-]{16,64})$/);
    if (rv) return env.RENDEZVOUS.get(env.RENDEZVOUS.idFromName(rv[1])).fetch(request);
    // Every /v1 request is authenticated by the caller's DeviceDO, which holds its public key.
    const auth = parseAuth(request.headers.get("Authorization"));
    if (!auth || !/^[0-9a-f]{32}$/.test(auth.id)) return Response.json({ error: "unauthorized" }, { status: 401 });
    return env.DEVICE.get(env.DEVICE.idFromName(auth.id)).fetch(request);
  },
} satisfies ExportedHandler<Env>;
