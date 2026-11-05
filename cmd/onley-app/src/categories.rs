//! Classifies duplicate groups by the first file's extension.

use onley_core::db::DuplicateGroup;
use onley_core::i18n as keys;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Category {
    #[default]
    All,
    Images,
    Videos,
    Documents,
    Archives,
    System,
    Search,
}

pub const TABS: [Category; 7] = [
    Category::All,
    Category::Images,
    Category::Videos,
    Category::Documents,
    Category::Archives,
    Category::System,
    Category::Search,
];

const IMAGE: &[&str] = &[
    "avif", "bmp", "cr2", "dng", "gif", "heic", "jpeg", "jpg", "nef", "png", "raw", "svg", "tif",
    "tiff", "webp",
];
const VIDEO: &[&str] = &[
    "avi", "flv", "m4v", "mkv", "mov", "mp4", "mpeg", "mpg", "webm", "wmv",
];
const DOCUMENT: &[&str] = &[
    "csv", "doc", "docx", "key", "md", "numbers", "pages", "pdf", "ppt", "pptx", "rtf", "txt",
    "xls", "xlsx",
];
const ARCHIVE: &[&str] = &[
    "7z", "bz2", "dmg", "gz", "iso", "rar", "tar", "tgz", "xz", "zip",
];

/// Returns the category of a file name.
pub fn file_category(name: &str) -> Category {
    let extension = Path::new(name)
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let extension = extension.as_str();
    if IMAGE.contains(&extension) {
        Category::Images
    } else if VIDEO.contains(&extension) {
        Category::Videos
    } else if DOCUMENT.contains(&extension) {
        Category::Documents
    } else if ARCHIVE.contains(&extension) {
        Category::Archives
    } else {
        Category::System
    }
}

impl Category {
    /// The value used in a test id.
    pub fn key_name(self) -> &'static str {
        match self {
            Category::Images => "images",
            Category::Videos => "videos",
            Category::Documents => "documents",
            Category::Archives => "archives",
            Category::System => "system",
            Category::Search => "search",
            Category::All => "all",
        }
    }

    /// The message key of its tab label.
    pub fn label_key(self) -> &'static str {
        match self {
            Category::Images => keys::IMAGES,
            Category::Videos => keys::VIDEOS,
            Category::Documents => keys::DOCUMENTS,
            Category::Archives => keys::ARCHIVES,
            Category::System => keys::SYSTEM,
            Category::Search => keys::SEARCH,
            Category::All => keys::ALL_COPIES,
        }
    }
}

/// Reports whether a group has a file under one of the roots.
pub fn group_in_roots(group: &DuplicateGroup, roots: &[String]) -> bool {
    group.files.iter().any(|file| {
        roots.iter().any(|root| {
            let root = Path::new(root);
            file.path == root || file.path.starts_with(root)
        })
    })
}

/// Reports whether a name or path contains the lower-case query.
pub fn group_matches(group: &DuplicateGroup, query: &str) -> bool {
    group.files.iter().any(|file| {
        file.name.to_lowercase().contains(query)
            || file.path.to_string_lossy().to_lowercase().contains(query)
    })
}

/// Groups digits with commas for the large header numbers.
pub fn format_count(n: usize) -> String {
    let digits = n.to_string();
    if digits.len() <= 3 {
        return digits;
    }
    let mut out = String::new();
    let first = digits.len() % 3;
    if first > 0 {
        out.push_str(&digits[..first]);
    }
    for i in (first..digits.len()).step_by(3) {
        if !out.is_empty() {
            out.push(',');
        }
        out.push_str(&digits[i..i + 3]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_extensions() {
        assert_eq!(file_category("a.JPG"), Category::Images);
        assert_eq!(file_category("a.mp4"), Category::Videos);
        assert_eq!(file_category("a.pdf"), Category::Documents);
        assert_eq!(file_category("a.zip"), Category::Archives);
        assert_eq!(file_category("a.bin"), Category::System);
        assert_eq!(file_category("noext"), Category::System);
    }

    #[test]
    fn counts_are_grouped() {
        assert_eq!(format_count(7), "7");
        assert_eq!(format_count(1234), "1,234");
        assert_eq!(format_count(1234567), "1,234,567");
    }
}
