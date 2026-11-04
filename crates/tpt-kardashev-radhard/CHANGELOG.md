# Changelog: tpt-kardashev-radhard

## Unreleased

- `Nmr<T, N>` and `Tmr<T>` with majority vote, scrub and typestate wrappers.
- SECDED(72, 64) encode and decode, exhaustive single- and double-error tests.
- CRC-32C-checked cells and buffers.
- Checkpoint with rollback and a watchdog progress marker.
- Seeded fault injector.
- `volatile` feature: `VolatileRegister<T>` with volatile read and write; the only `unsafe` in the crate.
- `scrub_demo` example.
- Exhaustive and injector-driven tests. Kani proofs are not yet implemented.
