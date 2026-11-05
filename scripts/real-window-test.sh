#!/bin/sh
# Runs the desktop client's real-window UI test against a temporary index.
# Needs macOS, Xcode and the nsh command.
set -eu

root=$(mktemp -d "${TMPDIR:-/tmp}/onley-ui.XXXXXX")
trap 'rm -rf "$root"' EXIT

mkdir -p "$root/a-keep" "$root/z-gone"
printf 'onley real-window test\n' > "$root/a-keep/one.txt"
printf 'onley real-window test\n' > "$root/z-gone/two.txt"

# The test opens this index, keeps the first path and deletes the other copy.
cargo run -q -p onley -- -db "$root/local.db" scan "$root" >/dev/null

ONLEY_DB="$root/local.db" ONLEY_HOME="$root" nsh test --target macos cmd/onley-app
