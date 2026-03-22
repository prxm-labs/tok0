use crate::bridge::integrity;
use std::path::Path;

/// Result of a hook integrity check.
#[derive(Debug, PartialEq, Eq)]
pub enum VerifyResult {
    /// Hook file exists and its SHA-256 matches the expected hash.
    Ok,
    /// Hook file exists but its SHA-256 does not match — likely tampered.
    Tampered,
    /// Hook file does not exist at the given path.
    Missing,
}

/// Check the integrity of a hook file against an expected SHA-256 hex hash.
///
/// Returns:
/// - `VerifyResult::Missing`  — path does not exist
/// - `VerifyResult::Tampered` — path exists but hash mismatches
/// - `VerifyResult::Ok`       — path exists and hash matches
pub fn check_integrity(hook_path: &Path, expected_hash: &str) -> VerifyResult {
    if !hook_path.exists() {
        return VerifyResult::Missing;
    }
    match integrity::verify_hook(hook_path, expected_hash) {
        Ok(true) => VerifyResult::Ok,
        Ok(false) => VerifyResult::Tampered,
        Err(e) => {
            eprintln!("tok0: integrity check warning: {}", e);
            VerifyResult::Tampered
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use tempfile::TempDir;

    fn sha256_hex(data: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(data);
        format!("{:x}", h.finalize())
    }

    fn write_tmp(dir: &TempDir, name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, content).expect("Failed to write temp file");
        path
    }

    #[test]
    fn ok_for_matching_hash() {
        let dir = tempfile::tempdir().expect("tempdir");
        let content = b"#!/bin/sh\nexec tok0 rewrite \"$@\"\n";
        let path = write_tmp(&dir, "hook.sh", content);
        let expected = sha256_hex(content);
        assert_eq!(check_integrity(&path, &expected), VerifyResult::Ok);
    }

    #[test]
    fn tampered_for_hash_mismatch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let content = b"#!/bin/sh\nexec tok0 rewrite \"$@\"\n";
        let path = write_tmp(&dir, "hook.sh", content);
        assert_eq!(
            check_integrity(
                &path,
                "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
            ),
            VerifyResult::Tampered,
        );
    }

    #[test]
    fn missing_for_nonexistent_path() {
        let path = std::path::Path::new("/nonexistent/tok0_hook.sh");
        assert_eq!(check_integrity(path, "anyhash"), VerifyResult::Missing,);
    }
}
