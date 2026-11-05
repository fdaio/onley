//! Where the photos come from.

#[cfg(target_os = "android")]
pub mod android;
pub mod folder;
#[cfg(target_os = "ios")]
pub mod ios;

use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Whether the app may read the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Unknown,
    Denied,
    Limited,
    Authorized,
    Unsupported,
}

impl Access {
    pub fn can_read(self) -> bool {
        matches!(self, Self::Limited | Self::Authorized)
    }
}

/// One photo before it is hashed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetMeta {
    pub id: String,
    pub name: String,
    /// Bytes, or 0 when the platform does not tell before reading.
    pub size: i64,
    /// Seconds since the Unix epoch.
    pub modified: i64,
    pub width: u32,
    pub height: u32,
}

/// The photos of one device.
pub trait PhotoLibrary: Send + Sync {
    fn access(&self) -> Access;
    /// Asks the system for access and returns what it granted.
    fn request(&self) -> Access;
    fn list(&self) -> Result<Vec<AssetMeta>, String>;
    /// Writes one photo to a temporary file the scanner may read.
    fn export(&self, id: &str) -> Result<PathBuf, String>;
    /// Removes a file [`export`](Self::export) wrote.
    fn cleanup(&self, path: &Path);
    /// Deletes photos from the library.
    fn delete(&self, ids: &[String]) -> Result<usize, String>;
}

/// Opens the platform's library.
pub fn open() -> Arc<dyn PhotoLibrary> {
    #[cfg(target_os = "ios")]
    {
        Arc::new(ios::IosLibrary::new())
    }
    #[cfg(target_os = "android")]
    {
        Arc::new(android::AndroidLibrary::new())
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        Arc::new(folder::FolderLibrary::from_env())
    }
}
