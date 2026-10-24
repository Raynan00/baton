import type { Env } from "../src/device";

declare module "cloudflare:test" {
  interface ProvidedEnv extends Env {}
}
