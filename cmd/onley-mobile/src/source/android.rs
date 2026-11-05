//! Android photo access.
//!
//! The Android client is the native SAF app in `cmd/onley-android`. This NSH
//! screen is iOS-only; this placeholder keeps it building for Android and
//! tells the user where to go.

use super::{Access, AssetMeta, PhotoLibrary};
use std::path::{Path, PathBuf};

pub struct AndroidLibrary;

impl AndroidLibrary {
    pub fn new() -> Self {
        Self
    }
}

const NEEDS_HOST: &str = "Android uses the SAF app in cmd/onley-android; this NSH app is iOS-only";

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
