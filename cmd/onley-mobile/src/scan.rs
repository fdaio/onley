//! Hashes the photos a library exports.

use crate::model::Asset;
use crate::source::{AssetMeta, PhotoLibrary};
use md5::{Digest, Md5};
use std::fs::File;
use std::io::{self, Read};
use std::sync::atomic::{AtomicBool, Ordering};

const CHUNK: usize = 1 << 20;

/// What a scan reports while it runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanProgress {
    pub total: usize,
    pub done: usize,
    pub failed: usize,
    pub bytes: i64,
    pub current: String,
}

/// What a scan produced.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanOutcome {
    pub assets: Vec<Asset>,
    pub failed: usize,
    pub last_error: String,
}

/// Exports and hashes every photo. A photo that cannot be read is counted
/// and skipped; a cancelled scan keeps what it already hashed.
pub fn scan(
    library: &dyn PhotoLibrary,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(ScanProgress),
) -> Result<ScanOutcome, String> {
    let metas = library.list()?;
    let total = metas.len();
    let mut outcome = ScanOutcome::default();
    let mut bytes = 0;

    for (done, meta) in metas.into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        progress(ScanProgress {
            total,
            done,
            failed: outcome.failed,
            bytes,
            current: meta.name.clone(),
        });

        let path = match library.export(&meta.id) {
            Ok(path) => path,
            Err(err) => {
                outcome.failed += 1;
                outcome.last_error = err;
                continue;
            }
        };
        let result = hash_file(&path, cancel);
        let size = std::fs::metadata(&path)
            .map(|m| m.len() as i64)
            .unwrap_or(meta.size);
        library.cleanup(&path);

        match result {
            Ok(hash) => {
                bytes += size.max(0);
                outcome.assets.push(Asset {
                    id: meta.id,
                    name: meta.name,
                    size,
                    modified: meta.modified,
                    width: meta.width,
                    height: meta.height,
                    hash,
                });
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => break,
            Err(err) => {
                outcome.failed += 1;
                outcome.last_error = err.to_string();
            }
        }
    }

    progress(ScanProgress {
        total,
        done: outcome.assets.len() + outcome.failed,
        failed: outcome.failed,
        bytes,
        current: String::new(),
    });
    Ok(outcome)
}

/// Formats a digest as lowercase hex.
fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Hashes a file in chunks, stopping when `cancel` is set.
pub fn hash_file(path: &std::path::Path, cancel: &AtomicBool) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Md5::new();
    let mut buffer = vec![0u8; CHUNK];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        if cancel.load(Ordering::Relaxed) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "scan cancelled"));
        }
    }
    Ok(hex_digest(hasher.finalize().as_slice()))
}

/// A library backed by a folder, for the desktop preview and tests.
pub fn meta_from_path(id: String, path: &std::path::Path) -> AssetMeta {
    let metadata = std::fs::metadata(path).ok();
    AssetMeta {
        id,
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        size: metadata.as_ref().map(|m| m.len() as i64).unwrap_or(0),
        modified: metadata
            .and_then(|m| m.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0),
        width: 0,
        height: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::folder::FolderLibrary;
    use std::fs;

    #[test]
    fn hashes_duplicate_files_into_one_group() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), b"same bytes").unwrap();
        fs::write(dir.path().join("b.jpg"), b"same bytes").unwrap();
        fs::write(dir.path().join("c.jpg"), b"other bytes").unwrap();
        let library = FolderLibrary::new(dir.path().to_path_buf());
        let cancel = AtomicBool::new(false);
        let outcome = scan(&library, &cancel, &mut |_| {}).unwrap();
        assert_eq!(outcome.failed, 0);
        let groups = crate::model::group_duplicates(outcome.assets);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].assets.len(), 2);
    }

    #[test]
    fn cancellation_keeps_what_was_hashed() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), b"same bytes").unwrap();
        let library = FolderLibrary::new(dir.path().to_path_buf());
        let cancel = AtomicBool::new(true);
        let outcome = scan(&library, &cancel, &mut |_| {}).unwrap();
        assert!(outcome.assets.is_empty());
    }

    #[test]
    fn a_missing_folder_is_an_error() {
        let library = FolderLibrary::new("/nonexistent-onley-mobile-test".into());
        let cancel = AtomicBool::new(false);
        assert!(scan(&library, &cancel, &mut |_| {}).is_err());
    }
}
