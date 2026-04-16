// Type declarations for the `cloudflare:test` virtual module exposed by
// @cloudflare/vitest-pool-workers. Tells TypeScript what `env` looks like
// inside vitest tests running in the Workers pool.

declare module 'cloudflare:test' {
  interface ProvidedEnv {
    DB: D1Database;
    STATS_CACHE: KVNamespace;
  }
  export const env: ProvidedEnv;

  // Test helper for applying D1 migrations against the in-memory DB.
  export function applyD1Migrations(
    db: D1Database,
    migrations: Array<{ name: string; queries: string[] }>
  ): Promise<void>;
}
