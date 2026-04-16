import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { INACTIVE_DAYS, type Bindings, type StatsResponse } from '@/types';

export const statsRoute = new Hono<{ Bindings: Bindings }>();

const CACHE_TTL_SECONDS = 60;
const CACHE_KEY = 'stats:public:v1';

statsRoute.use(
  '*',
  cors({
    origin: (origin) => {
      if (!origin) return null;
      if (origin === 'https://tok0.dev' || origin === 'https://www.tok0.dev') return origin;
      if (/^https:\/\/.*\.tok0-website\.pages\.dev$/.test(origin)) return origin;
      return null;
    },
    allowMethods: ['GET', 'OPTIONS'],
    maxAge: 86400,
  })
);

statsRoute.get('/', async (c) => {
  const cached = await c.env.STATS_CACHE.get<StatsResponse>(CACHE_KEY, 'json');
  if (cached) return c.json(cached);

  const now = Date.now();
  const cutoff = now - INACTIVE_DAYS * 86400_000;
  const since30d = now - 30 * 86400_000;
  const since30dDate = new Date(since30d).toISOString().slice(0, 10);
  const db = c.env.DB;

  const total = await db
    .prepare(`SELECT COUNT(*) AS n FROM instances`)
    .first<{ n: number }>();
  const active = await db
    .prepare(`SELECT COUNT(*) AS n FROM instances WHERE last_seen >= ?`)
    .bind(cutoff)
    .first<{ n: number }>();

  const byCountry = await db
    .prepare(
      `SELECT COALESCE(country, '??') AS country, COUNT(*) AS n
       FROM instances WHERE last_seen >= ?
       GROUP BY country ORDER BY n DESC`
    )
    .bind(cutoff)
    .all<{ country: string; n: number }>();

  const byOs = await db
    .prepare(
      `SELECT os, COUNT(*) AS n
       FROM instances WHERE last_seen >= ?
       GROUP BY os ORDER BY n DESC`
    )
    .bind(cutoff)
    .all<{ os: string; n: number }>();

  const byVersion = await db
    .prepare(
      `SELECT version, COUNT(*) AS n
       FROM instances WHERE last_seen >= ?
       GROUP BY version ORDER BY n DESC`
    )
    .bind(cutoff)
    .all<{ version: string; n: number }>();

  const totals = await db
    .prepare(
      `SELECT
         COALESCE(SUM(total_saved_tokens), 0) AS total_saved,
         COALESCE(AVG(avg_savings_pct), 0)    AS avg_savings_pct
       FROM instances WHERE last_seen >= ?`
    )
    .bind(cutoff)
    .first<{ total_saved: number; avg_savings_pct: number }>();

  const topCommands = await db
    .prepare(
      `SELECT command,
              SUM(total_count) AS count,
              SUM(total_saved) AS saved_tokens
       FROM command_stats
       WHERE date >= ?
       GROUP BY command
       ORDER BY saved_tokens DESC
       LIMIT 10`
    )
    .bind(since30dDate)
    .all<{ command: string; count: number; saved_tokens: number }>();

  // Daily savings series: cross-instance aggregate per UTC date. Window
  // last 30 days so /stats returns a stable shape regardless of how
  // long the system has been running.
  const daily = await db
    .prepare(
      `SELECT date,
              COUNT(DISTINCT installation_id) AS active_instances,
              SUM(commands)       AS commands,
              SUM(input_tokens)   AS input_tokens,
              SUM(output_tokens)  AS output_tokens,
              SUM(saved_tokens)   AS saved_tokens
       FROM instance_daily
       WHERE date >= ?
       GROUP BY date
       ORDER BY date ASC`
    )
    .bind(since30dDate)
    .all<{
      date: string;
      active_instances: number;
      commands: number;
      input_tokens: number;
      output_tokens: number;
      saved_tokens: number;
    }>();

  const response: StatsResponse = {
    active_instances: active?.n ?? 0,
    total_instances: total?.n ?? 0,
    inactive_threshold_days: INACTIVE_DAYS,
    by_country: Object.fromEntries((byCountry.results ?? []).map((r) => [r.country, r.n])),
    by_os: Object.fromEntries((byOs.results ?? []).map((r) => [r.os, r.n])),
    by_version: Object.fromEntries((byVersion.results ?? []).map((r) => [r.version, r.n])),
    total_saved_tokens: totals?.total_saved ?? 0,
    avg_savings_pct: Math.round((totals?.avg_savings_pct ?? 0) * 100) / 100,
    top_commands: (topCommands.results ?? []).map((r) => ({
      command: r.command,
      count: r.count,
      saved_tokens: r.saved_tokens,
    })),
    daily: (daily.results ?? []).map((r) => ({
      date: r.date,
      active_instances: r.active_instances,
      commands: r.commands,
      input_tokens: r.input_tokens,
      output_tokens: r.output_tokens,
      saved_tokens: r.saved_tokens,
      avg_savings_pct:
        r.input_tokens > 0
          ? Math.round((100 - (100 * r.output_tokens) / r.input_tokens) * 100) / 100
          : 0,
    })),
    generated_at: now,
  };

  await c.env.STATS_CACHE.put(CACHE_KEY, JSON.stringify(response), {
    expirationTtl: CACHE_TTL_SECONDS,
  });

  return c.json(response);
});
