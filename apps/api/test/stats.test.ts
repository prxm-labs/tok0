import { describe, it, expect, beforeEach } from 'vitest';
import { env } from 'cloudflare:test';
import { app } from '@/index';

async function clear() {
  await env.DB.prepare('DELETE FROM instance_daily').run();
  await env.DB.prepare('DELETE FROM command_stats').run();
  await env.DB.prepare('DELETE FROM instances').run();
  const { keys } = await env.STATS_CACHE.list();
  await Promise.all(keys.map((k) => env.STATS_CACHE.delete(k.name)));
}

async function seedInstance(
  id: string,
  opts: {
    last_seen?: number;
    country?: string | null;
    os?: string;
    version?: string;
    saved?: number;
    avg?: number;
  } = {}
) {
  const now = Date.now();
  await env.DB.prepare(
    `INSERT INTO instances (
       installation_id, first_seen, last_seen, country, os, version,
       total_commands, total_saved_tokens, avg_savings_pct
     ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`
  )
    .bind(
      id,
      now - 1000,
      opts.last_seen ?? now,
      opts.country ?? null,
      opts.os ?? 'macos-aarch64',
      opts.version ?? '0.1.1',
      100,
      opts.saved ?? 1000,
      opts.avg ?? 80
    )
    .run();
}

async function seedCommandStat(date: string, command: string, count: number, saved: number) {
  await env.DB.prepare(
    `INSERT INTO command_stats (date, command, total_count, total_saved) VALUES (?, ?, ?, ?)`
  )
    .bind(date, command, count, saved)
    .run();
}

describe('GET /stats', () => {
  beforeEach(clear);

  it('returns zeros on an empty DB', async () => {
    const res = await app.request('/stats', {}, env);
    expect(res.status).toBe(200);
    const body = (await res.json()) as Record<string, unknown>;
    expect(body.active_instances).toBe(0);
    expect(body.total_instances).toBe(0);
    expect(body.inactive_threshold_days).toBe(10);
    expect(body.total_saved_tokens).toBe(0);
    expect(body.top_commands).toEqual([]);
  });

  it('counts active vs inactive (10-day cutoff)', async () => {
    const now = Date.now();
    const day = 86400_000;
    await seedInstance('a'.repeat(64), { last_seen: now });               // active
    await seedInstance('b'.repeat(64), { last_seen: now - 5 * day });      // active
    await seedInstance('c'.repeat(64), { last_seen: now - 11 * day });     // inactive
    await seedInstance('d'.repeat(64), { last_seen: now - 30 * day });     // inactive

    const res = await app.request('/stats', {}, env);
    const body = (await res.json()) as { active_instances: number; total_instances: number };
    expect(body.active_instances).toBe(2);
    expect(body.total_instances).toBe(4);
  });

  it('aggregates by country (active only)', async () => {
    const now = Date.now();
    await seedInstance('a'.repeat(64), { country: 'US', last_seen: now });
    await seedInstance('b'.repeat(64), { country: 'US', last_seen: now });
    await seedInstance('c'.repeat(64), { country: 'DE', last_seen: now });
    await seedInstance('d'.repeat(64), { country: 'JP', last_seen: now - 11 * 86400_000 }); // inactive

    const res = await app.request('/stats', {}, env);
    const body = (await res.json()) as { by_country: Record<string, number> };
    expect(body.by_country).toEqual({ US: 2, DE: 1 });
  });

  it('aggregates by os and version', async () => {
    const now = Date.now();
    await seedInstance('a'.repeat(64), { os: 'macos-aarch64', version: '0.1.1', last_seen: now });
    await seedInstance('b'.repeat(64), { os: 'linux-x86_64', version: '0.1.1', last_seen: now });
    await seedInstance('c'.repeat(64), { os: 'linux-x86_64', version: '0.2.0', last_seen: now });

    const res = await app.request('/stats', {}, env);
    const body = (await res.json()) as { by_os: Record<string, number>; by_version: Record<string, number> };
    expect(body.by_os).toEqual({ 'linux-x86_64': 2, 'macos-aarch64': 1 });
    expect(body.by_version).toEqual({ '0.1.1': 2, '0.2.0': 1 });
  });

  it('returns daily savings series aggregated across instances', async () => {
    const today = new Date().toISOString().slice(0, 10);
    const yest = new Date(Date.now() - 86400_000).toISOString().slice(0, 10);

    // Two instances reporting today: total saved = 1000+2000 = 3000
    await env.DB.prepare(
      `INSERT INTO instance_daily (installation_id, date, commands, input_tokens, output_tokens, saved_tokens)
       VALUES (?, ?, ?, ?, ?, ?)`
    ).bind('a'.repeat(64), today, 10, 2000, 1000, 1000).run();
    await env.DB.prepare(
      `INSERT INTO instance_daily (installation_id, date, commands, input_tokens, output_tokens, saved_tokens)
       VALUES (?, ?, ?, ?, ?, ?)`
    ).bind('b'.repeat(64), today, 20, 5000, 3000, 2000).run();
    // One instance yesterday
    await env.DB.prepare(
      `INSERT INTO instance_daily (installation_id, date, commands, input_tokens, output_tokens, saved_tokens)
       VALUES (?, ?, ?, ?, ?, ?)`
    ).bind('a'.repeat(64), yest, 5, 1000, 200, 800).run();

    const res = await app.request('/stats', {}, env);
    const body = (await res.json()) as { daily: { date: string; saved_tokens: number; active_instances: number; avg_savings_pct: number }[] };
    const todayEntry = body.daily.find((d) => d.date === today)!;
    const yestEntry = body.daily.find((d) => d.date === yest)!;
    expect(todayEntry.saved_tokens).toBe(3000);
    expect(todayEntry.active_instances).toBe(2);
    expect(yestEntry.saved_tokens).toBe(800);
    expect(yestEntry.active_instances).toBe(1);
    // savings_pct: today input=7000 output=4000 → (1 - 4/7)*100 = ~42.86
    expect(todayEntry.avg_savings_pct).toBeCloseTo(42.86, 1);
  });

  it('returns top commands sorted by saved_tokens (last 30d)', async () => {
    const today = new Date().toISOString().slice(0, 10);
    await seedCommandStat(today, 'git log', 100, 50_000);
    await seedCommandStat(today, 'cargo build', 50, 80_000);
    await seedCommandStat(today, 'npm install', 200, 10_000);

    const res = await app.request('/stats', {}, env);
    const body = (await res.json()) as { top_commands: { command: string; saved_tokens: number }[] };
    expect(body.top_commands.map((t) => t.command)).toEqual(['cargo build', 'git log', 'npm install']);
  });

  it('caches the response in KV (60s)', async () => {
    await seedInstance('a'.repeat(64), {});
    const r1 = await app.request('/stats', {}, env);
    expect(r1.status).toBe(200);

    // Mutate DB; if cache works, /stats should still report the cached snapshot.
    await env.DB.prepare('DELETE FROM instances').run();

    const r2 = await app.request('/stats', {}, env);
    const body = (await r2.json()) as { total_instances: number };
    expect(body.total_instances).toBe(1); // served from cache
  });

  it('CORS allows tok0.dev origin', async () => {
    const res = await app.request('/stats', {
      method: 'OPTIONS',
      headers: {
        Origin: 'https://tok0.dev',
        'Access-Control-Request-Method': 'GET',
      },
    }, env);
    expect(res.headers.get('Access-Control-Allow-Origin')).toBe('https://tok0.dev');
  });

  it('CORS rejects unknown origin', async () => {
    const res = await app.request('/stats', {
      method: 'OPTIONS',
      headers: {
        Origin: 'https://evil.example',
        'Access-Control-Request-Method': 'GET',
      },
    }, env);
    expect(res.headers.get('Access-Control-Allow-Origin')).toBeNull();
  });
});

describe('GET /healthz', () => {
  it('returns ok', async () => {
    const res = await app.request('/healthz', {}, env);
    expect(res.status).toBe(200);
    const body = (await res.json()) as { ok: boolean };
    expect(body.ok).toBe(true);
  });
});
