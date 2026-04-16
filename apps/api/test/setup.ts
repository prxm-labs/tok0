// Applies D1 migrations to the in-memory test database before any test
// runs. The Workers runtime has no fs, so we import the migration SQL
// as a raw string via Vite's `?raw` query.

import { applyD1Migrations, env } from 'cloudflare:test';
// @ts-expect-error -- ?raw is a Vite feature, not standard TS
import migration0001 from '../migrations/0001_init.sql?raw';
// @ts-expect-error -- ?raw is a Vite feature, not standard TS
import migration0002 from '../migrations/0002_daily_savings.sql?raw';

function parseSql(sql: string): string[] {
  const stripped = sql
    .split('\n')
    .map((line) => line.replace(/^\s*--.*$/, '').trimEnd())
    .filter((line) => line.length > 0)
    .join('\n');

  return stripped
    .split(';')
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

await applyD1Migrations(env.DB, [
  { name: '0001_init.sql', queries: parseSql(migration0001 as string) },
  { name: '0002_daily_savings.sql', queries: parseSql(migration0002 as string) },
]);
