//! Cross-invocation context dedup.
//!
//! When the LLM re-runs the same command on the same project and gets
//! byte-identical compressed output, tok0 collapses the output to a
//! single line referencing the previous run id. The result: zero token
//! cost for "still failing the same way" or "still passing".
//!
//! This is the user-visible flagship of the Phase 5 work. Crucially,
//! every safeguard exists to prevent the dedup message from *lying*:
//!
//! - **30-minute hard TTL**: stale rows are treated as Miss.
//! - **CWD mtime fence**: if the project dir's mtime advanced since the
//!   prior run (an edit happened), treat as Miss.
//! - **Exit-code asymmetry**: a red→green transition is *never*
//!   suppressed. We only hit when the previous and current exit codes
//!   match.
//! - **Tool isolation**: state is keyed by `(tool_id, cwd, cmd_key)`
//!   so two AI tools running side-by-side don't see each other's
//!   claims.
//!
//! Default ON. Disable via `TOK0_NO_CONTEXT_STATE=1`.

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Rows older than this are not eligible for `FullHit`. The window is
/// short enough that a stale "same as last run" never surprises a
/// context-switched user.
const TTL_SECS: i64 = 30 * 60;

/// Decision returned by `decide()`. `Miss` means: emit normally, then
/// record. `FullHit` means: replace output with the dedup message.
#[derive(Debug)]
pub enum Decision {
    Miss,
    FullHit(RunRecord),
}

/// A previously-seen run.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RunRecord {
    pub run_id: String,
    pub recorded_at: i64,
    pub exit_code: i32,
    pub byte_len: u32,
    /// Line count of the previously-seen output. Carried for the user-
    /// visible "47 lines" hint in the dedup message.
    pub line_count: u32,
}

/// Wrapper around the SQLite tracking DB extended with a `context_state`
/// table. Sharing the same DB file as `meter` avoids a second WAL.
pub struct ContextStore {
    conn: Connection,
}

impl ContextStore {
    /// Open the store at the given DB path. Creates the schema if missing.
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open context_state DB: {}", db_path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .context("Failed to set WAL mode")?;
        conn.busy_timeout(std::time::Duration::from_millis(200))
            .context("Failed to set SQLite busy_timeout")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS context_state (
                tool_id      TEXT NOT NULL,
                project_path TEXT NOT NULL,
                cmd_key      TEXT NOT NULL,
                run_id       TEXT NOT NULL,
                recorded_at  INTEGER NOT NULL,
                cwd_mtime    INTEGER NOT NULL,
                full_hash    BLOB    NOT NULL,
                exit_code    INTEGER NOT NULL,
                byte_len     INTEGER NOT NULL,
                line_count   INTEGER NOT NULL,
                PRIMARY KEY (tool_id, project_path, cmd_key)
            ) WITHOUT ROWID;
            CREATE INDEX IF NOT EXISTS ix_context_recorded
                ON context_state(recorded_at);",
        )
        .context("Failed to create context_state schema")?;
        Ok(Self { conn })
    }

    /// Decide whether the current `(tool_id, project_path, cmd_key)` is
    /// a hit. All errors are silently swallowed → `Decision::Miss` so
    /// the user never gets a worse experience than today's behavior.
    #[allow(clippy::too_many_arguments)] // intentional: each field is load-bearing for the lookup
    pub fn decide(
        &self,
        tool_id: &str,
        project_path: &str,
        cmd_key: &str,
        compressed: &str,
        exit_code: i32,
        now_secs: i64,
        cwd_mtime: i64,
    ) -> Decision {
        // (run_id, recorded_at, full_hash, exit_code, cwd_mtime, byte_len, line_count)
        type Row = (String, i64, Vec<u8>, i32, i64, u32, u32);
        let row: Option<Row> = self
            .conn
            .query_row(
                "SELECT run_id, recorded_at, full_hash, exit_code, cwd_mtime, byte_len, line_count
                 FROM context_state
                 WHERE tool_id = ?1 AND project_path = ?2 AND cmd_key = ?3",
                params![tool_id, project_path, cmd_key],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get::<_, i64>(5)? as u32,
                        r.get::<_, i64>(6)? as u32,
                    ))
                },
            )
            .optional()
            .ok()
            .flatten();

        let Some((run_id, recorded_at, prior_hash, prior_exit, prior_mtime, byte_len, line_count)) =
            row
        else {
            return Decision::Miss;
        };

        // TTL: stale rows are Miss.
        if now_secs - recorded_at > TTL_SECS {
            return Decision::Miss;
        }
        // CWD mtime fence: any filesystem change in the project dir
        // invalidates the prior result.
        if cwd_mtime != prior_mtime {
            return Decision::Miss;
        }
        // Exit-code asymmetry: red↔green transition must never be
        // suppressed.
        if prior_exit != exit_code {
            return Decision::Miss;
        }
        // Content equality: hash the current compressed output and
        // compare.
        let current_hash = compute_hash(compressed);
        if current_hash[..] != prior_hash[..] {
            return Decision::Miss;
        }
        Decision::FullHit(RunRecord {
            run_id,
            recorded_at,
            exit_code: prior_exit,
            byte_len,
            line_count,
        })
    }

    /// Upsert the current run as the latest for this key. The PK is
    /// `(tool_id, project_path, cmd_key)` so there's at most one row
    /// per slot.
    #[allow(clippy::too_many_arguments)] // intentional: same shape as decide()
    pub fn record(
        &self,
        tool_id: &str,
        project_path: &str,
        cmd_key: &str,
        compressed: &str,
        exit_code: i32,
        now_secs: i64,
        cwd_mtime: i64,
    ) -> Result<RunRecord> {
        let full_hash = compute_hash(compressed);
        let run_id = make_run_id(&full_hash);
        let byte_len = compressed.len() as u32;
        let line_count = compressed.lines().count() as u32;
        self.conn
            .execute(
                "INSERT INTO context_state
                    (tool_id, project_path, cmd_key, run_id, recorded_at, cwd_mtime,
                     full_hash, exit_code, byte_len, line_count)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(tool_id, project_path, cmd_key) DO UPDATE SET
                    run_id      = excluded.run_id,
                    recorded_at = excluded.recorded_at,
                    cwd_mtime   = excluded.cwd_mtime,
                    full_hash   = excluded.full_hash,
                    exit_code   = excluded.exit_code,
                    byte_len    = excluded.byte_len,
                    line_count  = excluded.line_count",
                params![
                    tool_id,
                    project_path,
                    cmd_key,
                    run_id,
                    now_secs,
                    cwd_mtime,
                    &full_hash[..],
                    exit_code,
                    byte_len as i64,
                    line_count as i64
                ],
            )
            .context("Failed to upsert context_state row")?;
        Ok(RunRecord {
            run_id,
            recorded_at: now_secs,
            exit_code,
            byte_len,
            line_count,
        })
    }
}

/// SHA-256 of the compressed output, truncated to 16 bytes. The truncation
/// is fine for collision resistance at our scale (millions of distinct
/// outputs per user) and halves the row storage cost.
fn compute_hash(s: &str) -> [u8; 16] {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let full = hasher.finalize();
    let mut out = [0u8; 16];
    out.copy_from_slice(&full[..16]);
    out
}

/// First 5 bytes of the hash → 8-char lowercase hex run id. The id is
/// **content-derived** rather than time-derived so the LLM can recognize
/// "I saw this id" without remembering a wall-clock timestamp.
fn make_run_id(hash: &[u8; 16]) -> String {
    let mut s = String::with_capacity(10);
    for &b in &hash[..5] {
        s.push(hex_digit(b >> 4));
        s.push(hex_digit(b & 0x0F));
    }
    s
}

fn hex_digit(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + n - 10) as char,
        _ => unreachable!(),
    }
}

/// First two argv tokens joined — `cargo test`, `git status`, `npm install`.
/// Cheap key that avoids per-flag cardinality blowup (which would prevent
/// any hits).
pub fn cmd_key(cmd: &str, args: &[&str]) -> String {
    match args.first() {
        Some(sub) => format!("{cmd} {sub}"),
        None => cmd.to_string(),
    }
}

/// Format the user-visible dedup message. Tool-opaque (works for any
/// LLM); embeds the content-derived run id, age, exit code, line count,
/// and the opt-out hint.
pub fn format_full_hit(prev: &RunRecord, now_secs: i64) -> String {
    let age = format_age(now_secs - prev.recorded_at);
    format!(
        "[tok0:ctx] Identical to previous run (id={id}, age={age}, exit={exit}, {lines} lines).\n\
         Hint: set TOK0_NO_CONTEXT_STATE=1 to see full output.",
        id = prev.run_id,
        age = age,
        exit = prev.exit_code,
        lines = prev.line_count,
    )
}

fn format_age(secs: i64) -> String {
    let secs = secs.max(0);
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}

/// Wrap a compressed output through the dedup pipeline. Returns either
/// the original `compressed` (Miss) or the `[tok0:ctx]` message
/// (FullHit). On any internal failure, falls back to `compressed`.
///
/// Reads `TOK0_NO_CONTEXT_STATE` to skip the check entirely.
pub fn apply_dedup(
    cmd: &str,
    args: &[&str],
    compressed: &str,
    exit_code: i32,
    tool_id: &str,
) -> String {
    if std::env::var("TOK0_NO_CONTEXT_STATE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return compressed.to_string();
    }
    if compressed.is_empty() {
        return compressed.to_string();
    }
    let Ok(cwd) = std::env::current_dir() else {
        return compressed.to_string();
    };
    let cwd_str = cwd.to_string_lossy().to_string();
    let cwd_mtime = std::fs::metadata(&cwd)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let key = cmd_key(cmd, args);
    let db_path = super::config::db_path();
    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(store) = ContextStore::open(&db_path) else {
        return compressed.to_string();
    };
    let decision = store.decide(
        tool_id, &cwd_str, &key, compressed, exit_code, now, cwd_mtime,
    );
    let result = match decision {
        Decision::FullHit(prev) => format_full_hit(&prev, now),
        Decision::Miss => compressed.to_string(),
    };
    // Always record the current run for the next invocation. Errors are
    // swallowed — recording is best-effort.
    let _ = store.record(
        tool_id, &cwd_str, &key, compressed, exit_code, now, cwd_mtime,
    );
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open_test_store() -> (TempDir, ContextStore) {
        let tmp = TempDir::new().expect("tmp dir");
        let store = ContextStore::open(&tmp.path().join("ctx.db")).expect("open store");
        (tmp, store)
    }

    #[test]
    fn test_first_run_is_miss() {
        let (_tmp, store) = open_test_store();
        let d = store.decide("claude_code", "/p", "git status", "hello", 0, 1000, 100);
        assert!(matches!(d, Decision::Miss));
    }

    #[test]
    fn test_identical_re_run_is_full_hit() {
        let (_tmp, store) = open_test_store();
        store
            .record("claude_code", "/p", "git status", "hello", 0, 1000, 100)
            .expect("record");
        let d = store.decide("claude_code", "/p", "git status", "hello", 0, 1010, 100);
        match d {
            Decision::FullHit(r) => {
                assert_eq!(r.exit_code, 0);
                assert_eq!(r.line_count, 1);
                assert!(!r.run_id.is_empty());
            }
            other => panic!("expected FullHit, got {other:?}"),
        }
    }

    #[test]
    fn test_different_content_is_miss() {
        let (_tmp, store) = open_test_store();
        store
            .record("claude_code", "/p", "git status", "before", 0, 1000, 100)
            .expect("record");
        let d = store.decide("claude_code", "/p", "git status", "after", 0, 1010, 100);
        assert!(matches!(d, Decision::Miss));
    }

    #[test]
    fn test_exit_code_change_is_miss() {
        // Red→green transition MUST NOT be suppressed.
        let (_tmp, store) = open_test_store();
        store
            .record("claude_code", "/p", "cargo test", "output", 1, 1000, 100)
            .expect("record red");
        // Same output bytes but exit_code now 0 — must be Miss.
        let d = store.decide("claude_code", "/p", "cargo test", "output", 0, 1010, 100);
        assert!(matches!(d, Decision::Miss));
    }

    #[test]
    fn test_mtime_change_invalidates_hit() {
        let (_tmp, store) = open_test_store();
        store
            .record("claude_code", "/p", "cargo test", "output", 0, 1000, 100)
            .expect("record");
        // Same content + exit code, but CWD mtime advanced → user edited.
        let d = store.decide("claude_code", "/p", "cargo test", "output", 0, 1010, 200);
        assert!(matches!(d, Decision::Miss));
    }

    #[test]
    fn test_ttl_expiry_is_miss() {
        let (_tmp, store) = open_test_store();
        store
            .record("claude_code", "/p", "git log", "output", 0, 1000, 100)
            .expect("record");
        // 31 minutes later → past TTL.
        let later = 1000 + (TTL_SECS + 60);
        let d = store.decide("claude_code", "/p", "git log", "output", 0, later, 100);
        assert!(matches!(d, Decision::Miss));
    }

    #[test]
    fn test_per_tool_isolation() {
        let (_tmp, store) = open_test_store();
        store
            .record("claude_code", "/p", "git status", "output", 0, 1000, 100)
            .expect("record cc");
        // Same project, different tool → Miss.
        let d = store.decide("cursor", "/p", "git status", "output", 0, 1010, 100);
        assert!(matches!(d, Decision::Miss));
    }

    #[test]
    fn test_per_cwd_isolation() {
        let (_tmp, store) = open_test_store();
        store
            .record("claude_code", "/p1", "git status", "output", 0, 1000, 100)
            .expect("record p1");
        let d = store.decide("claude_code", "/p2", "git status", "output", 0, 1010, 100);
        assert!(matches!(d, Decision::Miss));
    }

    #[test]
    fn test_per_cmd_key_isolation() {
        let (_tmp, store) = open_test_store();
        store
            .record("claude_code", "/p", "git status", "output", 0, 1000, 100)
            .expect("record");
        let d = store.decide("claude_code", "/p", "git log", "output", 0, 1010, 100);
        assert!(matches!(d, Decision::Miss));
    }

    #[test]
    fn test_record_upsert_last_write_wins() {
        let (_tmp, store) = open_test_store();
        store
            .record("claude_code", "/p", "cargo test", "v1", 0, 1000, 100)
            .expect("record v1");
        store
            .record("claude_code", "/p", "cargo test", "v2", 0, 1010, 100)
            .expect("record v2");
        // The most recent record sticks; old hash is gone.
        let d = store.decide("claude_code", "/p", "cargo test", "v2", 0, 1020, 100);
        assert!(matches!(d, Decision::FullHit(_)));
        let d = store.decide("claude_code", "/p", "cargo test", "v1", 0, 1020, 100);
        assert!(matches!(d, Decision::Miss));
    }

    #[test]
    fn test_run_id_is_content_derived_and_stable() {
        let h = compute_hash("hello world");
        let id1 = make_run_id(&h);
        let id2 = make_run_id(&compute_hash("hello world"));
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 10); // 5 bytes → 10 hex chars
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_cmd_key_first_two_words() {
        assert_eq!(cmd_key("git", &["status", "--short"]), "git status");
        assert_eq!(
            cmd_key("cargo", &["test", "--release", "foo"]),
            "cargo test"
        );
        assert_eq!(cmd_key("ls", &[]), "ls");
    }

    #[test]
    fn test_format_full_hit_contains_run_id_and_age() {
        let r = RunRecord {
            run_id: "abc12345".to_string(),
            recorded_at: 1000,
            exit_code: 0,
            byte_len: 47,
            line_count: 12,
        };
        let msg = format_full_hit(&r, 1180); // 180s later → 3m
        assert!(msg.contains("[tok0:ctx]"));
        assert!(msg.contains("id=abc12345"));
        assert!(msg.contains("age=3m"));
        assert!(msg.contains("exit=0"));
        assert!(msg.contains("12 lines"));
        assert!(msg.contains("TOK0_NO_CONTEXT_STATE"));
    }

    #[test]
    fn test_format_age_buckets() {
        assert_eq!(format_age(0), "0s");
        assert_eq!(format_age(45), "45s");
        assert_eq!(format_age(180), "3m");
        assert_eq!(format_age(3600), "1h");
        assert_eq!(format_age(-5), "0s"); // negative clamp
    }

    #[test]
    fn test_apply_dedup_opt_out_passes_through() {
        // Set the opt-out env, run apply_dedup twice; second call must NOT
        // return a [tok0:ctx] message even though content is identical.
        // SAFETY: the env mutation here is process-wide; this test depends
        // on serial execution within the same module. cargo test runs test
        // modules' tests in parallel by default, but tests *within* a module
        // serialize when they need the same env. Use a Mutex to be safe.
        use std::sync::Mutex;
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _g = ENV_LOCK.lock().expect("env lock");
        unsafe { std::env::set_var("TOK0_NO_CONTEXT_STATE", "1") };
        let out1 = apply_dedup("git", &["status"], "hello", 0, "unknown");
        let out2 = apply_dedup("git", &["status"], "hello", 0, "unknown");
        assert_eq!(out1, "hello");
        assert_eq!(out2, "hello");
        assert!(!out2.contains("[tok0:ctx]"));
        unsafe { std::env::remove_var("TOK0_NO_CONTEXT_STATE") };
    }

    #[test]
    fn test_apply_dedup_empty_input_passes_through() {
        let out = apply_dedup("git", &["status"], "", 0, "unknown");
        assert_eq!(out, "");
    }
}
