# Third-party notices

## Scope

This file covers the dependencies and assets distributed with onley. The full
dependency graph is in `Cargo.toml` and `Cargo.lock`; refer to each crate's
repository for its complete license text.

## NSH

The desktop client is built with Native Shell (NSH),
https://github.com/longbridge/nsh, Copyright 2026 Longbridge, licensed under
the Apache License, Version 2.0
(https://www.apache.org/licenses/LICENSE-2.0).

The NSH distribution carries the complete notices for its components and
bundled assets. Keep them with any distributed build of the client.

## Rust crates

The CLI and desktop client use these direct crates:

- `rusqlite` — MIT
- `md-5` — Apache-2.0 / MIT
- `walkdir` — MIT / Unlicense
- `notify` — CC0-1.0
- `ureq` — MIT / Apache-2.0
- `axum` — MIT
- `tokio` — MIT
- `serde` and `serde_json` — MIT / Apache-2.0
- `same-file` — MIT / Unlicense
- `dirs` — MIT / Apache-2.0
- `url` — MIT / Apache-2.0
- `ctrlc` — MIT / Apache-2.0

Indirect crates are pulled in by these direct dependencies; refer to their
source distributions for notices.
