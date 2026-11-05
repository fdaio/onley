//! Duplicate groups over photos, without a platform.

use std::collections::HashMap;

/// One photo the library exposes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub size: i64,
    pub modified: i64,
    pub width: u32,
    pub height: u32,
    pub hash: String,
}

/// Copies of one photo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub hash: String,
    pub assets: Vec<Asset>,
}

impl Group {
    /// Bytes freed when every copy but the kept one goes.
    pub fn reclaimable(&self) -> i64 {
        let kept = self
            .assets
            .iter()
            .map(|asset| asset.size)
            .min()
            .unwrap_or(0);
        self.assets.iter().map(|asset| asset.size).sum::<i64>() - kept
    }

    /// The copy the policy keeps.
    pub fn kept_id(&self, policy: KeepPolicy) -> Option<&str> {
        self.assets
            .iter()
            .max_by(|a, b| policy.order(a, b))
            .map(|asset| asset.id.as_str())
    }
}

/// Which copy of a group stays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeepPolicy {
    #[default]
    Newest,
    Oldest,
}

impl KeepPolicy {
    fn order(self, a: &Asset, b: &Asset) -> std::cmp::Ordering {
        match self {
            Self::Newest => key(a).cmp(&key(b)),
            Self::Oldest => key(b).cmp(&key(a)),
        }
    }
}

fn key(asset: &Asset) -> (i64, &str, &str) {
    (asset.modified, asset.name.as_str(), asset.id.as_str())
}

/// What the screen shows above the groups.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub groups: usize,
    pub copies: usize,
    pub reclaimable: i64,
}

/// Groups the assets by content hash, keeps groups with more than one copy,
/// and orders them by the space they can free.
pub fn group_duplicates(assets: Vec<Asset>) -> Vec<Group> {
    let mut by_hash: HashMap<String, Vec<Asset>> = HashMap::new();
    for asset in assets {
        by_hash.entry(asset.hash.clone()).or_default().push(asset);
    }
    let mut groups: Vec<Group> = by_hash
        .into_iter()
        .filter(|(_, assets)| assets.len() > 1)
        .map(|(hash, mut assets)| {
            assets.sort_by(|a, b| (b.modified, &b.name, &b.id).cmp(&(a.modified, &a.name, &a.id)));
            Group { hash, assets }
        })
        .collect();
    groups.sort_by(|a, b| {
        b.reclaimable()
            .cmp(&a.reclaimable())
            .then_with(|| a.hash.cmp(&b.hash))
    });
    groups
}

/// Counts the groups and the copies a cleanup removes.
pub fn summarize(groups: &[Group]) -> Summary {
    Summary {
        groups: groups.len(),
        copies: groups.iter().map(|group| group.assets.len() - 1).sum(),
        reclaimable: groups.iter().map(Group::reclaimable).sum(),
    }
}

/// Formats a byte count for a small screen.
pub fn bytes(value: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = value.max(0) as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit + 1 < UNITS.len() {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(id: &str, hash: &str, size: i64, modified: i64) -> Asset {
        Asset {
            id: id.to_owned(),
            name: format!("{id}.jpg"),
            size,
            modified,
            width: 100,
            height: 100,
            hash: hash.to_owned(),
        }
    }

    #[test]
    fn keeps_only_groups_with_more_than_one_copy() {
        let groups = group_duplicates(vec![
            asset("a", "same", 10, 1),
            asset("b", "same", 10, 2),
            asset("c", "unique", 20, 3),
        ]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].assets.len(), 2);
    }

    #[test]
    fn newest_and_oldest_pick_the_right_copy() {
        let group = Group {
            hash: "same".to_owned(),
            assets: vec![asset("old", "same", 10, 1), asset("new", "same", 10, 2)],
        };
        assert_eq!(group.kept_id(KeepPolicy::Newest), Some("new"));
        assert_eq!(group.kept_id(KeepPolicy::Oldest), Some("old"));
    }

    #[test]
    fn a_tie_uses_the_name_then_the_id() {
        let group = Group {
            hash: "same".to_owned(),
            assets: vec![asset("b", "same", 10, 1), asset("a", "same", 10, 1)],
        };
        assert_eq!(group.kept_id(KeepPolicy::Newest), Some("b"));
        assert_eq!(group.kept_id(KeepPolicy::Oldest), Some("a"));
    }

    #[test]
    fn reclaimable_excludes_the_kept_copy() {
        let group = Group {
            hash: "same".to_owned(),
            assets: vec![asset("a", "same", 10, 1), asset("b", "same", 30, 2)],
        };
        assert_eq!(group.reclaimable(), 30);
        assert_eq!(
            group
                .assets
                .iter()
                .filter(|asset| Some(asset.id.as_str()) != group.kept_id(KeepPolicy::Newest))
                .count(),
            1
        );
    }

    #[test]
    fn summary_counts_removable_copies() {
        let groups = vec![Group {
            hash: "same".to_owned(),
            assets: vec![asset("a", "same", 10, 1), asset("b", "same", 10, 2)],
        }];
        let summary = summarize(&groups);
        assert_eq!(summary.groups, 1);
        assert_eq!(summary.copies, 1);
        assert_eq!(summary.reclaimable, 10);
    }

    #[test]
    fn groups_are_ordered_by_reclaimable_space() {
        let groups = group_duplicates(vec![
            asset("a", "small", 10, 1),
            asset("b", "small", 10, 2),
            asset("c", "large", 100, 1),
            asset("d", "large", 100, 2),
        ]);
        assert_eq!(groups[0].hash, "large");
        assert_eq!(groups[1].hash, "small");
    }

    #[test]
    fn formats_bytes() {
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(1024), "1.0 KB");
        assert_eq!(bytes(1024 * 1024 * 3 / 2), "1.5 MB");
    }
}
