-- 0002_daily_savings.sql — per-instance per-day savings rollup.
--
-- Each /telemetry call REPLACEs (installation_id, date) with the latest
-- values reported by the CLI for "today". This handles the case where an
-- instance pings twice in one day: latest values win, so we don't double
-- count. Aggregating across instances at query time gives the daily total.
--
-- Anonymization: still no IP, still no PII. installation_id is the
-- client-side SHA-256 already used in `instances`. `date` is UTC.

CREATE TABLE instance_daily (
  installation_id TEXT NOT NULL,
  date TEXT NOT NULL,                    -- "YYYY-MM-DD" UTC
  commands INTEGER NOT NULL DEFAULT 0,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  saved_tokens INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (installation_id, date)
);

CREATE INDEX instance_daily_date_idx ON instance_daily(date);
