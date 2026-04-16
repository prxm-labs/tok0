import { Hono } from 'hono';
import { sql } from 'drizzle-orm';
import { TelemetryPayload, type Bindings } from '@/types';
import { getDb } from '@/db/client';

export const telemetryRoute = new Hono<{ Bindings: Bindings }>();

const MAX_BODY_BYTES = 64 * 1024;

// ISO 3166-1 alpha-2 sanity check. cf-ipcountry sends "XX" for unknown,
// "T1" for Tor exits. We accept any 2-letter uppercase code; anything
// else is dropped to NULL.
const COUNTRY_RE = /^[A-Z0-9]{2}$/;

function todayUtc(now: number): string {
  const d = new Date(now);
  const yyyy = d.getUTCFullYear();
  const mm = String(d.getUTCMonth() + 1).padStart(2, '0');
  const dd = String(d.getUTCDate()).padStart(2, '0');
  return `${yyyy}-${mm}-${dd}`;
}

telemetryRoute.post('/', async (c) => {
  const cl = Number(c.req.header('Content-Length') ?? '0');
  if (cl > MAX_BODY_BYTES) return c.json({ error: 'payload too large' }, 413);

  let parsed: unknown;
  try {
    parsed = await c.req.json();
  } catch {
    return c.json({ error: 'invalid json' }, 400);
  }

  const result = TelemetryPayload.safeParse(parsed);
  if (!result.success) {
    return c.json({ error: 'invalid body', issues: result.error.issues }, 400);
  }
  const p = result.data;

  // Country comes ONLY from Cloudflare's edge header. We never read or
  // store the client IP. cf-ipcountry is set to "XX" for unknown and may
  // be missing in local/test environments — store NULL in those cases.
  const cfCountry = c.req.header('cf-ipcountry') ?? '';
  const country =
    cfCountry && COUNTRY_RE.test(cfCountry) && cfCountry !== 'XX' && cfCountry !== 'T1'
      ? cfCountry
      : null;

  const now = Date.now();
  const today = todayUtc(now);
  const db = getDb(c.env.DB);

  // Upsert the instance. first_seen is preserved on update.
  await db.run(sql`
    INSERT INTO instances (
      installation_id, first_seen, last_seen, country, os, version,
      total_commands, total_saved_tokens, avg_savings_pct
    )
    VALUES (
      ${p.installation_id}, ${now}, ${now}, ${country}, ${p.os}, ${p.version},
      ${p.total_commands}, ${p.total_saved_tokens}, ${p.avg_savings_pct}
    )
    ON CONFLICT(installation_id) DO UPDATE SET
      last_seen = excluded.last_seen,
      country = COALESCE(excluded.country, instances.country),
      os = excluded.os,
      version = excluded.version,
      total_commands = excluded.total_commands,
      total_saved_tokens = excluded.total_saved_tokens,
      avg_savings_pct = excluded.avg_savings_pct
  `);

  // Roll up top commands into today's bucket. Each call adds the
  // instance's reported counts to the daily aggregate. The 24h CLI
  // cadence keeps double-reporting bounded.
  for (const tc of p.top_commands) {
    await db.run(sql`
      INSERT INTO command_stats (date, command, total_count, total_saved)
      VALUES (${today}, ${tc.command}, ${tc.count}, ${tc.saved_tokens})
      ON CONFLICT(date, command) DO UPDATE SET
        total_count = command_stats.total_count + excluded.total_count,
        total_saved = command_stats.total_saved + excluded.total_saved
    `);
  }

  // Per-instance per-day savings snapshot. REPLACE semantics: latest
  // values win. Aggregating across instances at /stats query time gives
  // the true daily total without double-counting same-day re-pings.
  if (p.today) {
    await db.run(sql`
      INSERT INTO instance_daily (
        installation_id, date, commands, input_tokens, output_tokens, saved_tokens
      )
      VALUES (
        ${p.installation_id}, ${p.today.date},
        ${p.today.commands}, ${p.today.input_tokens},
        ${p.today.output_tokens}, ${p.today.saved_tokens}
      )
      ON CONFLICT(installation_id, date) DO UPDATE SET
        commands = excluded.commands,
        input_tokens = excluded.input_tokens,
        output_tokens = excluded.output_tokens,
        saved_tokens = excluded.saved_tokens
    `);
  }

  return c.json({ ok: true });
});
