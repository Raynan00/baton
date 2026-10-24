import { defineWorkersConfig } from "@cloudflare/vitest-pool-workers/config";

// isolatedStorage is off: SQLite-backed DOs trip the isolated-storage stack (WAL files).
// Tests use fresh random device ids, so they don't share state.
export default defineWorkersConfig({
  test: {
    poolOptions: { workers: { isolatedStorage: false, wrangler: { configPath: "./wrangler.toml" } } },
  },
});
