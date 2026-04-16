-- 0001_init.sql — anonymous telemetry schema for tok0 cloud.
--
-- Anonymization rules (NON-NEGOTIABLE):
--   - No IP addresses are stored. Country comes from Cloudflare's
--     `cf-ipcountry` request header (computed at edge, never logged).
--   - `installation_id` is a SHA-256 hex digest computed client-side
--     by the CLI from (USER + config_dir). It is deterministic per
--     machine but contains no user-identifiable bytes.
--   - Sanitized command names only (e.g., "git log") — never argv,
--     paths, env, or hostnames. Sanitization happens client-side in
--     `engine::cloud::sanitize_command`.
--   - This database has NO `users`, `tokens`, or `teams` tables. It
--     is anonymous-only by construction. A future PR can add an
--     authenticated layer for team dashboards on a separate schema.

CREATE TABLE instances (
  installation_id TEXT PRIMARY KEY,
  first_seen INTEGER NOT NULL,           -- unix ms, server time
  last_seen INTEGER NOT NULL,            -- unix ms, server time
  country TEXT,                          -- ISO 3166-1 alpha-2 from cf-ipcountry, nullable
  os TEXT NOT NULL,                      -- "macos-aarch64", "linux-x86_64", etc.
  version TEXT NOT NULL,                 -- "0.1.1"
  total_commands INTEGER NOT NULL DEFAULT 0,
  total_saved_tokens INTEGER NOT NULL DEFAULT 0,
  avg_savings_pct REAL NOT NULL DEFAULT 0
);

CREATE INDEX instances_last_seen_idx ON instances(last_seen);
CREATE INDEX instances_country_idx ON instances(country);

-- Per-day, per-command rollup. Each /telemetry call adds the instance's
-- locally-tracked command counts into today's row. This is "report-batched"
-- aggregation: an instance reports a snapshot of its cumulative top
-- commands; we add the deltas (handled in code via UPSERT below). The
-- simpler approach we use for the MVP: each call is treated as a delta
-- contribution and we sum across calls. This over-counts if an instance
-- pings twice in a day, mitigated by the 24h cadence in the CLI.
CREATE TABLE command_stats (
  date TEXT NOT NULL,                    -- "YYYY-MM-DD" UTC
  command TEXT NOT NULL,                 -- sanitized
  total_count INTEGER NOT NULL DEFAULT 0,
  total_saved INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (date, command)
);

CREATE INDEX command_stats_date_idx ON command_stats(date);
