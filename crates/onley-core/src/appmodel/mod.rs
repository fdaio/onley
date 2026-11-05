//! The desktop client's decisions, without a UI.

mod policy;
mod roots;
mod speed;

pub use policy::{KeepPolicy, choose_keep, parse_keep_policy};
pub use roots::{clean, merge_roots};
pub use speed::{SpeedSample, gauge_max, rate};

use crate::db::{DuplicateGroup, FileRecord};
use crate::selfpath;
use std::collections::HashSet;
use std::path::Path;

/// Describes the duplicate index.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    pub groups: usize,
    pub duplicate_files: usize,
    pub wasted: i64,
}

/// Counts duplicate groups, files and the space that deleting all but one
/// copy per group would reclaim.
pub fn summarize(groups: &[DuplicateGroup]) -> Summary {
    let mut summary = Summary {
        groups: groups.len(),
        ..Summary::default()
    };
    for group in groups {
        summary.duplicate_files += group.files.len();
        if group.files.len() > 1 {
            summary.wasted += group.size * (group.files.len() as i64 - 1);
        }
    }
    summary
}

/// The files a delete would remove and the copies it would keep.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub targets: Vec<FileRecord>,
    pub keeps: Vec<FileRecord>,
    pub bytes: i64,
    pub groups: usize,
}

/// Returns the file in `group` that matches `protect`.
pub fn protected_file(group: &DuplicateGroup, protect: Option<&Path>) -> Option<FileRecord> {
    let protect = protect?;
    group
        .files
        .iter()
        .find(|file| selfpath::same(&file.path, protect))
        .cloned()
}

/// Keeps `keep[group.md5]` in each group and targets every other file.
///
/// A group with no valid choice is skipped. A group that holds the running
/// executable must keep it. Each path is targeted once.
pub fn build_plan(
    groups: &[DuplicateGroup],
    keep: &std::collections::HashMap<String, String>,
    protect: Option<&Path>,
) -> Plan {
    let mut plan = Plan::default();
    let mut seen: HashSet<String> = HashSet::new();
    for group in groups {
        let Some(keep_path) = keep.get(&group.md5) else {
            continue;
        };
        if let Some(protected) = protected_file(group, protect)
            && protected.path.to_string_lossy() != *keep_path
        {
            continue;
        }
        let found = group
            .files
            .iter()
            .any(|file| file.path.to_string_lossy() == *keep_path);
        if !found {
            continue;
        }
        plan.groups += 1;
        for file in &group.files {
            if file.path.to_string_lossy() == *keep_path {
                plan.keeps.push(file.clone());
                continue;
            }
            let key = file.path.to_string_lossy().into_owned();
            if !seen.insert(key) {
                continue;
            }
            plan.targets.push(file.clone());
            plan.bytes += file.size;
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::FileRecord;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn file(path: &str, size: i64, md5: &str, mtime: i64) -> FileRecord {
        FileRecord::new(PathBuf::from(path), size, md5, mtime)
    }

    fn group(md5: &str, size: i64, files: Vec<FileRecord>) -> DuplicateGroup {
        DuplicateGroup {
            md5: md5.into(),
            size,
            files,
        }
    }

    #[test]
    fn summarize_counts_groups_files_and_wasted() {
        let groups = vec![
            group(
                "a",
                10,
                vec![file("/a", 10, "a", 0), file("/b", 10, "a", 0)],
            ),
            group(
                "c",
                5,
                vec![
                    file("/c", 5, "c", 0),
                    file("/d", 5, "c", 0),
                    file("/e", 5, "c", 0),
                ],
            ),
        ];
        let summary = summarize(&groups);
        assert_eq!(summary.groups, 2);
        assert_eq!(summary.duplicate_files, 5);
        assert_eq!(summary.wasted, 10 + 10);
    }

    #[test]
    fn build_plan_keeps_the_chosen_copy() {
        let groups = vec![group(
            "a",
            10,
            vec![file("/a", 10, "a", 0), file("/b", 10, "a", 0)],
        )];
        let mut keep = HashMap::new();
        keep.insert("a".to_string(), "/a".to_string());
        let plan = build_plan(&groups, &keep, None);
        assert_eq!(plan.targets.len(), 1);
        assert_eq!(plan.targets[0].path, PathBuf::from("/b"));
        assert_eq!(plan.keeps[0].path, PathBuf::from("/a"));
        assert_eq!(plan.bytes, 10);
        assert_eq!(plan.groups, 1);
    }

    #[test]
    fn a_group_without_a_choice_is_skipped() {
        let groups = vec![group(
            "a",
            10,
            vec![file("/a", 10, "a", 0), file("/b", 10, "a", 0)],
        )];
        let plan = build_plan(&groups, &HashMap::new(), None);
        assert!(plan.targets.is_empty());
    }

    #[test]
    fn a_group_holding_the_executable_must_keep_it() {
        let groups = vec![group(
            "a",
            10,
            vec![file("/self", 10, "a", 0), file("/b", 10, "a", 0)],
        )];
        let mut keep = HashMap::new();
        keep.insert("a".to_string(), "/b".to_string());
        let plan = build_plan(&groups, &keep, Some(Path::new("/self")));
        assert!(plan.targets.is_empty());
        keep.insert("a".to_string(), "/self".to_string());
        let plan = build_plan(&groups, &keep, Some(Path::new("/self")));
        assert_eq!(plan.targets.len(), 1);
    }

    #[test]
    fn each_path_is_targeted_once() {
        let groups = vec![group(
            "a",
            10,
            vec![file("/a", 10, "a", 0), file("/b", 10, "a", 0)],
        )];
        let mut keep = HashMap::new();
        keep.insert("a".to_string(), "/a".to_string());
        let plan = build_plan(&groups, &keep, None);
        assert_eq!(plan.targets.len(), 1);
        assert_eq!(plan.bytes, 10);
    }
}
