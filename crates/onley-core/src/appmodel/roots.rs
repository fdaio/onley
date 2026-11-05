//! Normalises the folders a scan covers.

use std::path::{Component, Path, PathBuf};

/// Cleans the roots and removes any root covered by another.
///
/// A parent keeps its children out of the scan, equal paths collapse, and the
/// result is sorted so the scan order is stable.
pub fn merge_roots(roots: &[String]) -> Vec<String> {
    let mut out: Vec<PathBuf> = Vec::new();
    for root in roots {
        if root.is_empty() {
            continue;
        }
        let root = clean(Path::new(root));
        if out.iter().any(|existing| within(existing, &root)) {
            continue;
        }
        out.retain(|existing| !within(&root, existing));
        out.push(root);
    }
    out.sort();
    out.into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

/// `Path::components` normalisation without touching the file system.
pub fn clean(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    if out.as_os_str().is_empty() {
        out.push(".");
    }
    out
}

/// Reports whether `path` is `parent` or a descendant of it.
fn within(parent: &Path, path: &Path) -> bool {
    path == parent
        || path
            .strip_prefix(parent)
            .is_ok_and(|rest| !rest.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_parent_covers_its_child() {
        assert_eq!(
            merge_roots(&["/a/b".into(), "/a".into()]),
            vec!["/a".to_string()]
        );
        assert_eq!(
            merge_roots(&["/a".into(), "/a/b".into()]),
            vec!["/a".to_string()]
        );
    }

    #[test]
    fn equal_paths_collapse() {
        assert_eq!(
            merge_roots(&["/a".into(), "/a".into()]),
            vec!["/a".to_string()]
        );
    }

    #[test]
    fn the_result_is_sorted() {
        assert_eq!(
            merge_roots(&["/z".into(), "/a".into(), "/m".into()]),
            vec!["/a".to_string(), "/m".to_string(), "/z".to_string()]
        );
    }

    #[test]
    fn empty_entries_are_ignored() {
        assert_eq!(
            merge_roots(&["".into(), "/a".into()]),
            vec!["/a".to_string()]
        );
        assert!(merge_roots(&[]).is_empty());
    }

    #[test]
    fn clean_normalises_dots() {
        assert_eq!(clean(Path::new("/a/./b/../c")), PathBuf::from("/a/c"));
        assert_eq!(clean(Path::new("")), PathBuf::from("."));
    }
}
