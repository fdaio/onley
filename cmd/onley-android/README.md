# Onley Photos for Android

Find duplicate photos in a folder the user selects with the Storage Access
Framework (SAF). SAF needs no storage permission, and deleting uses the same
tree grant, so there is no separate MediaStore consent flow.

## Build

Requirements: JDK 11 and the Android SDK with platform 33 and build tools.
Point `ANDROID_HOME` at the SDK.

```sh
export ANDROID_HOME="$HOME/Library/Android/sdk"
./gradlew :app:assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

The app needs Android 9 (API 28) or newer.

## Use

1. **Select folder**: pick a folder with photos, for example `DCIM` or a
   subfolder. Android 11 and newer do not let an app select the storage root;
   choose a folder inside it.
2. **Scan**: the app lists every image under the folder, hashes it in full,
   and groups identical files.
3. Tap a group to choose the copy to keep. The default is the newest.
4. **Delete N copies**: confirm, and the app deletes every copy except the
   kept one. The kept copy stays.

The folder grant is persisted, so the next launch can scan it again.

## Test

```sh
./gradlew :app:testDebugUnitTest
```

The tests cover grouping, the keep policy, reclaimable space and formatting.

## Limits

- Scans only the folder the user selects, not the whole library.
- Images only; videos are skipped.
- No thumbnails yet. A group shows the number of copies, the reclaimable
  space, the kept file and its date.
- Some document providers do not support deletion. The app reports how many
  copies it deleted and how many failed.
- This is a native Android app, not the NSH iOS screen. NSH 0.4.0's standalone
  Android package writes only the INTERNET permission and has no Activity
  result hook for SAF, so the Android client is Java. Upstream requests:
  [longbridge/nsh#162](https://github.com/longbridge/nsh/issues/162) (folder
  picker) and
  [longbridge/nsh#915](https://github.com/longbridge/nsh/issues/915) (manifest
  permissions). If NSH adds a folder picker, this client can move back to the
  shared NSH UI.
