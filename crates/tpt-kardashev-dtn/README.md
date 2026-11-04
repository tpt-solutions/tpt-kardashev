# tpt-kardashev-dtn

Delay-tolerant networking: BPv7 bundles, bundle stores, contact-graph routing and an LTP convergence layer.

**Status:** Phase 0 scaffold. No public API yet.

## Features

- `std` (default): links the standard library.
- `alloc`: enables heap-allocating APIs. Core is `no_std`.

## Optional features

- `fec`: declared per spec §4.2; no code behind it yet.

## What it does not model

No compliance claim against RFC 9171 or RFC 5326. No BPSec. No TCPCL in v0.1. No bundle-level reliability; LTP-level only (spec §12.2).

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
