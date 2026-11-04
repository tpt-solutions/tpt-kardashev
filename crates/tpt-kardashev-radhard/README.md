# tpt-kardashev-radhard

Radiation-tolerant primitives: triple modular redundancy, SECDED, checksummed cells and checkpoints.

**Status:** Phase 0 scaffold. No public API yet.

## Features

- `std` (default): links the standard library.
- `alloc`: enables heap-allocating APIs. Core is `no_std`.

## What it does not model

Pending: stated in full at v0.1 (todo §1.3). Documented exception: volatile access will be the only `unsafe` in this crate, behind a feature.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
