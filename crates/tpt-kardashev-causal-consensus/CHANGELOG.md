# Changelog: tpt-kardashev-causal-consensus

## Unreleased

- `clock::HybridClock` and `clock::Timestamp`: hybrid logical clock. Timestamps are strictly increasing per node, strictly after any remote timestamp received, and a remote timestamp more than the configured drift ahead of local physical time is rejected without changing the clock. Tests in `tests/clocks.rs` (10 000 steps with physical time jumping and going backwards, causality along a three-node message chain, rejection leaves the clock unchanged, the wall part stays within the largest reading plus the bound across three nodes) and a unit test of counter overflow in `src/clock.rs`.
- `vector::VectorClock<N>` and `vector::CausalRelation`: fixed-size vector clock with tick, merge (entry-wise maximum) and the four relations. Tests in `tests/clocks.rs`: each relation, merge commutativity and idempotence, and an out-of-range index.
- `error::ClockError` (`#[non_exhaustive]`): `DriftExceeded`, `CounterOverflow`, `NodeOutOfRange`.
