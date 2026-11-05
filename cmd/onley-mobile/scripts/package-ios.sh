#!/bin/sh
# Packages the iOS simulator app, adds the photo-library usage description
# NSH does not write yet, signs it ad hoc, and optionally installs it.
#
#   ./scripts/package-ios.sh
#   ./scripts/package-ios.sh --run
set -eu

cd "$(dirname "$0")/.."
RUN=0
if [ "${1:-}" = "--run" ]; then
    RUN=1
fi

nsh package --target ios-simulator -o dist

APP="dist/Onley Photos.app"
PLIST="$APP/Info.plist"
DESCRIPTION="Onley finds duplicate photos in your library and removes the copies you choose."

if /usr/libexec/PlistBuddy -c "Set :NSPhotoLibraryUsageDescription $DESCRIPTION" "$PLIST" 2>/dev/null; then
    :
else
    /usr/libexec/PlistBuddy -c "Add :NSPhotoLibraryUsageDescription string $DESCRIPTION" "$PLIST"
fi
codesign --force --sign - "$APP"

if [ "$RUN" = 1 ]; then
    xcrun simctl install booted "$APP"
    xcrun simctl launch --terminate-running-process booted io.github.laotree.onley.photos
fi

echo "packaged $APP"
