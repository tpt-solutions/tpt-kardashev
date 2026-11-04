# CLAUDE.md: tpt-kardashev

Ten independent Rust library crates in one Cargo workspace. Read `spec.txt` for the design and `todo.md` for scope and order.

## Commands

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --all-features
cargo build -p <crate> --no-default-features
cargo build --workspace --no-default-features --target thumbv7em-none-eabihf
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo deny check licenses
```

## Rules

- Rust 1.90.0 pinned; MSRV 1.90. Do not raise it without a todo entry.
- No `unwrap`, `expect`, `panic!` in library code. Return `#[non_exhaustive]` error enums.
- `#![forbid(unsafe_code)]` in every crate. `radhard` is the one documented exception (volatile access, behind a feature).
- No dependency on another crate in this repo, and no `path = "../..."` dependencies to sibling repos.
- Deterministic: seeded randomness only; simulators use explicit virtual time; no output that depends on `HashMap` iteration order.
- Runtime, optional and build dependencies must be MIT-selectable (`deny.toml`). Dev-dependencies are exempt, but no code may be copied from any source unless its licence is on the allow-list.
- Reference implementations are read, not copied. Record origin in `docs/provenance/<crate>.md`.
- Every `.rs` file starts with `// SPDX-License-Identifier: MIT OR Apache-2.0`.
- Floating-point comparisons follow `docs/tolerance-policy.md`.
- Each crate's README states what it does not model. Keep it current.
- Commit messages: no AI-attribution trailers.

## Status tracking

`todo.md` checkboxes are the org tracker. Tick an item only when its code, test or doc exists and the relevant command passes.
