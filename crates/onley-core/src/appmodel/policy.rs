//! Chooses which copy of a duplicate group to keep.

use crate::db::{DuplicateGroup, FileRecord};

/// Chooses which copy of a duplicate group to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeepPolicy {
    Newest,
    #[default]
    Oldest,
    ByPath,
}

impl KeepPolicy {
    /// Returns the storage value for the policy.
    pub fn as_str(self) -> &'static str {
        match self {
            KeepPolicy::Newest => "newest",
            KeepPolicy::Oldest => "oldest",
            KeepPolicy::ByPath => "path",
        }
    }
}

impl std::fmt::Display for KeepPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Parses a stored policy. An unknown value falls back to the default.
pub fn parse_keep_policy(value: &str) -> (KeepPolicy, bool) {
    match value {
        "newest" => (KeepPolicy::Newest, true),
        "oldest" => (KeepPolicy::Oldest, true),
        "path" => (KeepPolicy::ByPath, true),
        _ => (KeepPolicy::default(), false),
    }
}

/// Returns the copy the policy keeps from `group`.
///
/// Modification time decides, and the path breaks a tie, so the choice is
/// stable across runs.
pub fn choose_keep(group: &DuplicateGroup, policy: KeepPolicy) -> Option<FileRecord> {
    let mut files = group.files.iter();
    let mut chosen = files.next()?.clone();
    for file in files {
        let better = match policy {
            KeepPolicy::Oldest => {
                file.mtime < chosen.mtime || (file.mtime == chosen.mtime && file.path < chosen.path)
            }
            KeepPolicy::ByPath => file.path < chosen.path,
            KeepPolicy::Newest => {
                file.mtime > chosen.mtime || (file.mtime == chosen.mtime && file.path < chosen.path)
            }
        };
        if better {
            chosen = file.clone();
        }
    }
    Some(chosen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn group() -> DuplicateGroup {
        DuplicateGroup {
            md5: "x".into(),
            size: 1,
            files: vec![
                FileRecord::new(PathBuf::from("/b"), 1, "x", 20),
                FileRecord::new(PathBuf::from("/a"), 1, "x", 10),
                FileRecord::new(PathBuf::from("/c"), 1, "x", 30),
            ],
        }
    }

    #[test]
    fn newest_keeps_the_latest_mtime() {
        assert_eq!(
            choose_keep(&group(), KeepPolicy::Newest).unwrap().path,
            PathBuf::from("/c")
        );
    }

    #[test]
    fn oldest_keeps_the_earliest_mtime() {
        assert_eq!(
            choose_keep(&group(), KeepPolicy::Oldest).unwrap().path,
            PathBuf::from("/a")
        );
    }

    #[test]
    fn by_path_keeps_the_first_path() {
        assert_eq!(
            choose_keep(&group(), KeepPolicy::ByPath).unwrap().path,
            PathBuf::from("/a")
        );
    }

    #[test]
    fn a_tie_uses_the_path() {
        let mut group = group();
        for file in &mut group.files {
            file.mtime = 10;
        }
        assert_eq!(
            choose_keep(&group, KeepPolicy::Newest).unwrap().path,
            PathBuf::from("/a")
        );
        assert_eq!(
            choose_keep(&group, KeepPolicy::Oldest).unwrap().path,
            PathBuf::from("/a")
        );
    }

    #[test]
    fn policies_round_trip() {
        for policy in [KeepPolicy::Newest, KeepPolicy::Oldest, KeepPolicy::ByPath] {
            assert_eq!(parse_keep_policy(policy.as_str()), (policy, true));
        }
        assert_eq!(parse_keep_policy("other"), (KeepPolicy::Oldest, false));
    }
}
