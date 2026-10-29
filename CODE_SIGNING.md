# Code signing policy

## Current status

Windows code signing is in progress. The Windows CLI and desktop executables in
the current releases are not code signed yet.

Release builds of the macOS desktop app are signed with an Apple Developer ID
and notarized. Linux packages are not code signed.

## Scope

This policy covers the Windows executables built from this repository:

- `onley.exe`, the command-line client
- `Onley.exe`, the desktop client

The macOS desktop app is covered by Apple Developer ID signing and
notarization. This policy covers the planned Windows signing.

## Signing provider

The project is seeking free code signing from
[SignPath.io](https://about.signpath.io), with a certificate from the
[SignPath Foundation](https://signpath.org). This is not active yet.

## Build and provenance

- Source: https://github.com/fdaio/onley
- License: MIT
- Builds: GitHub Actions, `.github/workflows/release.yml`, tag `v*`
- Only artifacts built from the tagged commit in that workflow are eligible for
  signing. Local builds are not signed.
- The desktop build sets the product name and version from
  `cmd/onley-app/nsgo.yaml`. Release assets use the tag as their version. The CLI
  binary does not embed version metadata.

## Roles

- Committers and reviewers: [repository contributors](https://github.com/fdaio/onley/graphs/contributors), including [@Laotree](https://github.com/Laotree).
- Approvers: [@Laotree](https://github.com/Laotree), repository owner.
- After signing is active, every signing request needs manual approval by an
  approver after the release candidate passes CI.

## Signing process after approval

1. A `v*` tag starts the release workflow.
2. CI builds the Windows CLI and desktop packages.
3. The Windows executables are submitted to SignPath.io.
4. An approver reviews the signing request.
5. SignPath signs the executables, and the workflow packages the signed files.
6. The release publishes the signed assets and checksums.

## Privacy

Onley runs locally. It has no telemetry and does not transfer information to
other systems unless the user runs `serve` or `replica`, or otherwise requests a
network operation. The client reads the local index at `~/.onley/local.db` and
the folders the user selects. See the [privacy policy](https://fdaio.github.io/onley/privacy.html).

## Contact

Open an issue at https://github.com/fdaio/onley/issues.
