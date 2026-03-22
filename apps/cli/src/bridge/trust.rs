use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Path to the trust marker file within a project directory.
fn marker_path(project_dir: &Path) -> std::path::PathBuf {
    project_dir.join(".tok0").join(".trusted")
}

/// Returns true if the project directory has been explicitly trusted.
#[allow(dead_code)]
pub fn is_trusted(project_dir: &Path) -> Result<bool> {
    let marker = marker_path(project_dir);
    Ok(marker.exists())
}

/// Mark a project directory as trusted by creating the `.tok0/.trusted` marker.
/// Idempotent — calling on an already-trusted project is a no-op.
pub fn trust_project(project_dir: &Path) -> Result<()> {
    let tok0_dir = project_dir.join(".tok0");
    fs::create_dir_all(&tok0_dir).with_context(|| {
        format!(
            "Failed to create .tok0 directory in {}",
            project_dir.display()
        )
    })?;
    let marker = tok0_dir.join(".trusted");
    if !marker.exists() {
        fs::write(&marker, b"")
            .with_context(|| format!("Failed to create trust marker at {}", marker.display()))?;
    }
    Ok(())
}

/// Remove the trust marker from a project directory.
/// Idempotent — calling on an untrusted project is a no-op.
pub fn untrust_project(project_dir: &Path) -> Result<()> {
    let marker = marker_path(project_dir);
    if marker.exists() {
        fs::remove_file(&marker)
            .with_context(|| format!("Failed to remove trust marker at {}", marker.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp dir")
    }

    #[test]
    fn untrusted_by_default() {
        let dir = tmp();
        assert!(!is_trusted(dir.path()).expect("is_trusted failed"));
    }

    #[test]
    fn trust_creates_marker() {
        let dir = tmp();
        trust_project(dir.path()).expect("trust_project failed");
        assert!(is_trusted(dir.path()).expect("is_trusted failed"));
        assert!(dir.path().join(".tok0").join(".trusted").exists());
    }

    #[test]
    fn untrust_removes_marker() {
        let dir = tmp();
        trust_project(dir.path()).expect("trust_project failed");
        untrust_project(dir.path()).expect("untrust_project failed");
        assert!(!is_trusted(dir.path()).expect("is_trusted failed"));
    }

    #[test]
    fn double_trust_is_idempotent() {
        let dir = tmp();
        trust_project(dir.path()).expect("first trust failed");
        trust_project(dir.path()).expect("second trust failed");
        assert!(is_trusted(dir.path()).expect("is_trusted failed"));
    }

    #[test]
    fn double_untrust_is_idempotent() {
        let dir = tmp();
        // Never trusted — first untrust should be a no-op.
        untrust_project(dir.path()).expect("first untrust failed");
        // Trust then untrust twice.
        trust_project(dir.path()).expect("trust failed");
        untrust_project(dir.path()).expect("second untrust failed");
        untrust_project(dir.path()).expect("third untrust failed");
        assert!(!is_trusted(dir.path()).expect("is_trusted failed"));
    }

    #[test]
    fn is_trusted_false_after_untrust() {
        let dir = tmp();
        trust_project(dir.path()).expect("trust failed");
        assert!(is_trusted(dir.path()).expect("is_trusted before untrust failed"));
        untrust_project(dir.path()).expect("untrust failed");
        assert!(!is_trusted(dir.path()).expect("is_trusted after untrust failed"));
    }
}
