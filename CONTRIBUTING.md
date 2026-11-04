# Contributing to tpt-kardashev

## Licence of contributions

The repository is dual-licensed under MIT OR Apache-2.0. Unless you state otherwise, any contribution you submit for inclusion is licensed under the same terms, without any additional conditions (the Apache-2.0 contribution clause, section 5).

## Dependency licences

Every runtime, optional and build dependency must be MIT-selectable (spec §4.5). `cargo deny check licenses` enforces the allow-list in `deny.toml`. Dev-dependencies are exempt from that check, but no code from any dependency may be pasted into library source unless its licence is on the allow-list.

## No copying of third-party code

Reference implementations of standards are read as documents, not copied. Do not paste code from GPL-family or Apache-only implementations when porting algorithms. Ported or adapted code must be recorded in `docs/provenance/<crate>.md` with its origin and licence.

## Admission rule for new crates

A new crate is accepted only if it:

1. has one concern, stated in one sentence;
2. exposes a small trait-based public API;
3. contains no application code (no daemons, UIs, CLIs beyond `examples/`);
4. builds alone with `cargo build -p <crate> --no-default-features`.

## Checks before opening a change

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check licenses
```

Library code must not use `unwrap`, `expect` or `panic!`. Return errors instead.

## Commit messages

Commit messages describe the change in the imperative mood. Do not add AI-attribution trailers.
