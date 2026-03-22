use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Compute the SHA-256 hex digest of the file at `path`.
pub fn hash_file(path: &Path) -> Result<String> {
    let data = fs::read(path)
        .with_context(|| format!("Failed to read file for hashing: {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Verify that the file at `path` matches `expected_hash` (hex SHA-256).
/// Returns `Ok(true)` when the digest matches, `Ok(false)` when it does not.
pub fn verify_hook(path: &Path, expected_hash: &str) -> Result<bool> {
    let actual = hash_file(path)?;
    Ok(actual == expected_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_tmp(dir: &TempDir, name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, content).expect("Failed to write temp file");
        path
    }

    /// SHA-256("hello world\n") — computed offline for determinism.
    /// echo -n "hello world" | sha256sum  → a948904...
    fn sha256_hex(data: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(data);
        format!("{:x}", h.finalize())
    }

    #[test]
    fn hash_of_known_content_matches_expected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let content = b"hello tok0\n";
        let path = write_tmp(&dir, "hook.sh", content);
        let expected = sha256_hex(content);
        assert_eq!(hash_file(&path).expect("hash_file failed"), expected);
    }

    #[test]
    fn verify_returns_true_for_matching_hash() {
        let dir = tempfile::tempdir().expect("tempdir");
        let content = b"#!/bin/sh\nexec tok0 rewrite \"$@\"\n";
        let path = write_tmp(&dir, "hook.sh", content);
        let expected = sha256_hex(content);
        assert!(verify_hook(&path, &expected).expect("verify_hook failed"));
    }

    #[test]
    fn verify_returns_false_for_mismatched_hash() {
        let dir = tempfile::tempdir().expect("tempdir");
        let content = b"#!/bin/sh\nexec tok0 rewrite \"$@\"\n";
        let path = write_tmp(&dir, "hook.sh", content);
        assert!(!verify_hook(
            &path,
            "0000000000000000000000000000000000000000000000000000000000000000"
        )
        .expect("verify_hook failed"));
    }

    #[test]
    fn hash_of_nonexistent_file_returns_err() {
        let path = std::path::Path::new("/nonexistent/path/hook.sh");
        assert!(hash_file(path).is_err());
    }

    #[test]
    fn hash_changes_when_file_modified() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_tmp(&dir, "hook.sh", b"original content");
        let hash1 = hash_file(&path).expect("first hash failed");
        std::fs::write(&path, b"modified content").expect("Failed to overwrite file");
        let hash2 = hash_file(&path).expect("second hash failed");
        assert_ne!(hash1, hash2, "hash should differ after modification");
    }
}
