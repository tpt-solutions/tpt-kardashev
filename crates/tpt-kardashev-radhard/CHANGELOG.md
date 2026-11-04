# Changelog: tpt-kardashev-radhard

## Unreleased

- `Nmr<T, N>` and `Tmr<T>` with majority vote, scrub and typestate wrappers.
- SECDED(72, 64) encode and decode, exhaustive single- and double-error tests.
- CRC-32C-checked cells and buffers.
- Checkpoint with rollback and a watchdog progress marker.
- Seeded fault injector.
- Exhaustive and injector-driven tests. Kani proofs and volatile access are not yet implemented.
