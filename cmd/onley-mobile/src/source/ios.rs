//! PhotoKit, the iOS photo library.

use super::{Access, AssetMeta, PhotoLibrary};
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSArray, NSError, NSString, NSURL};
use objc2_photos::{
    PHAccessLevel, PHAsset, PHAssetChangeRequest, PHAssetMediaType, PHAssetResource,
    PHAssetResourceManager, PHAssetResourceRequestOptions, PHAssetResourceType,
    PHAuthorizationStatus, PHPhotoLibrary,
};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// The system photo library.
pub struct IosLibrary {
    temp: PathBuf,
}

impl IosLibrary {
    pub fn new() -> Self {
        let temp = std::env::temp_dir().join("onley-photos");
        let _ = std::fs::create_dir_all(&temp);
        Self { temp }
    }
}

impl PhotoLibrary for IosLibrary {
    fn access(&self) -> Access {
        access_of(unsafe {
            PHPhotoLibrary::authorizationStatusForAccessLevel(PHAccessLevel::ReadWrite)
        })
    }

    fn request(&self) -> Access {
        let (tx, rx) = mpsc::channel();
        let block = RcBlock::new(move |status: PHAuthorizationStatus| {
            let _ = tx.send(status);
        });
        unsafe {
            PHPhotoLibrary::requestAuthorizationForAccessLevel_handler(
                PHAccessLevel::ReadWrite,
                &block,
            );
        }
        rx.recv_timeout(Duration::from_secs(300))
            .map(access_of)
            .unwrap_or_else(|_| self.access())
    }

    fn list(&self) -> Result<Vec<AssetMeta>, String> {
        let fetch =
            unsafe { PHAsset::fetchAssetsWithMediaType_options(PHAssetMediaType::Image, None) };
        let count = unsafe { fetch.count() } as usize;
        let mut assets = Vec::with_capacity(count);
        for index in 0..count {
            let asset = unsafe { fetch.objectAtIndex(index) };
            let id = unsafe { asset.localIdentifier() }.to_string();
            let resources = unsafe { PHAssetResource::assetResourcesForAsset(&asset) };
            let name = pick_resource(&resources)
                .map(|resource| unsafe { resource.originalFilename() }.to_string())
                .unwrap_or_else(|| id.clone());
            let modified = unsafe {
                asset
                    .modificationDate()
                    .or_else(|| asset.creationDate())
                    .map(|date| date.timeIntervalSince1970() as i64)
                    .unwrap_or(0)
            };
            assets.push(AssetMeta {
                id,
                name,
                size: 0,
                modified,
                width: unsafe { asset.pixelWidth() } as u32,
                height: unsafe { asset.pixelHeight() } as u32,
            });
        }
        Ok(assets)
    }

    fn export(&self, id: &str) -> Result<PathBuf, String> {
        let asset = fetch_asset(id)?;
        let resources = unsafe { PHAssetResource::assetResourcesForAsset(&asset) };
        let resource =
            pick_resource(&resources).ok_or_else(|| format!("{id} has no photo resource"))?;
        let name = unsafe { resource.originalFilename() }.to_string();
        let path = self
            .temp
            .join(format!("{}.{}", sanitize(id), extension(&name)));
        let _ = std::fs::remove_file(&path);

        let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
        let (tx, rx) = mpsc::channel();
        let block = RcBlock::new(move |error: *mut NSError| {
            let _ = tx.send(error.is_null());
        });
        let options = unsafe { PHAssetResourceRequestOptions::new() };
        unsafe {
            options.setNetworkAccessAllowed(false);
            PHAssetResourceManager::defaultManager()
                .writeDataForAssetResource_toFile_options_completionHandler(
                    &resource,
                    &url,
                    Some(&options),
                    &block,
                );
        }
        match rx.recv_timeout(Duration::from_secs(120)) {
            Ok(true) => Ok(path),
            Ok(false) => Err(format!("cannot read {name}")),
            Err(_) => Err(format!("timed out reading {name}")),
        }
    }

    fn cleanup(&self, path: &Path) {
        let _ = std::fs::remove_file(path);
    }

    fn delete(&self, ids: &[String]) -> Result<usize, String> {
        if ids.is_empty() {
            return Ok(0);
        }
        let identifiers: Vec<Retained<NSString>> =
            ids.iter().map(|id| NSString::from_str(id)).collect();
        let identifiers = NSArray::from_retained_slice(&identifiers);
        let fetch = unsafe { PHAsset::fetchAssetsWithLocalIdentifiers_options(&identifiers, None) };
        let count = unsafe { fetch.count() } as usize;
        if count == 0 {
            return Ok(0);
        }
        let assets: Vec<Retained<PHAsset>> = (0..count)
            .map(|index| unsafe { fetch.objectAtIndex(index) })
            .collect();
        let assets = NSArray::from_retained_slice(&assets);
        let change = RcBlock::new(move || unsafe {
            PHAssetChangeRequest::deleteAssets(ProtocolObject::from_ref(&*assets));
        });
        let library = unsafe { PHPhotoLibrary::sharedPhotoLibrary() };
        let result =
            unsafe { library.performChangesAndWait_error(RcBlock::as_ptr(&change) as *mut _) };
        match result {
            Ok(()) => Ok(count),
            Err(err) => Err(err.localizedDescription().to_string()),
        }
    }
}

fn access_of(status: PHAuthorizationStatus) -> Access {
    if status == PHAuthorizationStatus::Authorized {
        Access::Authorized
    } else if status == PHAuthorizationStatus::Limited {
        Access::Limited
    } else if status == PHAuthorizationStatus::Denied || status == PHAuthorizationStatus::Restricted
    {
        Access::Denied
    } else {
        Access::Unknown
    }
}

fn fetch_asset(id: &str) -> Result<Retained<PHAsset>, String> {
    let identifiers = NSArray::from_retained_slice(&[NSString::from_str(id)]);
    let fetch = unsafe { PHAsset::fetchAssetsWithLocalIdentifiers_options(&identifiers, None) };
    if unsafe { fetch.count() } == 0 {
        return Err(format!("{id} is gone"));
    }
    Ok(unsafe { fetch.objectAtIndex(0) })
}

fn pick_resource(resources: &NSArray<PHAssetResource>) -> Option<Retained<PHAssetResource>> {
    let count = resources.count() as usize;
    let mut best: Option<(u8, Retained<PHAssetResource>)> = None;
    for index in 0..count {
        let resource = resources.objectAtIndex(index);
        let rank = match unsafe { resource.r#type() } {
            PHAssetResourceType::Photo => 0,
            PHAssetResourceType::FullSizePhoto => 1,
            PHAssetResourceType::AlternatePhoto => 2,
            PHAssetResourceType::PhotoProxy => 3,
            _ => continue,
        };
        if best.as_ref().is_none_or(|(current, _)| rank < *current) {
            best = Some((rank, resource));
        }
    }
    best.map(|(_, resource)| resource)
}

fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .filter(|extension| !extension.is_empty())
        .unwrap_or_else(|| "bin".to_owned())
}
