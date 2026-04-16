import { z } from 'zod';

// Wire-format contract with apps/cli/src/engine/telemetry.rs::TelemetryPayload.
// `top_commands` is an extension consumed by the API for the "most prominent
// commands" stat; the existing CLI fields (installation_id, version, os,
// total_commands, total_saved_tokens, avg_savings_pct) remain unchanged.

export const TopCommand = z.object({
  command: z.string().min(1).max(100),
  count: z.number().int().nonnegative(),
  saved_tokens: z.number().int().nonnegative(),
});
export type TopCommand = z.infer<typeof TopCommand>;

// Today's slice as seen by the instance at ping time. The API uses this
// to derive cross-instance daily totals via REPLACE-on-conflict per
// (installation_id, date). If absent (older CLI versions), the daily
// rollup just won't include this instance for the day.
export const TodaySnapshot = z.object({
  date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/, 'date must be YYYY-MM-DD'),
  commands: z.number().int().nonnegative(),
  input_tokens: z.number().int().nonnegative(),
  output_tokens: z.number().int().nonnegative(),
  saved_tokens: z.number().int().nonnegative(),
});
export type TodaySnapshot = z.infer<typeof TodaySnapshot>;

export const TelemetryPayload = z.object({
  installation_id: z
    .string()
    .regex(/^[0-9a-f]{64}$/, 'installation_id must be a 64-char SHA-256 hex'),
  version: z.string().min(1).max(40),
  os: z.string().min(1).max(40),
  total_commands: z.number().int().nonnegative(),
  total_saved_tokens: z.number().int().nonnegative(),
  avg_savings_pct: z.number().min(0).max(100),
  top_commands: z.array(TopCommand).max(50).default([]),
  today: TodaySnapshot.optional(),
});
export type TelemetryPayload = z.infer<typeof TelemetryPayload>;

// Active = last_seen within INACTIVE_DAYS days.
export const INACTIVE_DAYS = 10;

export type DailyEntry = {
  date: string;
  active_instances: number;
  commands: number;
  input_tokens: number;
  output_tokens: number;
  saved_tokens: number;
  avg_savings_pct: number;
};

export type StatsResponse = {
  active_instances: number;
  total_instances: number;
  inactive_threshold_days: number;
  by_country: Record<string, number>;
  by_os: Record<string, number>;
  by_version: Record<string, number>;
  total_saved_tokens: number;
  avg_savings_pct: number;
  top_commands: { command: string; count: number; saved_tokens: number }[];
  daily: DailyEntry[];
  generated_at: number;
};

export type Bindings = {
  DB: D1Database;
  STATS_CACHE: KVNamespace;
};
