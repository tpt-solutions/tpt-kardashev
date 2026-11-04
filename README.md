# tpt-kardashev

Foundation libraries for planetary-to-galactic scale: ten independent Rust crates in one Cargo workspace.

Status: Phase 0 scaffold. The crates are empty skeletons. Scope and order are in [todo.md](todo.md); the design is in [spec.txt](spec.txt).

## Crates

| Crate | Concern |
|---|---|
| [`tpt-kardashev-archive-codes`](crates/tpt-kardashev-archive-codes) | Archival and error-correcting codes |
| [`tpt-kardashev-relativity`](crates/tpt-kardashev-relativity) | Special and weak-field relativity, causality |
| [`tpt-kardashev-radhard`](crates/tpt-kardashev-radhard) | Radiation-tolerant computing primitives |
| [`tpt-kardashev-energy-ledger`](crates/tpt-kardashev-energy-ledger) | Double-entry energy accounting |
| [`tpt-kardashev-dtn`](crates/tpt-kardashev-dtn) | Delay-tolerant networking |
| [`tpt-kardashev-astrodynamics`](crates/tpt-kardashev-astrodynamics) | Time scales, frames, orbits |
| [`tpt-kardashev-nuclear-plasma`](crates/tpt-kardashev-nuclear-plasma) | Equilibrium and neutronics |
| [`tpt-kardashev-causal-consensus`](crates/tpt-kardashev-causal-consensus) | Causal ordering and consensus |
| [`tpt-kardashev-isru`](crates/tpt-kardashev-isru) | In-situ resource planning |
| [`tpt-kardashev-reflect`](crates/tpt-kardashev-reflect) | Verified self-modification |

## Dependency graph

Crates have no mandatory dependencies on each other. Optional edges are feature flags only:

- `astrodynamics` → `relativity` (feature `relativity`), `dtn` (feature `dtn`)
- `dtn` → `archive-codes` (feature `fec`)
- `causal-consensus` → `relativity` (feature `relativity`)
- `isru` → `energy-ledger` (feature `energy`)
- `reflect`, `nuclear-plasma`: independent

## Design principles

1. One concern per crate, with a small trait-based API.
2. No mandatory internal dependencies; every crate builds alone.
3. `no_std` first, with `alloc` and `std` as features.
4. Zero runtime dependencies by default.
5. No sibling path dependencies on other repos.
6. Deterministic: seeded randomness, explicit virtual time.
7. Errors, not panics; `unsafe` forbidden except where a crate documents why it needs it.
8. Reference code is ported, not linked; provenance is recorded in `docs/provenance/`.
9. Honest about assumptions; each crate's README says what it does not model.
10. Only MIT-selectable dependencies (spec §4.5).

## Non-goals

- No applications, daemons, UIs or CLIs beyond `examples/`.
- No hardware drivers, flight software buses or ground-station control.
- No claim of standards compliance or flight qualification.
- No speculative physics.
- No bundled datasets (nuclear data, ephemerides). Users supply them.

## Building

The toolchain is pinned in `rust-toolchain.toml` (Rust 1.90.0, MSRV 1.90).

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check licenses
```

## Licence

Dual-licensed under either of

- MIT licence ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you, as defined in the Apache-2.0 licence, shall be dual licensed as above, without any additional terms or conditions. See [CONTRIBUTING.md](CONTRIBUTING.md).

Copyright (c) 2026 TPT Solutions.
