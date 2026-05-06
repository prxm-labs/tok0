use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub fn parse_version(v: &str) -> Result<(u64, u64, u64)> {
    let v = v.trim_start_matches('v');
    let parts: Vec<&str> = v.splitn(3, '.').collect();
    if parts.len() != 3 {
        anyhow::bail!("Invalid semver: expected X.Y.Z, got '{}'", v);
    }
    let major = parts[0]
        .parse::<u64>()
        .with_context(|| format!("Invalid major: {}", parts[0]))?;
    let minor = parts[1]
        .parse::<u64>()
        .with_context(|| format!("Invalid minor: {}", parts[1]))?;
    let patch = parts[2]
        .parse::<u64>()
        .with_context(|| format!("Invalid patch: {}", parts[2]))?;
    Ok((major, minor, patch))
}

pub fn is_newer(candidate: &str, current: &str) -> Result<bool> {
    let c = parse_version(candidate)?;
    let cur = parse_version(current)?;
    Ok(c > cur)
}

#[derive(Debug)]
pub struct ReleaseInfo {
    pub version: String,
    pub asset_url: String,
    pub checksum_url: String,
}

pub fn parse_release_json(json: &str) -> Result<ReleaseInfo> {
    let val: serde_json::Value =
        serde_json::from_str(json).context("Failed to parse GitHub release JSON")?;
    let tag = val["tag_name"]
        .as_str()
        .context("Missing tag_name in release")?
        .trim_start_matches('v')
        .to_string();
    let target = current_platform_target();
    let assets = val["assets"].as_array().context("Missing assets array")?;
    let asset = assets
        .iter()
        .find(|a| {
            a["name"]
                .as_str()
                .map(|n| n.contains(target))
                .unwrap_or(false)
        })
        .with_context(|| format!("No asset found for platform: {}", target))?;
    let asset_url = asset["browser_download_url"]
        .as_str()
        .context("Missing browser_download_url")?
        .to_string();
    let checksum_url = format!("{}.sha256", asset_url);
    Ok(ReleaseInfo {
        version: tag,
        asset_url,
        checksum_url,
    })
}

fn current_platform_target() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "x86_64-apple-darwin";
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "aarch64-apple-darwin";
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "x86_64-unknown-linux-musl";
    #[cfg(target_os = "windows")]
    return "x86_64-pc-windows-msvc";
    #[allow(unreachable_code)]
    "x86_64-unknown-linux-musl"
}

const GITHUB_API: &str = "https://api.github.com/repos/prxm-labs/tok0/releases/latest";
const CHECK_INTERVAL_SECS: u64 = 86400;

/// Check GitHub for a newer release. Returns None if rate-limited or up to date.
pub fn check_for_update(current_version: &str) -> Result<Option<ReleaseInfo>> {
    let stamp = last_check_stamp_path()?;
    if !should_check_with_stamp(&stamp)? {
        return Ok(None);
    }
    let result = fetch_latest_release(current_version);
    let _ = record_check_time(&stamp);
    result
}

/// Fetch latest release from GitHub and return it if newer than current.
fn fetch_latest_release(current_version: &str) -> Result<Option<ReleaseInfo>> {
    let response = ureq::get(GITHUB_API)
        .set("User-Agent", &format!("tok0/{}", current_version))
        .call()
        .context("Failed to reach GitHub API")?
        .into_string()
        .context("Failed to read GitHub API response")?;

    let release = parse_release_json(&response)?;
    if is_newer(&release.version, current_version)? {
        Ok(Some(release))
    } else {
        Ok(None)
    }
}

fn should_check_with_stamp(stamp: &std::path::Path) -> Result<bool> {
    if !stamp.exists() {
        return Ok(true);
    }
    let mtime = stamp
        .metadata()
        .and_then(|m| m.modified())
        .context("Failed to read stamp mtime")?;
    let age = SystemTime::now()
        .duration_since(mtime)
        .unwrap_or(Duration::MAX);
    Ok(age.as_secs() > CHECK_INTERVAL_SECS)
}

fn last_check_stamp_path() -> Result<PathBuf> {
    let dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("tok0");
    std::fs::create_dir_all(&dir).context("Failed to create cache dir")?;
    Ok(dir.join("last_update_check"))
}

fn record_check_time(stamp: &std::path::Path) -> Result<()> {
    std::fs::write(stamp, "").context("Failed to write update stamp")
}

fn verify_sha256(data: &[u8], expected: &str) -> Result<()> {
    use sha2::{Digest, Sha256};
    let actual = format!("{:x}", Sha256::digest(data));
    if actual != expected {
        anyhow::bail!("SHA-256 mismatch: expected {}, got {}", expected, actual);
    }
    Ok(())
}

pub fn download_release(info: &ReleaseInfo, dest: &std::path::Path) -> Result<()> {
    use std::io::Read as _;

    // Download binary
    let response = ureq::get(&info.asset_url)
        .set("User-Agent", "tok0/updater")
        .call()
        .context("Failed to download release binary")?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .context("Failed to read release binary")?;

    // Download and parse checksum
    let checksum_response = ureq::get(&info.checksum_url)
        .set("User-Agent", "tok0/updater")
        .call()
        .context("Failed to download checksum file")?;
    let checksum_file = checksum_response
        .into_string()
        .context("Failed to read checksum")?;
    let expected_hash = checksum_file
        .split_whitespace()
        .next()
        .context("Empty checksum file")?;

    // Verify integrity
    verify_sha256(&bytes, expected_hash)?;

    // Write to temp file then atomic rename
    let tmp = dest.with_extension("tmp");
    std::fs::write(&tmp, &bytes)
        .with_context(|| format!("Failed to write temp binary: {}", tmp.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .context("Failed to set executable permissions")?;
    }

    std::fs::rename(&tmp, dest)
        .with_context(|| format!("Failed to atomically replace binary: {}", dest.display()))?;

    Ok(())
}

pub fn replace_binary_with_rollback(
    new_binary: &std::path::Path,
    current_binary: &std::path::Path,
) -> Result<()> {
    let backup = current_binary.with_extension("bak");
    std::fs::copy(current_binary, &backup)
        .with_context(|| format!("Failed to back up current binary to {}", backup.display()))?;

    match std::fs::rename(new_binary, current_binary) {
        Ok(_) => {
            let _ = std::fs::remove_file(&backup);
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::rename(&backup, current_binary);
            Err(e).context("Failed to replace binary; rolled back to previous version")
        }
    }
}

/// Verify that tok0 can write to the directory containing its current
/// binary before kicking off any download. When tok0 is installed at a
/// system-wide path like `/usr/local/bin/tok0`, the user running
/// `tok0 update` typically does not own that directory and the silent
/// failure mode is "downloaded N MB, then permission denied on the
/// rename step". Failing fast with an actionable message is much better.
fn preflight_write_access(current_binary: &Path) -> Result<()> {
    let dir = current_binary
        .parent()
        .context("Could not determine parent directory of current binary")?;

    // Attempt to create a uniquely-named tempfile in the install dir.
    // tempfile auto-deletes on drop, so we leave no probe artifact.
    match tempfile::Builder::new()
        .prefix(".tok0-update-probe-")
        .tempfile_in(dir)
    {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            anyhow::bail!(
                "Cannot update tok0: permission denied writing to {}.\n\n\
                 Try one of:\n  \
                 sudo tok0 update\n  \
                 TOK0_INSTALL_DIR=\"$HOME/.local/bin\" sh -c \"$(curl -fsSL https://tok0.dev/install.sh)\"",
                dir.display()
            )
        }
        Err(e) => {
            Err(e).with_context(|| format!("Failed to verify writability of {}", dir.display()))
        }
    }
}

pub fn run_update() -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    eprintln!("tok0: current version {}", current);

    let current_binary =
        std::env::current_exe().context("Failed to determine current binary path")?;
    // Bail before any network round-trips if we can't write to the
    // install dir. This avoids the "download succeeded, install failed
    // with a leftover .tmp" trap when tok0 lives in /usr/local/bin.
    preflight_write_access(&current_binary)?;

    eprintln!("tok0: checking for updates...");

    let release = match check_for_update_forced(current)? {
        None => {
            eprintln!("tok0: already up to date.");
            return Ok(());
        }
        Some(r) => r,
    };

    eprintln!(
        "tok0: new version available: {} -> {}",
        current, release.version
    );

    let tmp_path = current_binary.with_extension("new");

    eprintln!("tok0: downloading...");
    download_release(&release, &tmp_path).context("Failed to download new release")?;

    eprintln!("tok0: verifying and installing...");
    replace_binary_with_rollback(&tmp_path, &current_binary)
        .context("Failed to install new binary")?;

    eprintln!(
        "tok0: updated to {}. Restart tok0 to use the new version.",
        release.version
    );
    Ok(())
}

fn format_update_hint(new_version: &str) -> String {
    format!(
        "\ntok0: update available v{} — run `tok0 update` to install.\n",
        new_version
    )
}

/// Call after meta commands complete. Rate-limited, silent on failure.
pub fn maybe_print_update_hint() {
    let current = env!("CARGO_PKG_VERSION");
    if let Ok(Some(release)) = check_for_update(current) {
        eprint!("{}", format_update_hint(&release.version));
    }
}

/// Like check_for_update but bypasses rate limiting (for explicit `tok0 update`).
fn check_for_update_forced(current_version: &str) -> Result<Option<ReleaseInfo>> {
    let result = fetch_latest_release(current_version);
    // Record check time so passive hints don't re-check immediately
    if let Ok(stamp) = last_check_stamp_path() {
        let _ = record_check_time(&stamp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("1.2.3").expect("valid"), (1, 2, 3));
        assert_eq!(parse_version("v1.2.3").expect("valid"), (1, 2, 3));
        assert_eq!(parse_version("0.0.1").expect("valid"), (0, 0, 1));
    }

    #[test]
    fn test_is_newer() {
        assert!(is_newer("1.2.4", "1.2.3").expect("valid"));
        assert!(is_newer("2.0.0", "1.9.9").expect("valid"));
        assert!(!is_newer("1.2.3", "1.2.3").expect("valid"));
        assert!(!is_newer("1.2.2", "1.2.3").expect("valid"));
    }

    #[test]
    fn test_invalid_version() {
        assert!(parse_version("not-a-version").is_err());
        assert!(parse_version("1.2").is_err());
    }

    #[test]
    fn test_parse_github_release_response() {
        let target = super::current_platform_target();
        let json = format!(
            r#"{{"tag_name":"v1.5.0","assets":[{{"name":"tok0-{}","browser_download_url":"https://github.com/prxm-labs/tok0/releases/download/v1.5.0/tok0-{}"}}]}}"#,
            target, target
        );
        let release = parse_release_json(&json).expect("valid json");
        assert_eq!(release.version, "1.5.0");
        assert!(!release.asset_url.is_empty());
    }

    #[test]
    fn test_should_check_respects_stamp() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let stamp = tmp.path().join("last_update_check");
        // No stamp → should check
        assert!(should_check_with_stamp(&stamp).expect("check"));
        // Create fresh stamp → should NOT check
        std::fs::write(&stamp, "").expect("write stamp");
        assert!(!should_check_with_stamp(&stamp).expect("check"));
    }

    #[test]
    fn test_verify_sha256_match() {
        use sha2::{Digest, Sha256};
        let data = b"fake binary data";
        let actual = format!("{:x}", Sha256::digest(data));
        assert_eq!(actual.len(), 64);
        assert!(verify_sha256(data, &actual).is_ok());
    }

    #[test]
    fn test_verify_sha256_mismatch() {
        let data = b"fake binary data";
        let wrong = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(verify_sha256(data, wrong).is_err());
    }

    #[test]
    fn test_replace_binary_with_rollback() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let current = tmp.path().join("tok0");
        let new_bin = tmp.path().join("tok0.new");
        std::fs::write(&current, b"old binary").expect("write current");
        std::fs::write(&new_bin, b"new binary").expect("write new");
        replace_binary_with_rollback(&new_bin, &current).expect("replace");
        assert_eq!(
            std::fs::read_to_string(&current).expect("read"),
            "new binary"
        );
        assert!(
            !tmp.path().join("tok0.bak").exists(),
            "backup should be cleaned up"
        );
    }

    #[test]
    fn test_replace_binary_rollback_on_failure() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let current = tmp.path().join("tok0");
        let nonexistent = tmp.path().join("does-not-exist");
        std::fs::write(&current, b"original").expect("write current");
        // Trying to rename a nonexistent file should fail and rollback
        let result = replace_binary_with_rollback(&nonexistent, &current);
        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(&current).expect("read"), "original");
    }

    #[test]
    fn test_format_update_hint() {
        let hint = format_update_hint("1.5.0");
        assert!(hint.contains("1.5.0"));
        assert!(hint.contains("tok0 update"));
    }

    #[test]
    fn test_preflight_accepts_writable_parent() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let fake_binary = tmp.path().join("tok0");
        preflight_write_access(&fake_binary).expect("writable temp dir must pass preflight");
    }

    #[cfg(unix)]
    #[test]
    fn test_preflight_rejects_readonly_parent_with_actionable_hint() {
        use std::fs::Permissions;
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::TempDir::new().expect("temp dir");
        // Strip write bits from the parent — `tok0.tmp` cannot be created.
        std::fs::set_permissions(tmp.path(), Permissions::from_mode(0o555)).expect("chmod 0555");

        // Self-detect root: if we can still create a file in the dir
        // despite chmod 0o555, we have CAP_DAC_OVERRIDE / euid==0 and
        // the negative test cannot be set up. Skip cleanly.
        let probe = tmp.path().join(".root-detect");
        if std::fs::File::create(&probe).is_ok() {
            let _ = std::fs::remove_file(&probe);
            let _ = std::fs::set_permissions(tmp.path(), Permissions::from_mode(0o755));
            eprintln!("skipping: euid bypasses chmod (likely root)");
            return;
        }

        let fake_binary = tmp.path().join("tok0");
        let result = preflight_write_access(&fake_binary);

        // Restore writable so TempDir::drop can clean up.
        std::fs::set_permissions(tmp.path(), Permissions::from_mode(0o755)).expect("restore 0755");

        let err = result.expect_err("readonly parent must fail preflight");
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("permission denied") || msg.contains("Permission denied"),
            "error must mention permission denied: {}",
            msg
        );
        assert!(
            msg.contains("sudo tok0 update"),
            "error must point at the sudo recovery path: {}",
            msg
        );
        assert!(
            msg.contains("TOK0_INSTALL_DIR"),
            "error must point at the user-local-install recovery path: {}",
            msg
        );
    }
}
