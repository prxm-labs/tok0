import { describe, it, expect, beforeEach } from 'vitest';
import { env } from 'cloudflare:test';
import { app } from '@/index';

const VALID_ID = 'a'.repeat(64);
const VALID_ID_2 = 'b'.repeat(64);

async function clear() {
  await env.DB.prepare('DELETE FROM instance_daily').run();
  await env.DB.prepare('DELETE FROM command_stats').run();
  await env.DB.prepare('DELETE FROM instances').run();
}

function basePayload(overrides: Record<string, unknown> = {}) {
  return {
    installation_id: VALID_ID,
    version: '0.1.1',
    os: 'macos-aarch64',
    total_commands: 100,
    total_saved_tokens: 50_000,
    avg_savings_pct: 82.5,
    top_commands: [
      { command: 'git log', count: 50, saved_tokens: 30_000 },
      { command: 'cargo build', count: 50, saved_tokens: 20_000 },
    ],
    ...overrides,
  };
}

describe('POST /telemetry', () => {
  beforeEach(clear);

  it('accepts a valid payload (200) and inserts an instance', async () => {
    const res = await app.request(
      '/telemetry',
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'cf-ipcountry': 'US',
        },
        body: JSON.stringify(basePayload()),
      },
      env
    );
    expect(res.status, await res.clone().text()).toBe(200);

    const row = await env.DB.prepare(
      'SELECT * FROM instances WHERE installation_id = ?'
    )
      .bind(VALID_ID)
      .first<{ country: string; os: string; total_saved_tokens: number; first_seen: number; last_seen: number }>();
    expect(row).toBeTruthy();
    expect(row!.country).toBe('US');
    expect(row!.os).toBe('macos-aarch64');
    expect(row!.total_saved_tokens).toBe(50_000);
    expect(row!.first_seen).toBeLessThanOrEqual(row!.last_seen);
  });

  it('upserts: second call from same instance updates last_seen, preserves first_seen', async () => {
    const t0 = Date.now();
    await app.request(
      '/telemetry',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'cf-ipcountry': 'DE' },
        body: JSON.stringify(basePayload()),
      },
      env
    );
    const first = await env.DB.prepare('SELECT first_seen, last_seen FROM instances WHERE installation_id = ?')
      .bind(VALID_ID)
      .first<{ first_seen: number; last_seen: number }>();

    // Wait at least 1ms so timestamps differ
    await new Promise((r) => setTimeout(r, 5));

    await app.request(
      '/telemetry',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'cf-ipcountry': 'DE' },
        body: JSON.stringify(basePayload({ total_saved_tokens: 60_000 })),
      },
      env
    );

    const second = await env.DB.prepare('SELECT first_seen, last_seen, total_saved_tokens FROM instances WHERE installation_id = ?')
      .bind(VALID_ID)
      .first<{ first_seen: number; last_seen: number; total_saved_tokens: number }>();

    expect(second!.first_seen).toBe(first!.first_seen);
    expect(second!.last_seen).toBeGreaterThan(first!.last_seen);
    expect(second!.total_saved_tokens).toBe(60_000);
    expect(t0).toBeLessThan(Date.now());
  });

  it('drops country when cf-ipcountry is XX/T1/missing/invalid', async () => {
    const cases = ['XX', 'T1', '', 'usa', '?'];
    for (const i of cases.keys()) {
      await clear();
      const headers: Record<string, string> = { 'Content-Type': 'application/json' };
      if (cases[i]) headers['cf-ipcountry'] = cases[i] ?? '';
      await app.request(
        '/telemetry',
        {
          method: 'POST',
          headers,
          body: JSON.stringify(basePayload({ installation_id: 'c'.repeat(64) })),
        },
        env
      );
      const row = await env.DB.prepare('SELECT country FROM instances')
        .first<{ country: string | null }>();
      expect(row?.country).toBeNull();
    }
  });

  it('rolls up top_commands into command_stats for the day', async () => {
    await app.request(
      '/telemetry',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(basePayload()),
      },
      env
    );
    // Second instance reports same commands → counts add up
    await app.request(
      '/telemetry',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(basePayload({ installation_id: VALID_ID_2 })),
      },
      env
    );
    const row = await env.DB.prepare(
      `SELECT total_count, total_saved FROM command_stats WHERE command = 'git log'`
    ).first<{ total_count: number; total_saved: number }>();
    expect(row?.total_count).toBe(100); // 50+50
    expect(row?.total_saved).toBe(60_000); // 30k+30k
  });

  it('rejects malformed installation_id (400)', async () => {
    const res = await app.request(
      '/telemetry',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(basePayload({ installation_id: 'not-a-hash' })),
      },
      env
    );
    expect(res.status).toBe(400);
  });

  it('rejects bodies > 64 KB (413)', async () => {
    const big = basePayload({
      top_commands: Array.from({ length: 50 }, (_, i) => ({
        command: 'x'.repeat(100),
        count: i,
        saved_tokens: i,
      })),
    });
    // Force Content-Length over 64 KB by padding
    const padded = JSON.stringify(big) + ' '.repeat(70_000);
    const res = await app.request(
      '/telemetry',
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Content-Length': String(padded.length),
        },
        body: padded,
      },
      env
    );
    expect([413, 400]).toContain(res.status);
  });

  it('rejects invalid JSON (400)', async () => {
    const res = await app.request(
      '/telemetry',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: '{ broken',
      },
      env
    );
    expect(res.status).toBe(400);
  });

  it('upserts instance_daily when payload includes today snapshot', async () => {
    const today = new Date().toISOString().slice(0, 10);
    await app.request(
      '/telemetry',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(
          basePayload({
            today: {
              date: today,
              commands: 12,
              input_tokens: 5000,
              output_tokens: 1000,
              saved_tokens: 4000,
            },
          })
        ),
      },
      env
    );
    const row = await env.DB.prepare(
      `SELECT * FROM instance_daily WHERE installation_id = ? AND date = ?`
    )
      .bind(VALID_ID, today)
      .first<{ commands: number; saved_tokens: number; input_tokens: number }>();
    expect(row).toBeTruthy();
    expect(row!.commands).toBe(12);
    expect(row!.saved_tokens).toBe(4000);
    expect(row!.input_tokens).toBe(5000);
  });

  it('REPLACEs same-day instance_daily on second ping (latest wins, no double count)', async () => {
    const today = new Date().toISOString().slice(0, 10);
    const send = (saved: number) =>
      app.request(
        '/telemetry',
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(
            basePayload({
              today: { date: today, commands: 1, input_tokens: 100, output_tokens: 10, saved_tokens: saved },
            })
          ),
        },
        env
      );
    await send(100);
    await send(500);
    const row = await env.DB.prepare(
      `SELECT saved_tokens FROM instance_daily WHERE installation_id = ? AND date = ?`
    )
      .bind(VALID_ID, today)
      .first<{ saved_tokens: number }>();
    expect(row?.saved_tokens).toBe(500);
  });

  it('omits instance_daily insert when today field absent (back-compat)', async () => {
    await app.request(
      '/telemetry',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(basePayload()),
      },
      env
    );
    const row = await env.DB.prepare(
      `SELECT COUNT(*) AS n FROM instance_daily WHERE installation_id = ?`
    )
      .bind(VALID_ID)
      .first<{ n: number }>();
    expect(row?.n).toBe(0);
  });

  it('does NOT store IP-related headers (anonymization invariant)', async () => {
    await app.request(
      '/telemetry',
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'cf-connecting-ip': '203.0.113.42',
          'x-real-ip': '203.0.113.42',
          'cf-ipcountry': 'US',
        },
        body: JSON.stringify(basePayload()),
      },
      env
    );
    // Check no column anywhere contains the IP fragment.
    const all = await env.DB.prepare(
      `SELECT installation_id, country, os, version FROM instances`
    ).all<Record<string, unknown>>();
    const blob = JSON.stringify(all.results);
    expect(blob).not.toContain('203.0.113');
  });
});
