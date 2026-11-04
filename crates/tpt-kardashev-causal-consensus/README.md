# tpt-kardashev-causal-consensus

Causal ordering and hierarchical consensus (domains and federation) with consistency bounds stated per level.

**Status:** Phase 0 scaffold. No public API yet.

## Features

- `std` (default): links the standard library.
- `alloc`: enables heap-allocating APIs. Core is `no_std`.

## Optional features

- `relativity`: declared per spec §4.2; no code behind it yet.

## What it does not model

Honest limits under partition (CAP). Byzantine tolerance is deferred behind a later feature.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
