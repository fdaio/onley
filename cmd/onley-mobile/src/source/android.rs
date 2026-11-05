//! Android photo access.
//!
//! The standalone Android package NSH writes declares only the INTERNET
//! permission, and Android 11+ deletes media through a system consent flow
//! that needs an Activity. Both need a native host app; this placeholder
//! keeps the shared app building for Android and tells the user why.

use super::{Access, AssetMeta, PhotoLibrary};
use std::path::{Path, PathBuf};

pub struct AndroidLibrary;

impl AndroidLibrary {
    pub fn new() -> Self {
        Self
    }
}

const NEEDS_HOST: &str =
    "Android photo access needs a native host app; see cmd/onley-mobile/README.md";

impl PhotoLibrary for AndroidLibrary {
    fn access(&self) -> Access {
        Access::Unsupported
    }

    fn request(&self) -> Access {
        Access::Unsupported
    }

    fn list(&self) -> Result<Vec<AssetMeta>, String> {
        Err(NEEDS_HOST.to_owned())
    }

    fn export(&self, _id: &str) -> Result<PathBuf, String> {
        Err(NEEDS_HOST.to_owned())
    }

    fn cleanup(&self, _path: &Path) {}

    fn delete(&self, _ids: &[String]) -> Result<usize, String> {
        Err(NEEDS_HOST.to_owned())
    }
}
