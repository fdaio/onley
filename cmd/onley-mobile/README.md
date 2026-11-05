# Onley Photos

Find duplicate photos in the iOS photo library, keep one copy per group, and
delete the rest after one confirmation. This is a preview: the desktop client
stays the validated target.

## Status

- **iOS**: the PhotoKit backend lists the library, exports each photo to a
  temporary file, hashes it, and deletes the copies the user does not keep.
- **Android**: not implemented. The standalone NSH package writes a manifest
  with only the INTERNET permission, and Android 11+ deletes media through a
  system consent flow that needs an Activity. A native host app is required;
  `src/source/android.rs` is a placeholder that keeps the shared app building.
- **Desktop preview**: the same screen runs on macOS with a folder backend.

## Build and run

Requirements: Rust 1.88 or newer, Xcode with an iOS simulator, and `nsh` 0.4.0.
The NSH crates come from the `v0.4.0` Git tag, so no local NSH checkout is
needed.

```sh
cd cmd/onley-mobile
./scripts/package-ios.sh --run
```

The script packages the simulator app, adds `NSPhotoLibraryUsageDescription`
(NSH 0.4.0 does not write it yet), signs the app ad hoc, installs it, and
launches it. Tap **Allow Full Access** once; the app then scans the library.

For a device, package with `nsh package --target ios --sign ... --profile ...`,
add the same usage description to the app's `Info.plist`, and sign again. The
script currently covers the simulator.

To run the shared screen on macOS, point it at a folder of images:

```sh
ONLEY_MOBILE_PHOTOS="$HOME/Pictures/OnleyPhotos" cargo run
```

## Test

```sh
cargo test
```

The headless tests drive the real screen with a fake library: scan, group,
keep the newest copy, delete the older one, and check that the library received
the right id. The scanner and folder backend have their own unit tests.

## How it works

- `src/model.rs`: groups assets by MD5, applies the keep policy, and formats
  the summary.
- `src/scan.rs`: exports one photo at a time, hashes it in 1 MiB chunks, and
  removes the temporary file.
- `src/source/ios.rs`: PhotoKit list, export and delete.
- `src/source/folder.rs`: folder backend for the desktop preview and tests.
- `src/state.rs`: NSH signals, the scan and delete tasks, and progress.
- `src/ui.rs`: the phone screen.

## Limitations

- iOS only. Android needs a native host app; see the status above.
- iCloud-only photos are skipped because export does not request network
  access. A later version can enable it and show download progress.
- No thumbnails yet. Groups show names, sizes, dimensions and dates.
- The scan runs in the foreground and hashes every photo in full. A later
  version can add a size and partial-hash prefilter.
- The simulator may not keep a `simctl privacy grant photos` across boots. Use
  the Simulator UI to tap **Allow Full Access** once. On a device the system
  prompt is the normal flow.
- NSH 0.4.0 has no photo-library permission entry in its generated
  `Info.plist`; the packaging script patches it after packaging. Upstream NSH
  should grow a capability for this.
