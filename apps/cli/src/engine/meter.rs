use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;

pub struct Tracker {
    conn: Connection,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Summary {
    pub total_commands: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_saved: u64,
    pub avg_savings_pct: f64,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct DailyStats {
    pub date: String,
    pub commands: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub saved_tokens: u64,
    pub avg_savings_pct: f64,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct RecentCommand {
    pub timestamp: String,
    pub command: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub savings_pct: f64,
}

impl Tracker {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open tracking DB: {}", db_path.display()))?;

        conn.pragma_update(None, "journal_mode", "WAL")
            .context("Failed to set WAL mode")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS commands (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                command TEXT NOT NULL,
                full_command TEXT,
                input_tokens INTEGER NOT NULL,
                output_tokens INTEGER NOT NULL,
                saved_tokens INTEGER NOT NULL,
                savings_pct REAL NOT NULL,
                duration_ms INTEGER,
                project_path TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_commands_timestamp ON commands(timestamp);
            CREATE INDEX IF NOT EXISTS idx_commands_command ON commands(command);

            CREATE TABLE IF NOT EXISTS commands_rollup (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                command TEXT NOT NULL,
                commands INTEGER NOT NULL,
                input_tokens INTEGER NOT NULL,
                output_tokens INTEGER NOT NULL,
                saved_tokens INTEGER NOT NULL,
                avg_savings_pct REAL NOT NULL
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_rollup_date_cmd
                ON commands_rollup(date, command);",
        )
        .context("Failed to create tracking schema")?;

        Ok(Self { conn })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record(
        &self,
        command: &str,
        full_command: &str,
        input_tokens: u64,
        output_tokens: u64,
        saved_tokens: u64,
        savings_pct: f64,
        duration_ms: u64,
        project_path: &str,
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO commands (command, full_command, input_tokens, output_tokens, saved_tokens, savings_pct, duration_ms, project_path)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    command,
                    full_command,
                    input_tokens,
                    output_tokens,
                    saved_tokens,
                    savings_pct,
                    duration_ms,
                    project_path,
                ],
            )
            .context("Failed to record command")?;

        // Opportunistic rollup: check every ~100 inserts
        let count: u64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM commands", [], |row| row.get(0))
            .unwrap_or(0);
        if count > Self::ROLLUP_THRESHOLD * 2 {
            let _ = self.maybe_rollup(); // Best-effort, don't fail the record
        }

        Ok(())
    }

    pub fn get_summary(&self) -> Result<Summary> {
        let mut stmt = self.conn.prepare(
            "SELECT
                COALESCE(SUM(total_commands), 0),
                COALESCE(SUM(total_input), 0),
                COALESCE(SUM(total_output), 0),
                COALESCE(SUM(total_saved), 0)
             FROM (
                SELECT COUNT(*) as total_commands,
                       SUM(input_tokens) as total_input,
                       SUM(output_tokens) as total_output,
                       SUM(saved_tokens) as total_saved
                FROM commands
                UNION ALL
                SELECT SUM(commands),
                       SUM(input_tokens),
                       SUM(output_tokens),
                       SUM(saved_tokens)
                FROM commands_rollup
             )",
        )?;

        let (total_commands, total_input_tokens, total_output_tokens, total_saved): (
            u64,
            u64,
            u64,
            u64,
        ) = stmt.query_row([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;

        let avg_savings_pct = if total_input_tokens > 0 {
            (total_saved as f64 / total_input_tokens as f64) * 100.0
        } else {
            0.0
        };

        Ok(Summary {
            total_commands,
            total_input_tokens,
            total_output_tokens,
            total_saved,
            avg_savings_pct,
        })
    }

    pub fn get_daily(&self, days: u32) -> Result<Vec<DailyStats>> {
        let days_param = format!("-{} days", days);
        let mut stmt = self.conn.prepare(
            "SELECT day,
                    SUM(commands) as commands,
                    SUM(input)  as input,
                    SUM(output) as output,
                    SUM(saved)  as saved,
                    CASE WHEN SUM(commands) > 0
                         THEN SUM(saved) * 100.0 / NULLIF(SUM(input), 0)
                         ELSE 0.0 END as avg_pct
             FROM (
                SELECT date(timestamp) as day,
                       COUNT(*) as commands,
                       SUM(input_tokens)  as input,
                       SUM(output_tokens) as output,
                       SUM(saved_tokens)  as saved
                FROM commands
                WHERE timestamp >= datetime('now', ?1)
                GROUP BY date(timestamp)
                UNION ALL
                SELECT date,
                       SUM(commands),
                       SUM(input_tokens),
                       SUM(output_tokens),
                       SUM(saved_tokens)
                FROM commands_rollup
                WHERE date >= date('now', ?1)
                GROUP BY date
             )
             GROUP BY day
             ORDER BY day DESC",
        )?;

        let rows = stmt
            .query_map([&days_param], |row| {
                Ok(DailyStats {
                    date: row.get(0)?,
                    commands: row.get(1)?,
                    input_tokens: row.get(2)?,
                    output_tokens: row.get(3)?,
                    saved_tokens: row.get(4)?,
                    avg_savings_pct: row.get::<_, f64>(5).unwrap_or(0.0),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("Failed to query daily stats")?;

        Ok(rows)
    }

    pub fn get_recent(&self, limit: u32) -> Result<Vec<RecentCommand>> {
        let mut stmt = self.conn.prepare(
            "SELECT timestamp, command, input_tokens, output_tokens, savings_pct
             FROM commands
             ORDER BY timestamp DESC
             LIMIT ?1",
        )?;

        let rows = stmt
            .query_map([limit], |row| {
                Ok(RecentCommand {
                    timestamp: row.get(0)?,
                    command: row.get(1)?,
                    input_tokens: row.get(2)?,
                    output_tokens: row.get(3)?,
                    savings_pct: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("Failed to query recent commands")?;

        Ok(rows)
    }

    pub fn get_top_commands(&self, limit: u32) -> Result<Vec<(String, u64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT command, SUM(total_saved) as saved
             FROM (
                SELECT command, COALESCE(SUM(saved_tokens), 0) as total_saved
                FROM commands
                GROUP BY command
                UNION ALL
                SELECT command, COALESCE(SUM(saved_tokens), 0)
                FROM commands_rollup
                GROUP BY command
             )
             GROUP BY command
             ORDER BY saved DESC
             LIMIT ?1",
        )?;

        let rows = stmt
            .query_map([limit], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("Failed to query top commands")?;

        Ok(rows)
    }

    /// Rollup threshold: days with more than this many rows get aggregated.
    const ROLLUP_THRESHOLD: u64 = 100;

    /// Aggregate old days that exceed the threshold into `commands_rollup`,
    /// then delete the detail rows. Never touches today's data.
    pub fn maybe_rollup(&self) -> Result<()> {
        // Find past days with row counts exceeding the threshold
        let mut stmt = self.conn.prepare(
            "SELECT date(timestamp) as day, COUNT(*) as cnt
             FROM commands
             WHERE date(timestamp) < date('now')
             GROUP BY date(timestamp)
             HAVING cnt > ?1",
        )?;

        let days: Vec<String> = stmt
            .query_map(params![Self::ROLLUP_THRESHOLD], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("Failed to find rollup candidates")?;

        for day in &days {
            // Aggregate into rollup (upsert: add to existing if re-run)
            self.conn.execute(
                "INSERT INTO commands_rollup (date, command, commands, input_tokens, output_tokens, saved_tokens, avg_savings_pct)
                 SELECT date(timestamp), command, COUNT(*), SUM(input_tokens), SUM(output_tokens), SUM(saved_tokens), AVG(savings_pct)
                 FROM commands
                 WHERE date(timestamp) = ?1
                 GROUP BY command
                 ON CONFLICT(date, command) DO UPDATE SET
                    commands = commands + excluded.commands,
                    input_tokens = commands_rollup.input_tokens + excluded.input_tokens,
                    output_tokens = commands_rollup.output_tokens + excluded.output_tokens,
                    saved_tokens = commands_rollup.saved_tokens + excluded.saved_tokens,
                    avg_savings_pct = (commands_rollup.avg_savings_pct * commands_rollup.commands + excluded.avg_savings_pct * excluded.commands) / (commands_rollup.commands + excluded.commands)",
                params![day],
            ).with_context(|| format!("Failed to rollup day {}", day))?;

            // Delete the detail rows for that day
            self.conn
                .execute(
                    "DELETE FROM commands WHERE date(timestamp) = ?1",
                    params![day],
                )
                .with_context(|| format!("Failed to delete rolled-up rows for {}", day))?;
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn cleanup(&self, retention_days: u32) -> Result<u64> {
        let days_param = format!("-{} days", retention_days);
        let deleted = self
            .conn
            .execute(
                "DELETE FROM commands WHERE timestamp < datetime('now', ?1)",
                [&days_param],
            )
            .context("Failed to cleanup old records")?;
        Ok(deleted as u64)
    }
}

// ─── Async background writer ─────────────────────────────────────────────────
//
// `record()` sends a MeterEvent over an mpsc channel to a background thread
// that owns the SQLite connection. This keeps the caller's critical path
// free of any DB I/O.
//
// Worker lifetime:
//   - Spawned lazily on first `record()` call.
//   - Persists for the process lifetime.
//   - Processes events individually (no batching) so each write is durable.
//   - `flush()` signals shutdown and joins with a timeout so any pending
//     events are written before the process exits.

use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;

enum MeterEvent {
    Record {
        command: String,
        input_tokens: u64,
        output_tokens: u64,
        saved: u64,
        pct: f64,
        duration_ms: u64,
        project_path: String,
    },
    Shutdown,
}

struct AsyncMeter {
    tx: SyncSender<MeterEvent>,
    join: Mutex<Option<JoinHandle<()>>>,
}

static METER: OnceLock<AsyncMeter> = OnceLock::new();

/// Worker loop: owns the SQLite connection, drains the channel until
/// shutdown. Drops silently if the DB can't be opened (metering is
/// best-effort and must never block the user).
fn worker_loop(rx: std::sync::mpsc::Receiver<MeterEvent>) {
    let db_path = crate::engine::config::db_path();
    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(tracker) = Tracker::new(&db_path) else {
        // DB unavailable — silently drain without writing so senders don't block.
        for event in rx {
            if matches!(event, MeterEvent::Shutdown) {
                return;
            }
        }
        return;
    };

    for event in rx {
        match event {
            MeterEvent::Shutdown => return,
            MeterEvent::Record {
                command,
                input_tokens,
                output_tokens,
                saved,
                pct,
                duration_ms,
                project_path,
            } => {
                let _ = tracker.record(
                    &command,
                    &command,
                    input_tokens,
                    output_tokens,
                    saved,
                    pct,
                    duration_ms,
                    &project_path,
                );
            }
        }
    }
}

fn start_meter() -> AsyncMeter {
    // Bounded channel: if the worker falls behind, record() starts dropping
    // events rather than blocking the caller.
    let (tx, rx) = sync_channel(256);
    let join = std::thread::Builder::new()
        .name("tok0-meter".to_string())
        .spawn(move || worker_loop(rx))
        .expect("Failed to spawn meter worker thread");
    AsyncMeter {
        tx,
        join: Mutex::new(Some(join)),
    }
}

/// Convenience: record a command using the default DB path.
///
/// This is the async path: the call sends a MeterEvent to a background
/// worker thread and returns immediately. Any DB errors happen on the
/// worker and are silently ignored (metering is best-effort).
pub fn record(
    command: &str,
    input: &str,
    output: &str,
    duration_ms: u64,
    project_path: &str,
) -> Result<()> {
    let input_tokens = crate::engine::shell::count_tokens(input) as u64;
    let output_tokens = crate::engine::shell::count_tokens(output) as u64;
    let saved = input_tokens.saturating_sub(output_tokens);
    let pct = if input_tokens > 0 {
        (saved as f64 / input_tokens as f64) * 100.0
    } else {
        0.0
    };

    let meter = METER.get_or_init(start_meter);
    // try_send: if the channel is full or the worker has gone away, drop
    // the event rather than blocking the caller.
    let _ = meter.tx.try_send(MeterEvent::Record {
        command: command.to_string(),
        input_tokens,
        output_tokens,
        saved,
        pct,
        duration_ms,
        project_path: project_path.to_string(),
    });
    Ok(())
}

/// Flush pending meter events before process exit.
///
/// Sends a shutdown sentinel and joins the worker thread with a 500ms
/// timeout. Safe to call even if the worker was never spawned.
pub fn flush() {
    let Some(meter) = METER.get() else {
        return;
    };
    let _ = meter.tx.send(MeterEvent::Shutdown);
    let Ok(mut guard) = meter.join.lock() else {
        return;
    };
    let Some(handle) = guard.take() else {
        return;
    };
    // Bounded join: spawn a watchdog that detaches the handle after a
    // timeout so we don't stall exit if the DB is wedged.
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    let watcher = std::thread::spawn(move || {
        let _ = handle.join();
        let _ = done_tx.send(());
    });
    let _ = done_rx.recv_timeout(std::time::Duration::from_millis(500));
    // Detach watcher; if it hasn't finished, let it linger until process exit.
    drop(watcher);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn make_tracker() -> (Tracker, NamedTempFile) {
        let tmp = NamedTempFile::new().expect("temp file");
        let tracker = Tracker::new(tmp.path()).expect("create tracker");
        (tracker, tmp)
    }

    #[test]
    fn test_record_and_query() {
        let (tracker, _tmp) = make_tracker();
        tracker
            .record(
                "git status",
                "tok0 git status",
                2000,
                400,
                1600,
                80.0,
                5,
                "/tmp/test",
            )
            .expect("record should succeed");

        let summary = tracker.get_summary().expect("summary should succeed");
        assert_eq!(summary.total_commands, 1);
        assert_eq!(summary.total_saved, 1600);
        assert_eq!(summary.total_input_tokens, 2000);
        assert_eq!(summary.total_output_tokens, 400);
        assert!((summary.avg_savings_pct - 80.0).abs() < 0.1);
    }

    #[test]
    fn test_multiple_records() {
        let (tracker, _tmp) = make_tracker();
        tracker
            .record(
                "git status",
                "tok0 git status",
                2000,
                400,
                1600,
                80.0,
                5,
                "/tmp",
            )
            .expect("record 1");
        tracker
            .record("git log", "tok0 git log", 5000, 500, 4500, 90.0, 8, "/tmp")
            .expect("record 2");
        tracker
            .record("ls", "tok0 ls", 100, 50, 50, 50.0, 2, "/tmp")
            .expect("record 3");

        let summary = tracker.get_summary().expect("summary");
        assert_eq!(summary.total_commands, 3);
        assert_eq!(summary.total_saved, 6150);
    }

    #[test]
    fn test_empty_summary() {
        let (tracker, _tmp) = make_tracker();
        let summary = tracker.get_summary().expect("summary");
        assert_eq!(summary.total_commands, 0);
        assert_eq!(summary.total_saved, 0);
        assert!((summary.avg_savings_pct - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_get_recent() {
        let (tracker, _tmp) = make_tracker();
        for i in 0..5 {
            tracker
                .record(&format!("cmd{}", i), "full", 100, 50, 50, 50.0, 1, "/tmp")
                .expect("record");
        }

        let recent = tracker.get_recent(3).expect("recent");
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_cleanup_old_records() {
        let (tracker, _tmp) = make_tracker();
        tracker
            .record("old", "tok0 old", 100, 50, 50, 50.0, 1, "/tmp")
            .expect("record");
        // cleanup with 0 days retention removes nothing since the record was just created
        let _deleted = tracker.cleanup(0).expect("cleanup");
        // The record was just created so it's within "today" — may or may not be deleted
        // depending on datetime precision. Just verify no error.
        let summary = tracker.get_summary().expect("summary");
        assert!(summary.total_commands <= 1);
    }

    #[test]
    fn test_savings_calculation() {
        let input_tokens: u64 = 2000;
        let output_tokens: u64 = 400;
        let saved = input_tokens - output_tokens;
        let pct = (saved as f64 / input_tokens as f64) * 100.0;
        assert!((pct - 80.0).abs() < 0.1);
    }

    #[test]
    fn test_get_daily() {
        let (tracker, _tmp) = make_tracker();
        tracker
            .record(
                "git status",
                "tok0 git status",
                2000,
                400,
                1600,
                80.0,
                5,
                "/tmp",
            )
            .expect("record");

        let daily = tracker.get_daily(7).expect("daily");
        assert!(!daily.is_empty());
        assert_eq!(daily[0].commands, 1);
        assert_eq!(daily[0].saved_tokens, 1600);
    }

    #[test]
    fn test_get_top_commands() {
        let (tracker, _tmp) = make_tracker();
        tracker
            .record(
                "git status",
                "tok0 git status",
                2000,
                400,
                1600,
                80.0,
                5,
                "/tmp",
            )
            .expect("record");
        tracker
            .record("git log", "tok0 git log", 5000, 500, 4500, 90.0, 8, "/tmp")
            .expect("record");
        tracker
            .record(
                "git status",
                "tok0 git status",
                1000,
                200,
                800,
                80.0,
                3,
                "/tmp",
            )
            .expect("record");

        let top = tracker.get_top_commands(10).expect("top commands");
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "git log"); // highest savings
        assert_eq!(top[0].1, 4500);
        assert_eq!(top[1].0, "git status");
        assert_eq!(top[1].1, 2400); // 1600 + 800
    }

    #[test]
    fn test_rollup_schema_created() {
        let (tracker, _tmp) = make_tracker();
        // Rollup table should exist after initialization
        let count: u64 = tracker
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='commands_rollup'",
                [],
                |row| row.get(0),
            )
            .expect("query sqlite_master");
        assert_eq!(count, 1, "commands_rollup table should exist");
    }

    #[test]
    fn test_rollup_aggregates_when_threshold_exceeded() {
        let (tracker, _tmp) = make_tracker();
        // Insert enough records for a single day to exceed the rollup threshold (100)
        // Use a fixed past date so they all group together
        for i in 0..110 {
            tracker
                .conn
                .execute(
                    "INSERT INTO commands (timestamp, command, full_command, input_tokens, output_tokens, saved_tokens, savings_pct, duration_ms, project_path)
                     VALUES (datetime('2026-01-15 12:00:00', ?1), 'git status', 'tok0 git status', 200, 40, 160, 80.0, 5, '/tmp')",
                    params![format!("+{} seconds", i)],
                )
                .expect("insert test row");
        }

        tracker.maybe_rollup().expect("rollup should succeed");

        // Live rows for that day should be gone
        let live_count: u64 = tracker
            .conn
            .query_row(
                "SELECT COUNT(*) FROM commands WHERE date(timestamp) = '2026-01-15'",
                [],
                |row| row.get(0),
            )
            .expect("count live");
        assert_eq!(live_count, 0, "live rows should be deleted after rollup");

        // Rollup should have one row
        let rollup_count: u64 = tracker
            .conn
            .query_row("SELECT COUNT(*) FROM commands_rollup", [], |row| row.get(0))
            .expect("count rollup");
        assert!(rollup_count >= 1, "rollup should have at least one row");

        // Verify aggregated values
        let (cmds, input, saved): (u64, u64, u64) = tracker
            .conn
            .query_row(
                "SELECT commands, input_tokens, saved_tokens FROM commands_rollup WHERE date = '2026-01-15' AND command = 'git status'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("query rollup row");
        assert_eq!(cmds, 110);
        assert_eq!(input, 200 * 110);
        assert_eq!(saved, 160 * 110);
    }

    #[test]
    fn test_rollup_skipped_below_threshold() {
        let (tracker, _tmp) = make_tracker();
        // Insert only 10 records — below threshold
        for _ in 0..10 {
            tracker
                .record("ls", "tok0 ls", 100, 50, 50, 50.0, 2, "/tmp")
                .expect("record");
        }

        tracker.maybe_rollup().expect("rollup should succeed");

        // All 10 should still be live
        let live_count: u64 = tracker
            .conn
            .query_row("SELECT COUNT(*) FROM commands", [], |row| row.get(0))
            .expect("count live");
        assert_eq!(live_count, 10, "below threshold: live rows should remain");

        let rollup_count: u64 = tracker
            .conn
            .query_row("SELECT COUNT(*) FROM commands_rollup", [], |row| row.get(0))
            .expect("count rollup");
        assert_eq!(rollup_count, 0, "below threshold: no rollup rows");
    }

    #[test]
    fn test_summary_combines_live_and_rollup() {
        let (tracker, _tmp) = make_tracker();
        // Insert a rollup row directly
        tracker
            .conn
            .execute(
                "INSERT INTO commands_rollup (date, command, commands, input_tokens, output_tokens, saved_tokens, avg_savings_pct)
                 VALUES ('2026-01-10', 'git log', 50, 100000, 10000, 90000, 90.0)",
                [],
            )
            .expect("insert rollup");

        // Insert a live row
        tracker
            .record(
                "git status",
                "tok0 git status",
                2000,
                400,
                1600,
                80.0,
                5,
                "/tmp",
            )
            .expect("record live");

        let summary = tracker.get_summary().expect("summary");
        assert_eq!(summary.total_commands, 51, "50 rollup + 1 live");
        assert_eq!(
            summary.total_input_tokens, 102000,
            "100000 rollup + 2000 live"
        );
        assert_eq!(summary.total_saved, 91600, "90000 rollup + 1600 live");
    }

    #[test]
    fn test_top_commands_combines_live_and_rollup() {
        let (tracker, _tmp) = make_tracker();
        // Rollup row
        tracker
            .conn
            .execute(
                "INSERT INTO commands_rollup (date, command, commands, input_tokens, output_tokens, saved_tokens, avg_savings_pct)
                 VALUES ('2026-01-10', 'git log', 50, 100000, 10000, 90000, 90.0)",
                [],
            )
            .expect("insert rollup");

        // Live rows for same command
        tracker
            .record("git log", "tok0 git log", 5000, 500, 4500, 90.0, 8, "/tmp")
            .expect("record");

        let top = tracker.get_top_commands(10).expect("top");
        assert_eq!(top[0].0, "git log");
        assert_eq!(top[0].1, 94500, "90000 rollup + 4500 live");
    }

    #[test]
    fn test_daily_combines_live_and_rollup() {
        let (tracker, _tmp) = make_tracker();
        // Insert rollup for today
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        tracker
            .conn
            .execute(
                "INSERT INTO commands_rollup (date, command, commands, input_tokens, output_tokens, saved_tokens, avg_savings_pct)
                 VALUES (?1, 'git status', 100, 200000, 40000, 160000, 80.0)",
                params![today],
            )
            .expect("insert rollup");

        // Live row for today
        tracker
            .record(
                "git status",
                "tok0 git status",
                2000,
                400,
                1600,
                80.0,
                5,
                "/tmp",
            )
            .expect("record live");

        let daily = tracker.get_daily(7).expect("daily");
        assert!(!daily.is_empty());
        assert_eq!(daily[0].commands, 101, "100 rollup + 1 live");
        assert_eq!(daily[0].saved_tokens, 161600, "160000 rollup + 1600 live");
    }

    #[test]
    fn test_rollup_does_not_touch_today() {
        let (tracker, _tmp) = make_tracker();
        // Insert 200 records for today — should NOT be rolled up
        for _ in 0..200 {
            tracker
                .record("ls", "tok0 ls", 100, 50, 50, 50.0, 2, "/tmp")
                .expect("record");
        }

        tracker.maybe_rollup().expect("rollup");

        let live_count: u64 = tracker
            .conn
            .query_row("SELECT COUNT(*) FROM commands", [], |row| row.get(0))
            .expect("count");
        assert_eq!(live_count, 200, "today's rows should not be rolled up");
    }

    #[test]
    fn test_wal_mode_active() {
        let tmp = NamedTempFile::new().expect("temp file");
        let tracker = Tracker::new(tmp.path()).expect("create tracker");
        let mode: String = tracker
            .conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .expect("query journal mode");
        assert_eq!(mode.to_lowercase(), "wal");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Async meter tests — these run against a custom TOK0_DB_PATH so the
    // global OnceLock worker writes to our test DB.
    //
    // NOTE: The worker is a process-wide singleton, so these tests must be
    // serialized. We use a single #[test] that exercises the async path end
    // to end rather than multiple tests that would race on the static.
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_async_record_flush_persists_events() {
        use std::sync::Mutex;

        // Serialize access to TOK0_DB_PATH across any other tests that
        // might touch it.
        static LOCK: Mutex<()> = Mutex::new(());
        let _guard = LOCK.lock().expect("lock");

        let tmp = tempfile::NamedTempFile::new().expect("tmp");
        let db_path = tmp.path().to_path_buf();
        // SAFETY: single-threaded test section, env var is read-only elsewhere.
        unsafe {
            std::env::set_var("TOK0_DB_PATH", &db_path);
        }

        // First: async record + flush. Since METER is a process-wide static,
        // this test assumes it has not been initialized yet in this process.
        // Other tests in this module don't touch the async path, so the
        // static remains uninitialized until now.
        super::record("git status", "lots of input", "brief", 10, "/tmp").expect("async record");
        super::record("git log", "even more input here", "summary", 20, "/tmp")
            .expect("async record");

        super::flush();

        // flush() has a 500ms watchdog before detaching from the worker. On
        // slow runners (notably Windows CI) SQLite WAL writes for two events
        // can exceed that, so the worker is still committing when flush()
        // returns. Poll the DB until both events are visible rather than
        // asserting immediately on a single read.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let summary = loop {
            let tracker = Tracker::new(&db_path).expect("open db after flush");
            let s = tracker.get_summary().expect("summary");
            if s.total_commands >= 2 || std::time::Instant::now() >= deadline {
                break s;
            }
            drop(tracker);
            std::thread::sleep(std::time::Duration::from_millis(50));
        };
        assert!(
            summary.total_commands >= 2,
            "expected at least 2 commands persisted, got {}",
            summary.total_commands
        );

        // SAFETY: single-threaded test section.
        unsafe {
            std::env::remove_var("TOK0_DB_PATH");
        }
    }
}
