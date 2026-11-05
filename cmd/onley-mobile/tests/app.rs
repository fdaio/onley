//! Drives the phone app headless with a fake photo library.

use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use onley_mobile::source::{Access, AssetMeta, PhotoLibrary};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

static TEST_LOCK: Mutex<()> = Mutex::new(());

struct MockLibrary {
    _dir: tempfile::TempDir,
    assets: Vec<AssetMeta>,
    deleted: Arc<Mutex<Vec<String>>>,
}

impl MockLibrary {
    fn new(deleted: Arc<Mutex<Vec<String>>>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let files = [
            ("a.jpg", b"same bytes".as_slice(), 1),
            ("b.jpg", b"same bytes".as_slice(), 2),
            ("c.jpg", b"other bytes".as_slice(), 3),
        ];
        let mut assets = Vec::new();
        for (name, bytes, modified) in files {
            let path = dir.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            assets.push(AssetMeta {
                id: path.to_string_lossy().into_owned(),
                name: name.to_owned(),
                size: bytes.len() as i64,
                modified,
                width: 10,
                height: 10,
            });
        }
        Self {
            _dir: dir,
            assets,
            deleted,
        }
    }
}

impl PhotoLibrary for MockLibrary {
    fn access(&self) -> Access {
        Access::Authorized
    }

    fn request(&self) -> Access {
        Access::Authorized
    }

    fn list(&self) -> Result<Vec<AssetMeta>, String> {
        Ok(self.assets.clone())
    }

    fn export(&self, id: &str) -> Result<PathBuf, String> {
        Ok(PathBuf::from(id))
    }

    fn cleanup(&self, _path: &Path) {}

    fn delete(&self, ids: &[String]) -> Result<usize, String> {
        let mut deleted = self.deleted.lock().unwrap();
        for id in ids {
            std::fs::remove_file(id).map_err(|err| err.to_string())?;
            deleted.push(id.clone());
        }
        Ok(ids.len())
    }
}

#[test]
fn finds_and_deletes_the_older_copy() {
    let _guard = TEST_LOCK.lock().unwrap();
    let deleted = Arc::new(Mutex::new(Vec::new()));
    onley_mobile::set_library(Arc::new(MockLibrary::new(deleted.clone())));

    let mut app = Headless::start(EngineConfig::new(Root::of::<onley_mobile::App>())).unwrap();
    app.wait_for("id:scan").unwrap();
    app.press("id:scan").unwrap();
    app.wait_for("text:2 copies").unwrap();

    app.press("id:delete").unwrap();
    app.wait_for("id:confirm-dialog").unwrap();
    app.press("id:delete-confirm").unwrap();
    app.wait_for("text:No duplicate photos.").unwrap();

    let deleted = deleted.lock().unwrap();
    assert_eq!(deleted.len(), 1);
    assert!(deleted[0].ends_with("a.jpg"), "{deleted:?}");
}

#[test]
fn a_folder_without_duplicates_stays_empty() {
    let _guard = TEST_LOCK.lock().unwrap();
    let deleted = Arc::new(Mutex::new(Vec::new()));
    let mut library = MockLibrary::new(deleted.clone());
    library.assets.retain(|asset| asset.name == "c.jpg");
    onley_mobile::set_library(Arc::new(library));

    let mut app = Headless::start(EngineConfig::new(Root::of::<onley_mobile::App>())).unwrap();
    app.wait_for("id:scan").unwrap();
    app.press("id:scan").unwrap();
    app.wait_for("text:No duplicate photos.").unwrap();
    assert!(deleted.lock().unwrap().is_empty());
}
