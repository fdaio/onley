//! Identifies the running executable.

use std::path::{Path, PathBuf};

/// Returns the running executable's path with symlinks resolved.
pub fn resolve() -> Option<PathBuf> {
    let path = std::env::current_exe().ok()?;
    Some(std::fs::canonicalize(&path).unwrap_or(path))
}

/// Reports whether `a` and `b` name the same file.
pub fn same(a: &Path, b: &Path) -> bool {
    if a.as_os_str().is_empty() || b.as_os_str().is_empty() {
        return false;
    }
    if a == b {
        return true;
    }
    same_file::is_same_file(a, b).unwrap_or(false)
}

/// Reports whether `path` is the running executable.
pub fn is_self(path: &Path) -> bool {
    resolve().is_some_and(|me| same(&me, path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn the_running_executable_resolves() {
        let me = resolve().expect("current executable");
        assert!(me.is_absolute());
        assert!(same(&me, &me));
        assert!(is_self(&me));
    }

    #[test]
    fn different_files_differ() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::write(&a, b"a").unwrap();
        fs::write(&b, b"b").unwrap();
        assert!(!same(&a, &b));
        assert!(!is_self(&a));
    }
}
