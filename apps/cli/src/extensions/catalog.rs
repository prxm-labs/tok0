use anyhow::{Context, Result};
use std::path::PathBuf;

/// Returns the extensions directory, respecting TOK0_CONFIG_DIR env override.
pub fn extensions_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TOK0_CONFIG_DIR") {
        return PathBuf::from(dir).join("extensions");
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("tok0")
        .join("extensions")
}

/// List all installed extension names.
pub fn list_installed() -> Result<Vec<String>> {
    let dir = extensions_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }
    let entries = std::fs::read_dir(&dir)
        .with_context(|| format!("Failed to read extensions dir: {}", dir.display()))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.context("Failed to read dir entry")?;
        if entry
            .file_type()
            .context("Failed to get file type")?
            .is_dir()
        {
            names.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    Ok(names)
}

/// Validate extension name: alphanumeric, dash, and underscore only.
fn validate_extension_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Extension name cannot be empty");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        anyhow::bail!(
            "Invalid extension name '{}': must be alphanumeric, dash, or underscore only",
            name
        );
    }
    Ok(())
}

/// Validate that a string looks like a git commit SHA: 7-40 lowercase
/// hex characters. Anything else is rejected before we hand it to
/// `git checkout` (defense in depth — git itself would reject most of
/// these too, but we catch them earlier with a clearer error).
fn validate_commit_sha(sha: &str) -> Result<()> {
    let len = sha.len();
    if !(7..=40).contains(&len) {
        anyhow::bail!("Invalid commit SHA '{}': must be 7-40 hex characters", sha);
    }
    if !sha
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        anyhow::bail!(
            "Invalid commit SHA '{}': must be lowercase hex (a-f, 0-9)",
            sha
        );
    }
    Ok(())
}

/// Install an extension from a git URL.
///
/// `pin` (ROADMAP T1.3): if `Some(sha)`, the working tree is checked
/// out at that commit after clone. Without a pin, the extension tracks
/// the remote's default branch tip — a malicious extension author can
/// silently change behaviour at any time. With a pin, `tok0 ext list`
/// users always get the exact bytes they audited.
pub fn install_from_url(url: &str, name: &str, pin: Option<&str>) -> Result<()> {
    validate_extension_name(name)?;
    if let Some(sha) = pin {
        validate_commit_sha(sha)?;
    }
    let dest = extensions_dir().join(name);
    if dest.exists() {
        anyhow::bail!(
            "Extension '{}' is already installed at {}",
            name,
            dest.display()
        );
    }
    std::fs::create_dir_all(&dest)
        .with_context(|| format!("Failed to create extension dir: {}", dest.display()))?;

    if !(url.ends_with(".git") || url.starts_with("https://github.com")) {
        anyhow::bail!("Unsupported extension source: {} (must be a git URL)", url);
    }

    // For pinned installs we need full history — `--depth=1` would refuse
    // to check out an arbitrary SHA. Unpinned installs stay shallow.
    let mut clone_args = vec!["clone"];
    if pin.is_none() {
        clone_args.push("--depth=1");
    }
    clone_args.push(url);
    let dest_str = dest.to_str().unwrap_or_default();
    clone_args.push(dest_str);

    let status = std::process::Command::new("git")
        .args(&clone_args)
        .status()
        .context("Failed to run git clone")?;
    if !status.success() {
        let _ = std::fs::remove_dir_all(&dest);
        anyhow::bail!("git clone failed for {}", url);
    }

    if let Some(sha) = pin {
        let checkout = std::process::Command::new("git")
            .args(["-C", dest_str, "checkout", "--quiet", sha])
            .status()
            .context("Failed to run git checkout for the pinned commit")?;
        if !checkout.success() {
            let _ = std::fs::remove_dir_all(&dest);
            anyhow::bail!("Pinned commit '{}' not found in {}", sha, url);
        }
        println!(
            "Installed extension '{}' pinned to {} -> {}",
            name,
            sha,
            dest.display()
        );
    } else {
        println!("Installed extension '{}' -> {}", name, dest.display());
    }
    Ok(())
}

/// Remove an installed extension.
pub fn remove_extension(name: &str) -> Result<()> {
    validate_extension_name(name)?;
    let dest = extensions_dir().join(name);
    if !dest.exists() {
        anyhow::bail!("Extension '{}' is not installed", name);
    }
    std::fs::remove_dir_all(&dest)
        .with_context(|| format!("Failed to remove extension dir: {}", dest.display()))?;
    println!("Removed extension '{}'", name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper: list extensions from a specific directory.
    fn list_from_dir(dir: &std::path::Path) -> Result<Vec<String>> {
        if !dir.exists() {
            return Ok(vec![]);
        }
        let entries = std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read dir: {}", dir.display()))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.context("read entry")?;
            if entry.file_type().context("file type")?.is_dir() {
                names.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(names)
    }

    #[test]
    fn test_list_empty_when_no_extensions() {
        let tmp = TempDir::new().expect("tmp dir");
        let ext_dir = tmp.path().join("extensions");
        let result = list_from_dir(&ext_dir).expect("list");
        assert!(result.is_empty());
    }

    #[test]
    fn test_list_finds_installed_extension() {
        let tmp = TempDir::new().expect("tmp dir");
        let ext_dir = tmp.path().join("extensions");
        std::fs::create_dir_all(ext_dir.join("my-pack")).expect("create dir");
        let result = list_from_dir(&ext_dir).expect("list");
        assert_eq!(result, vec!["my-pack"]);
    }

    #[test]
    fn test_remove_missing_extension_errors() {
        let tmp = TempDir::new().expect("tmp dir");
        unsafe {
            std::env::set_var("TOK0_CONFIG_DIR", tmp.path().to_str().unwrap());
        }
        let result = remove_extension("nonexistent");
        unsafe {
            std::env::remove_var("TOK0_CONFIG_DIR");
        }
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_installed_extension() {
        let tmp = TempDir::new().expect("tmp dir");
        let ext_dir = tmp.path().join("extensions");
        std::fs::create_dir_all(ext_dir.join("removable")).expect("create dir");
        unsafe {
            std::env::set_var("TOK0_CONFIG_DIR", tmp.path().to_str().unwrap());
        }
        let result = remove_extension("removable");
        unsafe {
            std::env::remove_var("TOK0_CONFIG_DIR");
        }
        assert!(result.is_ok());
        assert!(!ext_dir.join("removable").exists());
    }

    #[test]
    fn test_validate_extension_name_valid() {
        assert!(validate_extension_name("my-extension").is_ok());
        assert!(validate_extension_name("my_extension").is_ok());
        assert!(validate_extension_name("ext123").is_ok());
        assert!(validate_extension_name("a").is_ok());
    }

    #[test]
    fn test_validate_extension_name_rejects_traversal() {
        assert!(validate_extension_name("../../../tmp/evil").is_err());
        assert!(validate_extension_name("foo/bar").is_err());
        assert!(validate_extension_name("foo..bar").is_err());
        assert!(validate_extension_name("").is_err());
        assert!(validate_extension_name("foo bar").is_err());
    }

    #[test]
    fn test_install_unsupported_source_errors() {
        let tmp = TempDir::new().expect("tmp dir");
        unsafe {
            std::env::set_var("TOK0_CONFIG_DIR", tmp.path().to_str().unwrap());
        }
        let result = install_from_url("ftp://example.com/archive.tar.gz", "bad-source", None);
        unsafe {
            std::env::remove_var("TOK0_CONFIG_DIR");
        }
        assert!(result.is_err());
    }

    #[test]
    fn test_install_duplicate_errors() {
        let tmp = TempDir::new().expect("tmp dir");
        let ext_dir = tmp.path().join("extensions");
        std::fs::create_dir_all(ext_dir.join("existing")).expect("create dir");
        unsafe {
            std::env::set_var("TOK0_CONFIG_DIR", tmp.path().to_str().unwrap());
        }
        let result = install_from_url("https://github.com/test/repo.git", "existing", None);
        unsafe {
            std::env::remove_var("TOK0_CONFIG_DIR");
        }
        assert!(result.is_err());
    }

    // ROADMAP T1.3 — commit-pin validation.
    #[test]
    fn test_validate_commit_sha_accepts_short_and_full() {
        assert!(validate_commit_sha("abc1234").is_ok()); // 7
        assert!(validate_commit_sha("a1b2c3d4e5").is_ok()); // 10
        assert!(validate_commit_sha(&"a".repeat(40)).is_ok()); // 40
    }

    #[test]
    fn test_validate_commit_sha_rejects_bad_input() {
        assert!(validate_commit_sha("").is_err());
        assert!(validate_commit_sha("abc").is_err()); // < 7
        assert!(validate_commit_sha(&"a".repeat(41)).is_err()); // > 40
        assert!(validate_commit_sha("abc123g").is_err()); // non-hex
        assert!(validate_commit_sha("ABC1234").is_err()); // uppercase
        assert!(validate_commit_sha("abc 123").is_err()); // whitespace
        assert!(validate_commit_sha("abc;rm -rf").is_err()); // shell metachar
    }

    #[test]
    fn test_install_pin_rejects_invalid_sha() {
        let tmp = TempDir::new().expect("tmp dir");
        unsafe {
            std::env::set_var("TOK0_CONFIG_DIR", tmp.path().to_str().unwrap());
        }
        let result = install_from_url(
            "https://github.com/prxm-labs/tok0.git",
            "pinned-bad",
            Some("not-a-real-sha"),
        );
        unsafe {
            std::env::remove_var("TOK0_CONFIG_DIR");
        }
        assert!(
            result.is_err(),
            "install must reject a malformed commit SHA before invoking git"
        );
    }

    #[test]
    fn test_install_pin_rejects_shell_metacharacters() {
        // Defense in depth: if validation slipped, git would still see
        // the bad arg, but we don't want it to even get there.
        let tmp = TempDir::new().expect("tmp dir");
        unsafe {
            std::env::set_var("TOK0_CONFIG_DIR", tmp.path().to_str().unwrap());
        }
        let result = install_from_url(
            "https://github.com/prxm-labs/tok0.git",
            "pinned-evil",
            Some("abc1234; rm -rf /"),
        );
        unsafe {
            std::env::remove_var("TOK0_CONFIG_DIR");
        }
        assert!(result.is_err());
    }
}
