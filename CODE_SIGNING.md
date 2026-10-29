# Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org).

## Scope

This policy covers the Windows executables built from this repository:

- `onley.exe`, the command-line client
- `Onley.exe`, the desktop client

The macOS desktop app is signed and notarized with an Apple Developer ID. This policy covers the Windows signing request.

## Build and provenance

- Source: https://github.com/fdaio/onley
- License: MIT
- Builds: GitHub Actions, `.github/workflows/release.yml`, tag `v*`
- Only artifacts built from the tagged commit in that workflow are signed. Local builds are not signed.
- The build sets the product name and version metadata; every artifact in a release uses the same version.

## Roles

- Committers and reviewers: [repository contributors](https://github.com/fdaio/onley/graphs/contributors), including [@Laotree](https://github.com/Laotree).
- Approvers: [@Laotree](https://github.com/Laotree), repository owner.
- Every signing request needs manual approval by an approver after the release candidate passes CI.

## Signing process

1. A `v*` tag starts the release workflow.
2. CI builds the Windows CLI and desktop packages.
3. The Windows executables are submitted to SignPath.io.
4. An approver reviews the signing request.
5. SignPath signs the executables, and the workflow packages the signed files.
6. The release publishes the signed assets and checksums.

## Privacy

Onley runs locally. It has no telemetry and does not transfer information to other systems unless the user runs `serve` or `replica`, or otherwise requests a network operation. The client reads the local index at `~/.onley/local.db` and the folders the user selects. See the README for details.

## Contact

Open an issue at https://github.com/fdaio/onley/issues.
