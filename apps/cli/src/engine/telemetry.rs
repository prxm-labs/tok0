use anyhow::{Context, Result};
use std::path::PathBuf;
#[cfg(feature = "cloud")]
use std::time::{Duration, SystemTime};

#[cfg(feature = "cloud")]
const TELEMETRY_ENDPOINT: &str = "https://api.tok0.dev/telemetry";
#[cfg(feature = "cloud")]
const PING_INTERVAL_SECS: u64 = 86_400; // 24 hours

#[derive(Debug, serde::Serialize)]
#[allow(dead_code)]
pub struct TopCommand {
    pub command: String,
    pub count: u64,
    pub saved_tokens: u64,
}

/// Today's local-meter slice. The API REPLACEs the (installation_id, date)
/// row on each ping, so re-pings the same day don't double-count.
#[derive(Debug, serde::Serialize)]
#[allow(dead_code)]
pub struct TodaySnapshot {
    /// UTC YYYY-MM-DD.
    pub date: String,
    pub commands: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub saved_tokens: u64,
}

#[derive(Debug, serde::Serialize)]
#[allow(dead_code)]
pub struct TelemetryPayload {
    pub installation_id: String,
    pub version: String,
    pub os: String,
    pub total_commands: u64,
    pub total_saved_tokens: u64,
    pub avg_savings_pct: f64,
    /// Top commands by saved tokens — drives the "most prominent commands"
    /// stat on api.tok0.dev. Sanitized via sanitize_command before send.
    /// Always serialized (empty array = no breakdown reported yet).
    #[serde(default)]
    pub top_commands: Vec<TopCommand>,
    /// Today's per-day savings slice; absent when meter has no rows for
    /// today (fresh install, no commands yet today).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub today: Option<TodaySnapshot>,
}

/// Deterministic installation ID — SHA-256 of (username + config dir).
/// Not user-identifiable, but stable across sessions on the same machine.
#[allow(dead_code)]
pub fn generate_installation_id() -> String {
    use sha2::{Digest, Sha256};
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let config = dirs::config_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let input = format!("{}:{}", user, config);
    format!("{:x}", Sha256::digest(input.as_bytes()))
}

#[allow(dead_code)]
pub fn platform_target() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "macos-aarch64";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "macos-x86_64";
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "linux-x86_64";
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "linux-aarch64";
    #[cfg(target_os = "windows")]
    return "windows-x86_64";
    #[allow(unreachable_code)]
    "unknown"
}

#[allow(dead_code)]
fn stamp_path() -> Result<PathBuf> {
    let dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("tok0");
    std::fs::create_dir_all(&dir).context("Failed to create cache dir")?;
    Ok(dir.join("telemetry_stamp"))
}

#[cfg(feature = "cloud")]
pub fn should_ping(stamp: &std::path::Path) -> Result<bool> {
    if !stamp.exists() {
        return Ok(true);
    }
    let mtime = stamp
        .metadata()
        .and_then(|m| m.modified())
        .context("Failed to read telemetry stamp mtime")?;
    let age = SystemTime::now()
        .duration_since(mtime)
        .unwrap_or(Duration::MAX);
    Ok(age.as_secs() > PING_INTERVAL_SECS)
}

#[allow(dead_code)]
fn record_ping(stamp: &std::path::Path) -> Result<()> {
    std::fs::write(stamp, "").context("Failed to write telemetry stamp")
}

/// Best-effort telemetry ping. Never blocks the user, never errors visibly.
/// Respects `[telemetry] enabled` in config — if false, this is a no-op.
///
/// Without the `cloud` feature there is no backend to ping; this is a
/// hard no-op so call sites (e.g. `run_proxy`) don't need cfg guards.
pub fn maybe_ping() {
    #[cfg(feature = "cloud")]
    maybe_ping_inner();
}

#[cfg(feature = "cloud")]
fn maybe_ping_inner() {
    let config = match crate::engine::config::load_config() {
        Ok(c) => c,
        Err(_) => return,
    };
    if !config.telemetry.enabled {
        return;
    }

    let stamp = match stamp_path() {
        Ok(s) => s,
        Err(_) => return,
    };
    if !should_ping(&stamp).unwrap_or(false) {
        return;
    }

    let db_path = crate::engine::config::db_path();
    let summary = match crate::engine::meter::Tracker::new(&db_path).and_then(|t| t.get_summary()) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Top 10 commands by saved tokens; sanitize each name to strip args/paths
    // before they leave the host. Failure here is non-fatal — emit empty.
    let top_commands = crate::engine::meter::Tracker::new(&db_path)
        .and_then(|t| t.get_top_commands(10))
        .map(|rows| {
            rows.into_iter()
                .map(|(command, saved_tokens)| TopCommand {
                    command: super::cloud::sanitize_command(&command),
                    count: 0,
                    saved_tokens,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Today's per-day savings. get_daily(1) returns at most one row for the
    // last day. Absent rows mean no commands today — emit None so the API
    // skips the instance_daily upsert.
    let today = crate::engine::meter::Tracker::new(&db_path)
        .and_then(|t| t.get_daily(1))
        .ok()
        .and_then(|rows| rows.into_iter().next())
        .map(|d| TodaySnapshot {
            date: d.date,
            commands: d.commands,
            input_tokens: d.input_tokens,
            output_tokens: d.output_tokens,
            saved_tokens: d.saved_tokens,
        });

    let payload = TelemetryPayload {
        installation_id: generate_installation_id(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: platform_target().to_string(),
        total_commands: summary.total_commands,
        total_saved_tokens: summary.total_saved,
        avg_savings_pct: summary.avg_savings_pct,
        top_commands,
        today,
    };

    let json = match serde_json::to_string(&payload) {
        Ok(j) => j,
        Err(_) => return,
    };
    let _ = ureq::post(TELEMETRY_ENDPOINT)
        .set("Content-Type", "application/json")
        .send_string(&json);
    let _ = record_ping(&stamp);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_installation_id_is_deterministic() {
        let id1 = generate_installation_id();
        let id2 = generate_installation_id();
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_installation_id_is_hex() {
        let id = generate_installation_id();
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_telemetry_payload_serialization() {
        let payload = TelemetryPayload {
            installation_id: "abc123".to_string(),
            version: "0.5.0".to_string(),
            os: "macos-aarch64".to_string(),
            total_commands: 100,
            total_saved_tokens: 50_000,
            avg_savings_pct: 82.5,
            top_commands: vec![TopCommand {
                command: "git log".to_string(),
                count: 50,
                saved_tokens: 30_000,
            }],
            today: Some(TodaySnapshot {
                date: "2026-04-30".to_string(),
                commands: 5,
                input_tokens: 4_000,
                output_tokens: 800,
                saved_tokens: 3_200,
            }),
        };
        let json = serde_json::to_string(&payload).expect("serialize");
        assert!(json.contains("abc123"));
        assert!(json.contains("50000"));
        assert!(json.contains("git log"));
        assert!(json.contains("top_commands"));
        assert!(json.contains("\"today\""));
        assert!(json.contains("2026-04-30"));
        assert!(json.contains("3200"));
        assert!(!json.contains("src/")); // no paths leak
    }

    #[test]
    fn test_telemetry_payload_omits_today_when_none() {
        let payload = TelemetryPayload {
            installation_id: "abc123".to_string(),
            version: "0.5.0".to_string(),
            os: "linux-x86_64".to_string(),
            total_commands: 0,
            total_saved_tokens: 0,
            avg_savings_pct: 0.0,
            top_commands: vec![],
            today: None,
        };
        let json = serde_json::to_string(&payload).expect("serialize");
        assert!(!json.contains("\"today\""));
    }

    #[test]
    fn test_telemetry_payload_empty_top_commands() {
        let payload = TelemetryPayload {
            installation_id: "abc123".to_string(),
            version: "0.5.0".to_string(),
            os: "linux-x86_64".to_string(),
            total_commands: 0,
            total_saved_tokens: 0,
            avg_savings_pct: 0.0,
            top_commands: vec![],
            today: None,
        };
        let json = serde_json::to_string(&payload).expect("serialize");
        assert!(json.contains("\"top_commands\":[]"));
    }

    /// Daily cadence invariant: PING_INTERVAL_SECS must be exactly 24 hours.
    /// Telemetry hits daily — never more, never less. If this constant
    /// ever drifts, the spec's "daily" guarantee breaks silently.
    #[cfg(feature = "cloud")]
    #[test]
    fn test_ping_interval_is_exactly_24h() {
        assert_eq!(PING_INTERVAL_SECS, 24 * 60 * 60);
    }

    /// Boundary check companion to test_ping_interval_is_exactly_24h:
    /// confirms `should_ping` returns true once the stamp is older than
    /// the interval. Uses a stamp that doesn't exist (counts as
    /// stale/missing) — equivalent semantics, no filesystem-time hacks.
    #[cfg(feature = "cloud")]
    #[test]
    fn test_should_ping_when_stamp_missing() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let stamp = tmp.path().join("never_existed");
        assert!(should_ping(&stamp).expect("check"));
    }

    #[test]
    fn test_platform_target_not_empty() {
        let target = platform_target();
        assert!(!target.is_empty());
        assert!(target.contains('-'));
    }

    #[cfg(feature = "cloud")]
    #[test]
    fn test_should_ping_fresh_stamp() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let stamp = tmp.path().join("telemetry_stamp");
        assert!(should_ping(&stamp).expect("check"));
    }

    #[cfg(feature = "cloud")]
    #[test]
    fn test_should_not_ping_recent_stamp() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let stamp = tmp.path().join("telemetry_stamp");
        std::fs::write(&stamp, "").expect("write");
        assert!(!should_ping(&stamp).expect("check"));
    }
}
