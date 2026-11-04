# tpt-kardashev-astrodynamics

Time scales, reference frames, orbital elements, propagation, Lambert solving and impulsive manoeuvres.

**Status:** Phase 0 scaffold. No public API yet.

## Features

- `std` (default): links the standard library.
- `alloc`: enables heap-allocating APIs. Core is `no_std`.

## Optional features

- `relativity`: declared per spec §4.2; no code behind it yet.
- `dtn`: declared per spec §4.2; no code behind it yet.

## What it does not model

Force-model limits are stated in full at v0.1 (todo §2.2). No ephemeris data is bundled. Low-thrust approximations are post-v0.1. SGP4 stays behind the `sgp4` feature (spec §12.4).

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
