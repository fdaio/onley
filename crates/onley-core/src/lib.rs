//! onley's index, scanning, cleaning, watching and replication.

pub mod appdir;
pub mod appmodel;
pub mod cleaner;
pub mod config;
pub mod db;
pub mod format;
pub mod i18n;
pub mod replica;
pub mod scanner;
pub mod selfpath;
pub mod ui;
pub mod watcher;

/// Reports a file-system permission failure.
pub fn is_permission_error(err: &std::io::Error) -> bool {
    err.kind() == std::io::ErrorKind::PermissionDenied
}
