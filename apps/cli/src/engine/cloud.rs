use anyhow::{Context, Result};
use zeroize::Zeroize;

/// Strip arguments from a command, keeping only the program and first subcommand.
/// Prevents file paths, secrets, or user data from being reported.
pub fn sanitize_command(command: &str) -> String {
    let parts: Vec<&str> = command.split_whitespace().collect();
    match parts.as_slice() {
        [] => String::new(),
        [prog] => prog.to_string(),
        [prog, sub, ..] if !sub.starts_with('-') => format!("{} {}", prog, sub),
        [prog, ..] => prog.to_string(),
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct ReportEvent {
    pub command: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub savings_pct: u64,
    pub timestamp_secs: u64,
}

pub struct CloudClient {
    api_url: String,
    api_key: Option<String>,
    pub(crate) pending: Vec<ReportEvent>,
    offline: bool,
}

// Wipe the bearer token from memory when the client is dropped.
impl Drop for CloudClient {
    fn drop(&mut self) {
        if let Some(key) = self.api_key.as_mut() {
            key.zeroize();
        }
    }
}

impl CloudClient {
    pub fn new(api_url: &str, api_key: Option<&str>) -> Self {
        Self {
            api_url: api_url.to_string(),
            api_key: api_key.map(|s| s.to_string()),
            pending: Vec::new(),
            offline: false,
        }
    }

    pub fn new_offline() -> Self {
        Self {
            api_url: String::new(),
            api_key: None,
            pending: Vec::new(),
            offline: true,
        }
    }

    pub fn from_config(config: &crate::engine::config::CloudConfig) -> Self {
        if !config.enabled {
            return Self::new_offline();
        }
        // Prefer the OS keyring (T1.4) and fall back to the legacy
        // plaintext `api_key` in TOML for users mid-migration. Once
        // they re-run `tok0 auth login` the plaintext line is wiped
        // by `set_cloud_credentials` and only the keyring remains.
        let key_from_keyring = crate::engine::config::get_api_key_from_keyring()
            .ok()
            .flatten();
        let key = key_from_keyring.as_deref().or(config.api_key.as_deref());
        let Some(key) = key else {
            return Self::new_offline();
        };
        let url = config.api_url.as_deref().unwrap_or("https://api.tok0.dev");
        Self::new(url, Some(key))
    }

    pub fn record(&mut self, command: &str, input_tokens: u64, output_tokens: u64) {
        let savings_pct = (input_tokens.saturating_sub(output_tokens))
            .saturating_mul(100)
            .checked_div(input_tokens)
            .unwrap_or(0);
        self.pending.push(ReportEvent {
            command: sanitize_command(command),
            input_tokens,
            output_tokens,
            savings_pct,
            timestamp_secs: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Flush pending events to the cloud API.
    /// If offline, this is a no-op (events stay pending).
    pub fn flush(&mut self) -> Result<()> {
        if self.offline || self.pending.is_empty() {
            return Ok(());
        }
        let payload = serde_json::to_string(&self.pending).context("Failed to serialize events")?;
        let api_key = self.api_key.as_deref().unwrap_or("");
        // The Bearer header allocates a fresh String that holds the token
        // verbatim. Wipe it before the heap returns to the allocator —
        // ROADMAP T1.2.
        let mut header = format!("Bearer {}", api_key);
        let result = ureq::post(&format!("{}/report", self.api_url))
            .set("Authorization", &header)
            .set("Content-Type", "application/json")
            .send_string(&payload);
        header.zeroize();
        result.context("Failed to send events to cloud API")?;
        self.pending.clear();
        Ok(())
    }
}

/// Validate a token against the cloud API.
pub fn validate_token(api_url: &str, token: &str) -> Result<()> {
    let mut header = format!("Bearer {}", token);
    let result = ureq::get(&format!("{}/team/stats", api_url))
        .set("Authorization", &header)
        .call();
    header.zeroize();
    result.context("Authentication failed — check your API key or network")?;
    Ok(())
}

/// Fetch team stats from the cloud API. Requires authentication.
pub fn fetch_team_stats(api_url: &str, api_key: &str) -> Result<serde_json::Value> {
    let mut header = format!("Bearer {}", api_key);
    let response = ureq::get(&format!("{}/team/stats", api_url))
        .set("Authorization", &header)
        .call();
    header.zeroize();
    let body = response
        .context("Failed to fetch team stats — check your connection")?
        .into_string()
        .context("Failed to read team stats response")?;
    let response: serde_json::Value =
        serde_json::from_str(&body).context("Failed to parse team stats response")?;
    Ok(response)
}

/// Format team stats for terminal display.
pub fn format_team_stats(stats: &serde_json::Value) -> String {
    let savings = stats["savings_pct"].as_u64().unwrap_or(0);
    let commands = stats["total_commands"].as_u64().unwrap_or(0);
    let input = stats["total_input_tokens"].as_u64().unwrap_or(0);
    let output = stats["total_output_tokens"].as_u64().unwrap_or(0);
    let members = stats["members"].as_u64().unwrap_or(0);
    format!(
        "Team savings : {}%\nTotal tokens : {} in → {} out\nCommands     : {}\nMembers      : {}",
        savings, input, output, commands, members
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_strips_paths() {
        assert_eq!(sanitize_command("git log src/main.rs"), "git log");
        assert_eq!(sanitize_command("cargo build --release"), "cargo build");
        assert_eq!(sanitize_command("grep -r password ./secrets"), "grep");
    }

    #[test]
    fn test_sanitize_keeps_subcommand() {
        assert_eq!(sanitize_command("git status"), "git status");
        assert_eq!(sanitize_command("npm run build"), "npm run");
        assert_eq!(sanitize_command("cargo test"), "cargo test");
    }

    #[test]
    fn test_sanitize_empty() {
        assert_eq!(sanitize_command(""), "");
    }

    #[test]
    fn test_sanitize_single_command() {
        assert_eq!(sanitize_command("ls"), "ls");
    }

    #[test]
    fn test_report_event_serialization() {
        let event = ReportEvent {
            command: "git log".to_string(),
            input_tokens: 500,
            output_tokens: 120,
            savings_pct: 76,
            timestamp_secs: 1712345678,
        };
        let json = serde_json::to_string(&event).expect("serialize");
        assert!(json.contains("git log"));
        assert!(json.contains("500"));
        assert!(!json.contains("src/"));
    }

    #[test]
    fn test_batch_accumulates_events() {
        let mut client = CloudClient::new_offline();
        client.record("git log", 500, 120);
        client.record("cargo build", 8000, 400);
        assert_eq!(client.pending_count(), 2);
    }

    #[test]
    fn test_offline_flush_is_noop() {
        let mut client = CloudClient::new_offline();
        client.record("git log", 500, 120);
        client.flush().expect("flush");
        assert_eq!(client.pending_count(), 1);
    }

    #[test]
    fn test_savings_pct_calculation() {
        let mut client = CloudClient::new_offline();
        client.record("test", 1000, 200);
        assert_eq!(client.pending[0].savings_pct, 80);
    }

    #[test]
    fn test_savings_pct_zero_input() {
        let mut client = CloudClient::new_offline();
        client.record("test", 0, 0);
        assert_eq!(client.pending[0].savings_pct, 0);
    }

    #[test]
    fn test_from_config_disabled() {
        let config = crate::engine::config::CloudConfig {
            enabled: false,
            api_url: Some("https://example.com".to_string()),
            api_key: Some("secret".to_string()),
            team_id: None,
        };
        let client = CloudClient::from_config(&config);
        assert!(client.offline);
    }

    #[test]
    fn test_from_config_no_key() {
        let config = crate::engine::config::CloudConfig {
            enabled: true,
            api_url: Some("https://example.com".to_string()),
            api_key: None,
            team_id: None,
        };
        let client = CloudClient::from_config(&config);
        assert!(client.offline);
    }

    #[test]
    fn test_format_team_stats() {
        let json = r#"{
            "total_input_tokens": 1000000,
            "total_output_tokens": 200000,
            "savings_pct": 80,
            "total_commands": 5000,
            "members": 12
        }"#;
        let val: serde_json::Value = serde_json::from_str(json).expect("parse");
        let formatted = format_team_stats(&val);
        assert!(formatted.contains("80%"));
        assert!(formatted.contains("5000"));
        assert!(formatted.contains("12"));
        assert!(formatted.contains("1000000"));
    }

    #[test]
    fn test_format_team_stats_missing_fields() {
        let val: serde_json::Value = serde_json::from_str("{}").expect("parse");
        let formatted = format_team_stats(&val);
        assert!(formatted.contains("0%"));
        assert!(formatted.contains("0 in"));
    }

    #[test]
    fn test_cloud_client_zeroizes_api_key_on_drop() {
        // Build a client with a long, distinctive key so we can inspect memory.
        let original = "tok_supersecret_key_zzzzzzz";
        let client = CloudClient::new("https://api.example.com", Some(original));
        // Pointer to the heap buffer backing the key.
        let buf_ptr = client.api_key.as_ref().expect("key present").as_ptr();
        let len = client.api_key.as_ref().expect("key present").len();

        // Trigger the Drop impl explicitly.
        drop(client);

        // After drop, the heap buffer should no longer contain the original bytes.
        // SAFETY: freed memory may have been reallocated; we're checking the
        // specific bytes set by zeroize before deallocation ran.
        // This is a best-effort check — the allocator may overwrite freed memory.
        // We only assert that the buffer does NOT still contain the full original.
        let buf_slice = unsafe { std::slice::from_raw_parts(buf_ptr, len) };
        let as_str = std::str::from_utf8(buf_slice).unwrap_or("");
        assert!(
            !as_str.contains(original),
            "api_key bytes should be wiped after drop, found: {:?}",
            as_str
        );
    }

    #[test]
    fn test_cloud_config_zeroizes_api_key_on_drop() {
        let original = "tok_secret_to_wipe_aaaaaaa";
        let config = crate::engine::config::CloudConfig {
            enabled: true,
            api_url: Some("https://api.example.com".to_string()),
            api_key: Some(original.to_string()),
            team_id: None,
        };
        let buf_ptr = config.api_key.as_ref().expect("key present").as_ptr();
        let len = config.api_key.as_ref().expect("key present").len();

        drop(config);

        let buf_slice = unsafe { std::slice::from_raw_parts(buf_ptr, len) };
        let as_str = std::str::from_utf8(buf_slice).unwrap_or("");
        assert!(
            !as_str.contains(original),
            "CloudConfig.api_key bytes should be wiped after drop, found: {:?}",
            as_str
        );
    }

    /// ROADMAP T1.2 invariant: the `Bearer <token>` String we synthesize
    /// for each HTTP request gets zeroized before the heap returns. The
    /// wipe pattern is what every cloud HTTP path uses.
    #[test]
    fn test_bearer_header_pattern_wipes_in_place() {
        use zeroize::Zeroize;
        let token = "tok_bearer_test_zzz";
        let mut header = format!("Bearer {}", token);
        let ptr = header.as_ptr();
        let len = header.len();
        header.zeroize();
        // SAFETY: header still owns the buffer (zeroize doesn't free).
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        let as_str = std::str::from_utf8(slice).unwrap_or("");
        assert!(
            !as_str.contains(token),
            "bearer header should be wiped, found: {:?}",
            as_str
        );
    }
}
