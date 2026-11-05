//! Removes files and their index records.

use crate::db::Db;
use crate::selfpath;
use std::io;
use std::path::{Path, PathBuf};

/// An attempt to delete the running executable.
#[derive(Debug)]
pub enum DeleteError {
    SelfDelete,
    NotFound,
    NotRegularFile,
    NoMtime,
    Changed,
    Io(io::Error),
}

impl std::fmt::Display for DeleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteError::SelfDelete => f.write_str("cannot delete the running executable"),
            DeleteError::NotFound => f.write_str("file is gone"),
            DeleteError::NotRegularFile => f.write_str("not a regular file"),
            DeleteError::NoMtime => f.write_str("index has no modification time; rescan first"),
            DeleteError::Changed => f.write_str("changed since scan"),
            DeleteError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for DeleteError {}

impl From<io::Error> for DeleteError {
    fn from(err: io::Error) -> Self {
        if err.kind() == io::ErrorKind::NotFound {
            DeleteError::NotFound
        } else {
            DeleteError::Io(err)
        }
    }
}

/// One file to delete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub path: PathBuf,
    pub size: i64,
    pub mtime: i64,
}

impl Target {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            size: 0,
            mtime: 0,
        }
    }
}

/// What happened to one target.
#[derive(Debug)]
pub struct Result {
    pub path: PathBuf,
    /// The file was removed from disk.
    pub deleted: bool,
    /// Verification found the file already gone. Its record is removed.
    pub missing: bool,
    /// A file-system failure. The record is kept.
    pub error: Option<DeleteError>,
    /// A failure to remove the record after the file was deleted.
    pub index_error: Option<rusqlite::Error>,
}

/// Reports whether `target` still matches the index.
pub fn verify(target: &Target) -> std::result::Result<(), DeleteError> {
    let metadata = std::fs::symlink_metadata(&target.path)?;
    if !metadata.is_file() {
        return Err(DeleteError::NotRegularFile);
    }
    if target.mtime == 0 {
        return Err(DeleteError::NoMtime);
    }
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    if metadata.len() as i64 != target.size || mtime != target.mtime {
        return Err(DeleteError::Changed);
    }
    Ok(())
}

/// Removes each target and then its record.
///
/// Verification refuses a file whose size or modification time no longer
/// matches the index, and drops the record of a file that is already gone.
/// A failure on one target does not stop the others.
pub fn delete(db: &Db, targets: &[Target], verify_first: bool) -> Vec<Result> {
    delete_with_protect(db, targets, verify_first, selfpath::resolve().as_deref())
}

/// [`delete`] with an explicit executable path to protect.
pub fn delete_with_protect(
    db: &Db,
    targets: &[Target],
    verify_first: bool,
    protect: Option<&Path>,
) -> Vec<Result> {
    let mut results = Vec::with_capacity(targets.len());
    for target in targets {
        let mut result = Result {
            path: target.path.clone(),
            deleted: false,
            missing: false,
            error: None,
            index_error: None,
        };

        if let Some(protect) = protect
            && selfpath::same(&target.path, protect)
        {
            result.error = Some(DeleteError::SelfDelete);
            results.push(result);
            continue;
        }

        if verify_first && let Err(err) = verify(target) {
            if matches!(err, DeleteError::NotFound) {
                result.missing = true;
                result.index_error = db.delete_record(&target.path).err();
            } else {
                result.error = Some(err);
            }
            results.push(result);
            continue;
        }

        if let Err(err) = std::fs::remove_file(&target.path) {
            result.error = Some(DeleteError::from(err));
            results.push(result);
            continue;
        }
        result.deleted = true;
        result.index_error = db.delete_record(&target.path).err();
        results.push(result);
    }
    results
}

/// Removes the directories that deleting the given files left empty.
///
/// It walks up from each file's directory and stops at the scan root that
/// contains it, so a root is never removed. A symlink is never followed.
pub fn remove_empty_parents(deleted: &[PathBuf], roots: &[String]) -> Vec<PathBuf> {
    let roots: Vec<PathBuf> = roots
        .iter()
        .filter(|root| !root.is_empty())
        .map(|root| crate::appmodel::clean(Path::new(root)))
        .collect();
    if roots.is_empty() {
        return Vec::new();
    }

    let mut seen = std::collections::HashSet::new();
    let mut dirs = Vec::new();
    for path in deleted {
        if path.as_os_str().is_empty() {
            continue;
        }
        let dir = crate::appmodel::clean(path).parent().map(Path::to_path_buf);
        if let Some(dir) = dir
            && seen.insert(dir.clone())
        {
            dirs.push(dir);
        }
    }

    let mut removed = Vec::new();
    for mut dir in dirs {
        while let Some(root) = containing_root(&dir, &roots) {
            if !within(root, &dir) || !plain_dir(&dir, root) {
                break;
            }
            if std::fs::remove_dir(&dir).is_err() {
                break;
            }
            removed.push(dir.clone());
            match dir.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => dir = parent.to_path_buf(),
                _ => break,
            }
        }
    }
    removed
}

fn containing_root<'a>(dir: &Path, roots: &'a [PathBuf]) -> Option<&'a PathBuf> {
    roots.iter().find(|root| within(root, dir))
}

/// Reports whether `path` is strictly inside `root`.
fn within(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .is_ok_and(|rest| !rest.as_os_str().is_empty())
}

/// Reports whether `dir` is a real directory reached from `root` without a
/// symlink on the way.
fn plain_dir(dir: &Path, root: &Path) -> bool {
    let Ok(metadata) = std::fs::symlink_metadata(dir) else {
        return false;
    };
    if !metadata.is_dir() {
        return false;
    }
    let Ok(relative) = dir.strip_prefix(root) else {
        return false;
    };
    let (Ok(resolved_root), Ok(resolved_dir)) =
        (std::fs::canonicalize(root), std::fs::canonicalize(dir))
    else {
        return false;
    };
    resolved_dir == resolved_root.join(relative)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn deletes_a_file_and_its_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        fs::write(&path, b"x").unwrap();
        let db = Db::open_in_memory().unwrap();
        db.upsert(&crate::db::FileRecord::new(&path, 1, "x", 0))
            .unwrap();
        let results = delete_with_protect(&db, &[Target::new(&path)], false, None);
        assert!(results[0].deleted);
        assert!(!path.exists());
        assert!(db.lookup(&path).unwrap().is_none());
    }

    #[test]
    fn verify_refuses_a_changed_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        fs::write(&path, b"x").unwrap();
        let metadata = fs::metadata(&path).unwrap();
        let target = Target {
            path: path.clone(),
            size: metadata.len() as i64 + 1,
            mtime: 1,
        };
        let db = Db::open_in_memory().unwrap();
        let results = delete_with_protect(&db, &[target], true, None);
        assert!(matches!(results[0].error, Some(DeleteError::Changed)));
        assert!(path.exists());
    }

    #[test]
    fn a_missing_file_drops_its_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        let db = Db::open_in_memory().unwrap();
        db.upsert(&crate::db::FileRecord::new(&path, 1, "x", 1))
            .unwrap();
        let target = Target {
            path: path.clone(),
            size: 1,
            mtime: 1,
        };
        let results = delete_with_protect(&db, &[target], true, None);
        assert!(results[0].missing);
        assert!(db.lookup(&path).unwrap().is_none());
    }

    #[test]
    fn the_running_executable_is_protected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("self");
        fs::write(&path, b"x").unwrap();
        let db = Db::open_in_memory().unwrap();
        let results = delete_with_protect(&db, &[Target::new(&path)], false, Some(&path));
        assert!(matches!(results[0].error, Some(DeleteError::SelfDelete)));
        assert!(path.exists());
    }

    #[test]
    fn empty_parents_are_removed_up_to_the_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let sub = root.join("a/b");
        fs::create_dir_all(&sub).unwrap();
        let file = sub.join("x.txt");
        fs::write(&file, b"x").unwrap();
        fs::remove_file(&file).unwrap();
        let removed = remove_empty_parents(&[file], &[root.to_string_lossy().into_owned()]);
        assert_eq!(removed.len(), 2);
        assert!(root.exists());
        assert!(!sub.exists());
    }

    #[test]
    fn a_folder_that_still_holds_files_stays() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("gone.txt");
        fs::write(&file, b"x").unwrap();
        fs::write(root.join("here.txt"), b"x").unwrap();
        fs::remove_file(&file).unwrap();
        let removed = remove_empty_parents(&[file], &[root.to_string_lossy().into_owned()]);
        assert!(removed.is_empty());
        assert!(root.exists());
    }

    #[test]
    fn the_root_is_never_removed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("x.txt");
        fs::write(&file, b"x").unwrap();
        fs::remove_file(&file).unwrap();
        let removed = remove_empty_parents(&[file], &[root.to_string_lossy().into_owned()]);
        assert!(removed.is_empty());
        assert!(root.exists());
    }
}
