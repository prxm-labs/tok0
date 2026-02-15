use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use zeroize::Zeroize;

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct Tok0Config {
    #[serde(default)]
    pub tracking: TrackingConfig,
    #[serde(default)]
    pub hooks: HooksConfig,
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub limits: LimitsConfig,
    #[serde(default)]
    pub telemetry: TelemetryConfig,
    #[serde(default)]
    pub update: UpdateConfig,
    #[serde(default)]
    pub cloud: CloudConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct TrackingConfig {
    pub enabled: bool,
    pub history_days: u32,
    pub database_path: Option<PathBuf>,
}

impl Default for TrackingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            history_days: 90,
            database_path: None,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct HooksConfig {
    #[serde(default)]
    pub exclude_commands: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DisplayConfig {
    pub colors: bool,
    pub emoji: bool,
    pub max_width: usize,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            colors: true,
            emoji: true,
            max_width: 120,
        }
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct LimitsConfig {
    pub grep_max_results: usize,
    pub grep_max_per_file: usize,
    pub status_max_files: usize,
    pub passthrough_max_chars: usize,
    pub command_timeout_secs: u64,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            grep_max_results: 100,
            grep_max_per_file: 10,
            status_max_files: 50,
            passthrough_max_chars: 50_000,
            command_timeout_secs: 30,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TelemetryConfig {
    pub enabled: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UpdateConfig {
    pub auto_check: bool,
    pub check_interval_hours: u64,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            auto_check: true,
            check_interval_hours: 24,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct CloudConfig {
    pub enabled: bool,
    pub api_url: Option<String>,
    pub team_id: Option<String>,
    pub api_key: Option<String>,
}

// Wipe the API key from memory when CloudConfig is dropped.
// The bool / api_url / team_id fields aren't sensitive.
impl Drop for CloudConfig {
    fn drop(&mut self) {
        if let Some(key) = self.api_key.as_mut() {
            key.zeroize();
        }
    }
}

pub fn load_config() -> Result<Tok0Config> {
    let path = config_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        toml::from_str(&content).context("Failed to parse config TOML")
    } else {
        Ok(Tok0Config::default())
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tok0")
        .join("config.toml")
}

/// Replace a TOML section (e.g., `[cloud]`) with new content.
/// If the section doesn't exist, appends it.
fn replace_toml_section(content: &str, section_name: &str, new_section: &str) -> String {
    let header = format!("[{}]", section_name);
    if content.contains(&header) {
        let mut result = String::new();
        let mut in_section = false;
        for line in content.lines() {
            if line.trim() == header {
                in_section = true;
                continue;
            }
            if in_section {
                if line.starts_with('[') {
                    result.push_str(new_section);
                    result.push_str(line);
                    result.push('\n');
                    in_section = false;
                }
                continue;
            }
            result.push_str(line);
            result.push('\n');
        }
        if in_section {
            result.push_str(new_section);
        }
        result
    } else {
        format!("{}\n{}", content.trim_end(), new_section)
    }
}

/// Toggle telemetry in the config file. Rewrites the `[telemetry]` section.
pub fn set_telemetry_enabled(config_path: &std::path::Path, enabled: bool) -> Result<()> {
    let content = if config_path.exists() {
        std::fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config: {}", config_path.display()))?
    } else {
        String::new()
    };

    let new_section = format!("[telemetry]\nenabled = {}\n", enabled);
    let updated = replace_toml_section(&content, "telemetry", &new_section);

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config dir: {}", parent.display()))?;
    }
    std::fs::write(config_path, updated.trim_start())
        .with_context(|| format!("Failed to write config: {}", config_path.display()))
}

// ROADMAP T1.4 — keyring-backed credential storage. The OS keyring is
// the right place for bearer tokens; ~/.config/tok0/config.toml is
// world-readable on most systems. These helpers are only compiled into
// `cloud`-feature builds (the only path that handles tokens).

/// Service identifier under which the API key is stored. macOS Keychain
/// shows it as the entry's "service"; Linux Secret Service uses it as
/// the schema attribute; Windows Credential Manager as the target name.
#[cfg(feature = "cloud")]
const KEYRING_SERVICE: &str = "com.prxm-labs.tok0";

#[cfg(feature = "cloud")]
const KEYRING_USER: &str = "cloud-api-key";

/// Write the API key to the OS keyring.
#[cfg(feature = "cloud")]
pub fn set_api_key_in_keyring(key: &str) -> Result<()> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("Failed to open keyring entry")?
        .set_password(key)
        .context("Failed to write API key to OS keyring")
}

/// Read the API key from the OS keyring. Returns `Ok(None)` if no entry
/// exists (first run, or after explicit logout).
#[cfg(feature = "cloud")]
pub fn get_api_key_from_keyring() -> Result<Option<String>> {
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("Failed to open keyring entry")?
        .get_password()
    {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::Error::new(e).context("Failed to read API key from keyring")),
    }
}

/// Delete the API key from the OS keyring. Idempotent.
#[cfg(feature = "cloud")]
pub fn clear_api_key_from_keyring() -> Result<()> {
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("Failed to open keyring entry")?
        .delete_credential()
    {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow::Error::new(e).context("Failed to clear API key from keyring")),
    }
}

/// Persist cloud credentials. The API key goes to the OS keyring; only
/// the API URL is written to plaintext config. Older versions wrote
/// `api_key = "..."` straight to config.toml — the new flow strips
/// that line if present so an upgrade-then-login removes the plaintext
/// without manual cleanup.
#[allow(dead_code)]
pub fn set_cloud_credentials(
    config_path: &std::path::Path,
    api_key: &str,
    api_url: &str,
) -> Result<()> {
    #[cfg(feature = "cloud")]
    set_api_key_in_keyring(api_key)?;
    // Suppress unused warnings when the cloud feature is off — without
    // it the function is also `#[allow(dead_code)]` and the keyring
    // call disappears, so the parameter is intentionally unused.
    #[cfg(not(feature = "cloud"))]
    let _ = api_key;

    let content = if config_path.exists() {
        std::fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config: {}", config_path.display()))?
    } else {
        String::new()
    };

    // No api_key field in the TOML anymore — only enabled flag + url.
    let new_section = format!("[cloud]\nenabled = true\napi_url = \"{}\"\n", api_url);

    let updated = replace_toml_section(&content, "cloud", &new_section);

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config dir: {}", parent.display()))?;
    }
    std::fs::write(config_path, updated.trim_start())
        .with_context(|| format!("Failed to write config: {}", config_path.display()))
}

/// Remove cloud credentials — sets enabled=false in config and clears
/// the keyring entry. Idempotent.
#[allow(dead_code)]
pub fn clear_cloud_credentials(config_path: &std::path::Path) -> Result<()> {
    #[cfg(feature = "cloud")]
    clear_api_key_from_keyring()?;

    let content = if config_path.exists() {
        std::fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config: {}", config_path.display()))?
    } else {
        return Ok(());
    };

    let cleared_section = "[cloud]\nenabled = false\n";
    let updated = replace_toml_section(&content, "cloud", cleared_section);

    std::fs::write(config_path, updated.trim_start())
        .with_context(|| format!("Failed to write config: {}", config_path.display()))
}

pub fn db_path() -> PathBuf {
    if let Ok(p) = std::env::var("TOK0_DB_PATH") {
        return PathBuf::from(p);
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tok0")
        .join("tracking.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Tok0Config::default();
        assert!(config.tracking.enabled);
        assert_eq!(config.tracking.history_days, 90);
        assert_eq!(config.limits.command_timeout_secs, 30);
        assert_eq!(config.limits.grep_max_results, 100);
    }

    #[test]
    fn test_parse_minimal_toml() {
        let toml_str = r#"
            [tracking]
            enabled = false
        "#;
        let config: Tok0Config = toml::from_str(toml_str).expect("should parse minimal TOML");
        assert!(!config.tracking.enabled);
        assert!(config.display.colors); // default
    }

    #[test]
    fn test_parse_full_toml() {
        let toml_str = r#"
            [tracking]
            enabled = true
            history_days = 30

            [display]
            colors = false
            emoji = false
            max_width = 80

            [limits]
            grep_max_results = 50
            grep_max_per_file = 5
            status_max_files = 25
            passthrough_max_chars = 10000
            command_timeout_secs = 60

            [cloud]
            enabled = true
            api_url = "https://api.example.com"
        "#;
        let config: Tok0Config = toml::from_str(toml_str).expect("should parse full TOML");
        assert_eq!(config.tracking.history_days, 30);
        assert!(!config.display.colors);
        assert_eq!(config.limits.grep_max_results, 50);
        assert!(config.cloud.enabled);
    }

    #[test]
    fn test_parse_empty_toml() {
        let config: Tok0Config = toml::from_str("").expect("should parse empty TOML");
        assert!(config.tracking.enabled);
    }

    #[test]
    fn test_config_path_not_empty() {
        let path = config_path();
        assert!(path.to_str().is_some());
    }

    #[test]
    fn test_set_telemetry_enabled_creates_section() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "").expect("write");
        set_telemetry_enabled(&path, false).expect("set");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.contains("[telemetry]"));
        assert!(content.contains("enabled = false"));
    }

    #[test]
    fn test_set_telemetry_enabled_updates_existing() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "[telemetry]\nenabled = true\n").expect("write");
        set_telemetry_enabled(&path, false).expect("set");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.contains("enabled = false"));
        assert_eq!(content.matches("[telemetry]").count(), 1);
    }

    #[test]
    /// Install the keyring crate's in-memory mock backend so tests on
    /// `--features cloud` don't touch the real OS keyring. The mock is
    /// process-local; setting it once per test is fine because cargo
    /// runs each test in its own thread but a single process.
    #[cfg(feature = "cloud")]
    fn install_mock_keyring() {
        // The crate uses `set_default_credential_builder`. Calling more
        // than once across tests is fine — the builder swap is idempotent.
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
    }

    #[test]
    fn test_set_cloud_credentials_creates_section() {
        #[cfg(feature = "cloud")]
        install_mock_keyring();
        let tmp = tempfile::TempDir::new().expect("tmp");
        let path = tmp.path().join("config.toml");
        std::fs::write(&path, "").expect("write");
        set_cloud_credentials(&path, "tok_abc123", "https://api.tok0.dev").expect("set");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.contains("[cloud]"));
        assert!(content.contains("enabled = true"));
        assert!(content.contains(r#"api_url = "https://api.tok0.dev""#));
        // T1.4: the API key must NEVER land in the plaintext config file.
        assert!(
            !content.contains("api_key"),
            "api_key field must not appear in config.toml: {}",
            content
        );
        assert!(
            !content.contains("tok_abc123"),
            "raw API key bytes must not appear in config.toml: {}",
            content
        );
    }

    #[test]
    fn test_set_cloud_credentials_replaces_existing() {
        #[cfg(feature = "cloud")]
        install_mock_keyring();
        let tmp = tempfile::TempDir::new().expect("tmp");
        let path = tmp.path().join("config.toml");
        // Pre-existing legacy plaintext key — must be stripped by the
        // upgrade-then-login flow.
        std::fs::write(
            &path,
            "[cloud]\nenabled = false\napi_key = \"old_plaintext_key\"\n",
        )
        .expect("write");
        set_cloud_credentials(&path, "new_key", "https://api.tok0.dev").expect("set");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(
            !content.contains("old_plaintext_key"),
            "legacy plaintext api_key must be wiped on first login"
        );
        assert!(
            !content.contains("new_key"),
            "new API key must not appear in config.toml plaintext"
        );
        assert!(!content.contains("api_key"));
        assert_eq!(content.matches("[cloud]").count(), 1);
    }

    #[test]
    fn test_clear_cloud_credentials() {
        #[cfg(feature = "cloud")]
        install_mock_keyring();
        let tmp = tempfile::TempDir::new().expect("tmp");
        let path = tmp.path().join("config.toml");
        // Worst case: legacy file still has plaintext api_key from a
        // pre-T1.4 install.
        std::fs::write(
            &path,
            "[cloud]\nenabled = true\napi_key = \"legacy_secret\"\napi_url = \"https://api.tok0.dev\"\n",
        )
        .expect("write");
        clear_cloud_credentials(&path).expect("clear");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.contains("enabled = false"));
        assert!(
            !content.contains("legacy_secret"),
            "clear must wipe any plaintext api_key from config: {}",
            content
        );
    }

    /// T1.4 keyring helpers exist and don't panic. We can't assert
    /// round-trip persistence here because keyring's `mock` backend
    /// gives each `Entry::new` its own private store, so set→get
    /// across distinct Entry instances always returns None. The
    /// real OS keyring shares state via the kernel/secret-service,
    /// and that path is exercised end-to-end by users at runtime.
    /// What we _can_ verify in tests:
    ///   - the helpers compile and execute under the `cloud` feature
    ///   - the no-plaintext invariant on the config file (covered by
    ///     `test_set_cloud_credentials_*` and `test_clear_cloud_credentials`).
    #[cfg(feature = "cloud")]
    #[test]
    fn test_keyring_helpers_dont_panic_under_mock() {
        install_mock_keyring();
        let _ = set_api_key_in_keyring("tok_smoke");
        let _ = get_api_key_from_keyring();
        let _ = clear_api_key_from_keyring();
    }

    #[test]
    fn test_clear_nonexistent_config_is_ok() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let path = tmp.path().join("does_not_exist.toml");
        clear_cloud_credentials(&path).expect("should not fail");
    }

    #[test]
    fn test_replace_toml_section_preserves_other_sections() {
        let content =
            "[tracking]\nenabled = true\n\n[cloud]\nold = true\n\n[display]\ncolors = false\n";
        let result = replace_toml_section(content, "cloud", "[cloud]\nnew = true\n");
        assert!(result.contains("[tracking]"));
        assert!(result.contains("enabled = true"));
        assert!(result.contains("[cloud]"));
        assert!(result.contains("new = true"));
        assert!(!result.contains("old = true"));
        assert!(result.contains("[display]"));
        assert!(result.contains("colors = false"));
    }
}
