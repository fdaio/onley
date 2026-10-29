# Third-party notices

## Scope

This file covers the dependencies and assets distributed with onley. The full
module graph is in `go.mod` and `go.sum`; refer to each module's repository for
its complete license text.

## NSGo

The desktop client is built with NSGo, https://github.com/oneo-me/nsgo,
Copyright 2026 ONEO, licensed under the Apache License, Version 2.0
(https://www.apache.org/licenses/LICENSE-2.0).

NSGo is a Go port of Native Shell (NSH),
https://github.com/longbridge/nsh, Copyright 2026 Longbridge, licensed under
the Apache License, Version 2.0. NSGo is not affiliated with, or endorsed by,
Longbridge.

The NSGo source distribution carries the complete notices for its ported
components and bundled assets. Keep them with any distributed build of the
client.

## Go modules

The CLI and desktop client use these direct modules:

- `github.com/fsnotify/fsnotify` — BSD-3-Clause
- `github.com/mattn/go-isatty` — MIT
- `modernc.org/sqlite` — BSD-3-Clause

Indirect modules are pulled in by these direct dependencies; refer to their
source distributions for notices.
