import { parseAuth } from "./auth";
import { DeviceDO, type Env } from "./device";

export { DeviceDO };

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const { pathname } = new URL(request.url);
    if (pathname === "/healthz") return new Response("ok");
    if (!pathname.startsWith("/v1/")) return new Response("not found", { status: 404 });
    // Every /v1 request is authenticated by the caller's DeviceDO, which holds its public key.
    const auth = parseAuth(request.headers.get("Authorization"));
    if (!auth || !/^[0-9a-f]{32}$/.test(auth.id)) return Response.json({ error: "unauthorized" }, { status: 401 });
    return env.DEVICE.get(env.DEVICE.idFromName(auth.id)).fetch(request);
  },
} satisfies ExportedHandler<Env>;
