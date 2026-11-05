//! A library backed by a folder, for the desktop preview and tests.

use super::{Access, AssetMeta, PhotoLibrary};
use crate::scan::meta_from_path;
use std::path::{Path, PathBuf};

const IMAGE_EXTENSIONS: &[&str] = &[
    "avif", "bmp", "gif", "heic", "heif", "jpeg", "jpg", "png", "tif", "tiff", "webp",
];

/// Photos under one directory tree.
pub struct FolderLibrary {
    root: PathBuf,
}

impl FolderLibrary {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// `ONLEY_MOBILE_PHOTOS`, else `~/Pictures/OnleyPhotos`, else
    /// `~/Pictures`.
    pub fn from_env() -> Self {
        let root = std::env::var_os("ONLEY_MOBILE_PHOTOS")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| {
                    let pictures = PathBuf::from(home).join("Pictures");
                    let onley = pictures.join("OnleyPhotos");
                    if onley.is_dir() { onley } else { pictures }
                })
            })
            .unwrap_or_else(|| PathBuf::from("."));
        Self { root }
    }

    fn collect(&self, dir: &Path, out: &mut Vec<AssetMeta>) -> Result<(), String> {
        let entries = std::fs::read_dir(dir).map_err(|err| format!("{}: {err}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|err| err.to_string())?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') {
                continue;
            }
            let file_type = entry.file_type().map_err(|err| err.to_string())?;
            if file_type.is_dir() {
                self.collect(&path, out)?;
            } else if file_type.is_file() && is_image(&path) {
                out.push(meta_from_path(path.to_string_lossy().into_owned(), &path));
            }
        }
        Ok(())
    }
}

impl PhotoLibrary for FolderLibrary {
    fn access(&self) -> Access {
        if self.root.is_dir() {
            Access::Authorized
        } else {
            Access::Denied
        }
    }

    fn request(&self) -> Access {
        self.access()
    }

    fn list(&self) -> Result<Vec<AssetMeta>, String> {
        if !self.root.is_dir() {
            return Err(format!("{} is not a folder", self.root.display()));
        }
        let mut assets = Vec::new();
        self.collect(&self.root, &mut assets)?;
        assets.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(assets)
    }

    fn export(&self, id: &str) -> Result<PathBuf, String> {
        Ok(PathBuf::from(id))
    }

    fn cleanup(&self, _path: &Path) {}

    fn delete(&self, ids: &[String]) -> Result<usize, String> {
        let mut deleted = 0;
        for id in ids {
            match std::fs::remove_file(id) {
                Ok(()) => deleted += 1,
                Err(err) => return Err(format!("{id}: {err}")),
            }
        }
        Ok(deleted)
    }
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|extension| IMAGE_EXTENSIONS.contains(&extension.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn lists_images_and_skips_other_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), b"a").unwrap();
        fs::write(dir.path().join("b.txt"), b"b").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/c.png"), b"c").unwrap();
        let library = FolderLibrary::new(dir.path().to_path_buf());
        let assets = library.list().unwrap();
        let names: Vec<_> = assets.iter().map(|asset| asset.name.as_str()).collect();
        assert_eq!(names, ["a.jpg", "c.png"]);
    }

    #[test]
    fn deletes_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.jpg");
        fs::write(&path, b"a").unwrap();
        let library = FolderLibrary::new(dir.path().to_path_buf());
        assert_eq!(
            library
                .delete(&[path.to_string_lossy().into_owned()])
                .unwrap(),
            1
        );
        assert!(!path.exists());
    }
}
