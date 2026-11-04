# tpt-kardashev-nuclear-plasma

Fixed-boundary Grad-Shafranov equilibrium and fixed-source Monte Carlo neutronics with a cross-section reader.

**Status:** Phase 0 scaffold. No public API yet.

## Features

- `std` (default): links the standard library.
- `alloc`: enables heap-allocating APIs. Core is `no_std`.

## What it does not model

No bundled nuclear data: users supply cross-section files (spec §2). Eigenvalue, free-boundary and MHD are post-v0.1.

## Licence

MIT OR Apache-2.0. See the workspace `LICENSE-MIT` and `LICENSE-APACHE`.
