//! Safety guards for delete operations.
//!
//! A frontend bug, IPC misuse, or a crafted/symlinked path could otherwise
//! cause an irreversible recursive deletion of a critical location such as
//! `/`, `$HOME`, or `/System`. Every delete boundary funnels through
//! [`validate_delete_target`] before any data is removed.

use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// Protected absolute roots that must never be deleted (and whose ancestors
/// must never be deleted either). These are macOS system and top-level user
/// locations. The user's own home directory is added dynamically in
/// [`validate_delete_target`] since it is not a fixed path.
const PROTECTED_ROOTS: &[&str] = &[
    "/",
    "/System",
    "/Library",
    "/Users",
    "/Applications",
    "/private",
    "/usr",
    "/bin",
    "/sbin",
    "/etc",
    "/var",
    "/opt",
    "/cores",
    "/Volumes",
];

/// Validate that `path` is safe to delete. Returns the canonicalized path on
/// success.
///
/// The validation:
/// - Rejects empty paths.
/// - Rejects non-existent paths (canonicalize fails for them).
/// - Resolves the real path via [`Path::canonicalize`], which follows symlinks
///   so a symlink that points at a protected directory is caught.
/// - Rejects the path if it **is** or **is an ancestor of** any protected root
///   (deleting it would destroy a protected location).
/// - Rejects the user's home directory itself, but allows subdirectories of it
///   (e.g. `~/Library/Caches/...`) so legitimate cache cleaners keep working.
/// - Rejects any filesystem/volume root (a path that is its own parent, or a
///   direct child of `/Volumes`).
pub fn validate_delete_target(path: &Path) -> Result<PathBuf> {
    if path.as_os_str().is_empty() {
        return Err(anyhow!("refusing to delete an empty path"));
    }

    // Resolve the real path. This follows symlinks, so a symlink that targets a
    // protected directory resolves to that directory and is rejected below. It
    // also fails for non-existent paths, which we treat as "nothing to delete".
    let canonical = path.canonicalize().map_err(|e| {
        anyhow!(
            "refusing to delete {}: cannot resolve real path: {}",
            path.display(),
            e
        )
    })?;

    // Reject any filesystem/volume root (a path equal to its own parent, e.g.
    // "/", or a mounted volume root like "/Volumes/<x>").
    match canonical.parent() {
        None => {
            return Err(anyhow!(
                "refusing to delete filesystem root {}",
                canonical.display()
            ));
        }
        Some(parent) => {
            if parent == canonical {
                return Err(anyhow!(
                    "refusing to delete filesystem root {}",
                    canonical.display()
                ));
            }
            if parent == Path::new("/Volumes") {
                return Err(anyhow!(
                    "refusing to delete volume root {}",
                    canonical.display()
                ));
            }
        }
    }

    // Build the full protected set: fixed roots plus the user's actual home.
    // The home directory is canonicalized so the comparison is apples-to-apples
    // (e.g. on macOS `/Users/x` is already canonical, but be defensive).
    let mut protected: Vec<PathBuf> = PROTECTED_ROOTS.iter().map(PathBuf::from).collect();
    if let Some(home) = dirs::home_dir() {
        let home = home.canonicalize().unwrap_or(home);
        protected.push(home);
    }

    for root in &protected {
        // Reject if the candidate *is* a protected root, or if deleting the
        // candidate would also delete a protected root (the candidate is an
        // ancestor of the root, i.e. root.starts_with(candidate)).
        if root.starts_with(&canonical) {
            return Err(anyhow!(
                "refusing to delete {}: it is or contains the protected path {}",
                canonical.display(),
                root.display()
            ));
        }
    }

    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn rejects_empty_path() {
        assert!(validate_delete_target(Path::new("")).is_err());
    }

    #[test]
    fn rejects_root() {
        assert!(validate_delete_target(Path::new("/")).is_err());
    }

    #[test]
    fn rejects_system() {
        // /System exists on macOS; if it doesn't (CI on Linux), canonicalize
        // fails and we still reject, so the assertion holds either way.
        assert!(validate_delete_target(Path::new("/System")).is_err());
    }

    #[test]
    fn rejects_library() {
        assert!(validate_delete_target(Path::new("/Library")).is_err());
    }

    #[test]
    fn rejects_home_directory() {
        if let Some(home) = dirs::home_dir() {
            assert!(
                validate_delete_target(&home).is_err(),
                "deleting the home directory must be rejected"
            );
        }
    }

    #[test]
    fn rejects_ancestor_of_protected_root() {
        // A path that *contains* a protected root must be rejected. Whatever
        // "/" canonicalizes to is an ancestor of every protected root.
        assert!(validate_delete_target(Path::new("/")).is_err());
    }

    #[test]
    fn rejects_symlink_to_home() {
        let dir = tempdir().unwrap();
        let link = dir.path().join("home_link");
        if let Some(home) = dirs::home_dir() {
            // Symlink pointing at the home directory: canonicalize resolves it
            // to home, which is protected.
            std::os::unix::fs::symlink(&home, &link).unwrap();
            assert!(
                validate_delete_target(&link).is_err(),
                "a symlink resolving to the home directory must be rejected"
            );
        }
    }

    #[test]
    fn accepts_normal_temp_subdir() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("cache");
        fs::create_dir(&target).unwrap();
        let result = validate_delete_target(&target);
        assert!(
            result.is_ok(),
            "a normal temp subdirectory must be accepted, got {:?}",
            result
        );
    }

    #[test]
    fn rejects_nonexistent_path() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        assert!(validate_delete_target(&missing).is_err());
    }
}
