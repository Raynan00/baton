export default {
  async fetch(request: Request): Promise<Response> {
    const { pathname } = new URL(request.url);
    if (pathname === "/healthz") return new Response("ok");
    return new Response("not found", { status: 404 });
  },
} satisfies ExportedHandler;
